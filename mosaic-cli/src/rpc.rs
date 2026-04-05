use anyhow::{Context, Result, anyhow, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use solana_account::Account;
use solana_account_decoder_client_types::UiAccountEncoding;
use solana_keypair::{Keypair, read_keypair_file};
use solana_pubkey::Pubkey;
use solana_rpc_client::rpc_client::RpcClient;
use solana_rpc_client_types::{
    config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType},
};
use solana_signer::Signer;
use solana_transaction::Transaction;
use std::{
    fs::File,
    path::{Path, PathBuf},
    str::FromStr,
};

use crate::{
    cli::{ClientConfig, DataEncodingArg, SessionSelectorArgs},
    instructions::{derive_root_pda, derive_signing_session_pda},
    models::{
        DataEncoding, InstructionAccountView, LoadedRoot, LoadedSession, LoadedSessionSpec,
        ResolvedSession, SessionSpecFile,
    },
    util::{address_to_pubkey, decode_data, decode_hex, expand_path},
};
use mosaic::state::{
    PackUnpack,
    root::Root,
    signing_session::{InstructionAccount as SessionInstructionAccount, SigningSession},
};

pub(crate) fn fetch_root(rpc: &RpcClient, config: &ClientConfig) -> Result<LoadedRoot> {
    let (root_pda, _) = derive_root_pda(&config.program_id);
    fetch_root_by_pubkey(rpc, config, root_pda)
}

pub(crate) fn list_sessions(
    rpc: &RpcClient,
    config: &ClientConfig,
    root_pda: &Pubkey,
) -> Result<Vec<(Pubkey, SigningSession)>> {
    let ui_accounts = rpc
        .get_program_ui_accounts_with_config(
            &config.program_id,
            RpcProgramAccountsConfig {
                filters: Some(vec![RpcFilterType::Memcmp(Memcmp::new(
                    2,
                    MemcmpEncodedBytes::Base64(BASE64_STANDARD.encode(root_pda.to_bytes())),
                ))]),
                account_config: RpcAccountInfoConfig {
                    encoding: Some(UiAccountEncoding::Base64),
                    commitment: Some(config.commitment.clone()),
                    ..Default::default()
                },
                with_context: Some(false),
                sort_results: Some(true),
            },
        )
        .context("failed to list program accounts for Mosaic")?;

    let mut sessions = Vec::new();
    for (pubkey, ui_account) in ui_accounts {
        let Some(account) = ui_account.decode::<Account>() else {
            continue;
        };
        let Ok(session) = SigningSession::unpack(&account.data) else {
            continue;
        };
        if address_to_pubkey(&session.root_pda) == *root_pda {
            sessions.push((pubkey, session));
        }
    }
    sessions.sort_by_key(|(_, session)| session.session_id);
    Ok(sessions)
}

pub(crate) fn resolve_session(
    rpc: &RpcClient,
    config: &ClientConfig,
    selector: &SessionSelectorArgs,
) -> Result<ResolvedSession> {
    if let Some(session_pubkey) = selector.session {
        let session = fetch_session_by_pubkey(rpc, config, session_pubkey)?;
        let root_pubkey = address_to_pubkey(&session.data.root_pda);
        let root = fetch_root_by_pubkey(rpc, config, root_pubkey)?;
        verify_session_matches_root(config, &root, &session)?;
        return Ok(ResolvedSession { root, session });
    }

    let root = fetch_root(rpc, config)?;
    let session_id = match selector.session_id {
        Some(session_id) => session_id,
        None => {
            if root.data.last_id == 0 {
                bail!("root {} has no signing sessions yet", root.pubkey);
            }
            root.data.last_id
        }
    };

    let (session_pubkey, _) =
        derive_signing_session_pda(&config.program_id, &root.pubkey, session_id);
    let session = fetch_session_by_pubkey(rpc, config, session_pubkey)?;
    verify_session_matches_root(config, &root, &session)?;

    Ok(ResolvedSession { root, session })
}

pub(crate) fn send_instruction(
    rpc: &RpcClient,
    config: &ClientConfig,
    payer: &Keypair,
    instruction: solana_instruction::Instruction,
    additional_signers: &[Keypair],
) -> Result<String> {
    let blockhash = rpc
        .get_latest_blockhash()
        .context("failed to fetch latest blockhash")?;

    let mut signers: Vec<&dyn Signer> = Vec::with_capacity(1 + additional_signers.len());
    signers.push(payer);
    for signer in additional_signers {
        signers.push(signer);
    }

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &signers,
        blockhash,
    );
    let signature = rpc
        .send_and_confirm_transaction_with_spinner_and_commitment(
            &transaction,
            config.commitment.clone(),
        )
        .map_err(humanize_mosaic_transaction_error)?;

    Ok(signature.to_string())
}

fn humanize_mosaic_transaction_error(
    error: solana_rpc_client::api::client_error::Error,
) -> anyhow::Error {
    let error_text = error.to_string();

    if let Some(code) = extract_custom_program_error_code(&error_text) {
        if let Some(message) = mosaic_error_message(code) {
            return anyhow!(
                "failed to send Mosaic transaction: {} (Mosaic program error {:#x} / {})",
                message,
                code,
                code
            );
        }
    }

    anyhow!("failed to send Mosaic transaction: {error_text}")
}

fn extract_custom_program_error_code(error_text: &str) -> Option<u32> {
    let marker = "custom program error: 0x";
    let start = error_text.find(marker)? + marker.len();
    let hex = error_text[start..]
        .chars()
        .take_while(|c| c.is_ascii_hexdigit())
        .collect::<String>();

    (!hex.is_empty())
        .then(|| u32::from_str_radix(&hex, 16).ok())
        .flatten()
}

fn mosaic_error_message(code: u32) -> Option<&'static str> {
    match code {
        6000 => Some("payer and signer must equal"),
        6001 => Some("payer account must be writable"),
        6002 => Some("root account must be writable"),
        6003 => Some("root account must be initialized"),
        6004 => Some("root account must not be initialized"),
        6005 => Some("root account owner must equal the Mosaic program id"),
        6006 => Some("signing session account must be writable"),
        6007 => Some("signing session account must be initialized"),
        6008 => Some("signing session account must not be initialized"),
        6009 => Some("signing session account owner must equal the Mosaic program id"),
        6010 => Some("signing session phase is incorrect for this action"),
        6011 => Some("provided destination program does not match the root destination program"),
        6012 => Some("signing session is already at the final stage"),
        6013 => Some("this operator already approved the signing session"),
        6014 => Some("the signer is not one of the configured operators"),
        6015 => Some("signing session id must equal root last id"),
        6016 => Some("approvals did not reach the configured threshold"),
        6017 => Some("root destination program must match the provided CPI program id"),
        6018 => Some("threshold cannot be higher than the number of operators"),
        6019 => Some("there must be at least one operator"),
        6020 => Some("threshold cannot be zero"),
        6021 => Some("maximum operators limit reached"),
        _ => None,
    }
}

pub(crate) fn load_default_signer(config: &ClientConfig) -> Result<Keypair> {
    load_keypair(&config.keypair_path)
}

pub(crate) fn load_additional_signers(paths: &[PathBuf]) -> Result<Vec<Keypair>> {
    paths
        .iter()
        .map(|path| load_keypair(&expand_path(path)?))
        .collect()
}

pub(crate) fn load_session_spec(path: &Path) -> Result<LoadedSessionSpec> {
    let path = expand_path(path)?;
    let file = File::open(&path)
        .with_context(|| format!("failed to open spec file {}", path.display()))?;
    let spec: SessionSpecFile = serde_json::from_reader(file)
        .with_context(|| format!("failed to parse session spec {}", path.display()))?;

    let instruction_data = decode_data(spec.data_encoding, &spec.data)?;
    let mut instruction_accounts = Vec::with_capacity(spec.accounts.len());
    for account in spec.accounts {
        let pubkey = Pubkey::from_str(&account.pubkey)
            .with_context(|| format!("invalid pubkey {} in {}", account.pubkey, path.display()))?;
        let stored = SessionInstructionAccount {
            pubkey: *pubkey.as_array(),
            signer: account.signer,
            writable: account.writable,
        };
        let serialized = stored
            .serialize()
            .map_err(|error| {
                anyhow!("failed to serialize instruction account {pubkey}: {error:?}")
            })?
            .0;

        instruction_accounts.push(serialized);
    }

    Ok(LoadedSessionSpec {
        instruction_data,
        instruction_accounts,
    })
}

pub(crate) fn load_inline_session_spec(
    data_encoding: DataEncodingArg,
    data: &str,
    accounts: &[String],
) -> Result<LoadedSessionSpec> {
    let instruction_data = decode_data(data_encoding.into(), data)?;
    let mut instruction_accounts = Vec::with_capacity(accounts.len());

    for raw_account in accounts {
        let bytes = decode_hex(raw_account)
            .with_context(|| format!("invalid inline account `{raw_account}`"))?;
        SessionInstructionAccount::deserialize(&bytes).map_err(|error| {
            anyhow!("failed to decode inline instruction account `{raw_account}`: {error:?}")
        })?;
        instruction_accounts.push(bytes);
    }

    Ok(LoadedSessionSpec {
        instruction_data,
        instruction_accounts,
    })
}

pub(crate) fn decode_instruction_accounts(
    session: &SigningSession,
) -> Result<Vec<InstructionAccountView>> {
    session
        .instruction_accounts
        .iter()
        .map(|raw| {
            let account = SessionInstructionAccount::deserialize(raw).map_err(|error| {
                anyhow!("failed to decode stored instruction account: {error:?}")
            })?;
            Ok(InstructionAccountView {
                pubkey: Pubkey::new_from_array(account.pubkey).to_string(),
                writable: account.writable,
                signer: account.signer,
            })
        })
        .collect()
}

fn fetch_root_by_pubkey(
    rpc: &RpcClient,
    config: &ClientConfig,
    pubkey: Pubkey,
) -> Result<LoadedRoot> {
    let (expected_root, expected_bump) = derive_root_pda(&config.program_id);
    if pubkey != expected_root {
        bail!(
            "root {} does not match the derived root PDA {} for program {}",
            pubkey,
            expected_root,
            config.program_id
        );
    }

    let account = rpc
        .get_account(&pubkey)
        .with_context(|| format!("failed to fetch root account {pubkey}"))?;
    if account.owner != config.program_id {
        bail!(
            "root account {} is owned by {}, expected {}",
            pubkey,
            account.owner,
            config.program_id
        );
    }

    let data = Root::unpack(&account.data)
        .with_context(|| format!("failed to decode root account {pubkey}"))?;
    if data.bump != expected_bump {
        bail!(
            "root {} bump {} does not match derived bump {}",
            pubkey,
            data.bump,
            expected_bump
        );
    }

    Ok(LoadedRoot { pubkey, data })
}

fn fetch_session_by_pubkey(
    rpc: &RpcClient,
    config: &ClientConfig,
    pubkey: Pubkey,
) -> Result<LoadedSession> {
    let account = rpc
        .get_account(&pubkey)
        .with_context(|| format!("failed to fetch signing session {pubkey}"))?;
    if account.owner != config.program_id {
        bail!(
            "signing session {} is owned by {}, expected {}",
            pubkey,
            account.owner,
            config.program_id
        );
    }

    let data = SigningSession::unpack(&account.data)
        .with_context(|| format!("failed to decode signing session {pubkey}"))?;

    Ok(LoadedSession { pubkey, data })
}

fn verify_session_matches_root(
    config: &ClientConfig,
    root: &LoadedRoot,
    session: &LoadedSession,
) -> Result<()> {
    let stored_root = address_to_pubkey(&session.data.root_pda);
    if stored_root != root.pubkey {
        bail!(
            "session {} belongs to root {}, expected {}",
            session.pubkey,
            stored_root,
            root.pubkey
        );
    }

    let (expected_session_pda, expected_bump) =
        derive_signing_session_pda(&config.program_id, &root.pubkey, session.data.session_id);
    if expected_session_pda != session.pubkey {
        bail!(
            "session {} does not match derived PDA {} for id {}",
            session.pubkey,
            expected_session_pda,
            session.data.session_id
        );
    }
    if expected_bump != session.data.bump {
        bail!(
            "session {} bump {} does not match derived bump {}",
            session.pubkey,
            session.data.bump,
            expected_bump
        );
    }

    Ok(())
}

fn load_keypair(path: &Path) -> Result<Keypair> {
    read_keypair_file(path)
        .map_err(|error| anyhow!("failed to read keypair from {}: {error}", path.display()))
}

impl From<DataEncodingArg> for DataEncoding {
    fn from(value: DataEncodingArg) -> Self {
        match value {
            DataEncodingArg::Hex => DataEncoding::Hex,
            DataEncodingArg::Base64 => DataEncoding::Base64,
            DataEncodingArg::Utf8 => DataEncoding::Utf8,
        }
    }
}
