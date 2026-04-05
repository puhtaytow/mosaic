use anyhow::{Context, Result, anyhow, bail};
use clap::{Parser, ValueEnum};
use mosaic::state::signing_session::InstructionAccount;
use serde::{Deserialize, Serialize};
use solana_commitment_config::CommitmentConfig;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::{Keypair, read_keypair_file, write_keypair_file};
use solana_pubkey::Pubkey;
use solana_rpc_client::rpc_client::RpcClient;
use solana_signer::Signer;
use solana_system_interface::instruction as system_instruction;
use solana_transaction::Transaction;
use std::{
    fmt::Write as _,
    fs,
    fs::File,
    path::{Path, PathBuf},
    process::Command as ProcessCommand,
    thread::sleep,
    time::{Duration, Instant},
};

const AUTO_RECORD_SPL_DATA_HEX: &str = "0104000000000000000500000068656c6c6f";
const RECORD_SPL_STORAGE_HEADER_LEN: usize = 33;

#[derive(Parser, Debug)]
#[command(
    name = "record-spl-init",
    about = "Bootstrap a full local manual-test environment for Mosaic + record-spl"
)]
struct Cli {
    #[arg(long, value_name = "PATH")]
    config_op0: PathBuf,

    #[arg(long, value_name = "PATH")]
    config_op1: PathBuf,

    #[arg(long, value_name = "PATH")]
    mosaic_program_keypair: PathBuf,

    #[arg(long, value_name = "PATH")]
    record_program_keypair: PathBuf,

    #[arg(long, value_name = "PATH")]
    mosaic_manifest: PathBuf,

    #[arg(long, value_name = "PATH")]
    mosaic_binary: PathBuf,

    #[arg(long, value_name = "PATH")]
    record_program_binary: PathBuf,

    #[arg(long, value_name = "PATH")]
    mosaic_cli_manifest: PathBuf,

    #[arg(long, value_name = "PATH")]
    storage_keypair: PathBuf,

    #[arg(long, default_value_t = 10, value_name = "SOL")]
    airdrop_sol: u64,

    #[arg(long, default_value_t = 133, value_name = "BYTES")]
    account_size: usize,

    #[arg(long, value_enum, default_value_t = Mode::Manual)]
    mode: Mode,

    #[arg(long, default_value_t = 1500, value_name = "MILLISECONDS")]
    pause_ms: u64,

    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    output: OutputFormat,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
enum CommitmentArg {
    Processed,
    Confirmed,
    Finalized,
}

impl CommitmentArg {
    fn into_config(self) -> CommitmentConfig {
        match self {
            CommitmentArg::Processed => CommitmentConfig::processed(),
            CommitmentArg::Confirmed => CommitmentConfig::confirmed(),
            CommitmentArg::Finalized => CommitmentConfig::finalized(),
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum Mode {
    Manual,
    Auto,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ClientConfigFile {
    rpc_url: String,
    keypair: PathBuf,
    #[serde(default)]
    program_id: Option<String>,
    #[serde(default)]
    commitment: Option<CommitmentArg>,
}

#[derive(Debug)]
struct LoadedConfig {
    path: PathBuf,
    file: ClientConfigFile,
    keypair_path: PathBuf,
    commitment: CommitmentConfig,
}

#[derive(Serialize)]
struct BootstrapOutput {
    rpc_url: String,
    operator_0: String,
    operator_1: String,
    mosaic_program_id: String,
    record_program_id: String,
    root_pda: String,
    storage_account: String,
    storage_keypair_path: String,
    account_size: usize,
    storage_account_hex: String,
    root_authority_account_hex: String,
    init_session_accounts: Vec<String>,
    config_op0: String,
    config_op1: String,
    next_steps: Vec<String>,
}

struct AutoFlowContext {
    mosaic_cli_binary: PathBuf,
    config_op0: PathBuf,
    config_op1: PathBuf,
    storage_account: Pubkey,
    pause: Duration,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let mut op0 = load_config(&cli.config_op0)?;
    let mut op1 = load_config(&cli.config_op1)?;

    if op0.file.rpc_url != op1.file.rpc_url {
        bail!(
            "operator configs point to different RPC URLs: `{}` vs `{}`",
            op0.file.rpc_url,
            op1.file.rpc_url
        );
    }

    let rpc = RpcClient::new_with_commitment(op0.file.rpc_url.clone(), op0.commitment.clone());
    rpc.get_latest_blockhash()
        .context("failed to reach the RPC node; start or reset the local validator first")?;

    let op0_keypair = load_keypair(&op0.keypair_path)?;
    let op1_keypair = load_keypair(&op1.keypair_path)?;

    eprintln!("Airdropping operator wallets");
    airdrop(&op0.file.rpc_url, cli.airdrop_sol, &op0_keypair.pubkey())?;
    airdrop(&op0.file.rpc_url, cli.airdrop_sol, &op1_keypair.pubkey())?;

    eprintln!("Building Mosaic SBF binary");
    build_mosaic_program(&cli.mosaic_manifest)?;

    let mosaic_program = load_keypair(&cli.mosaic_program_keypair)?.pubkey();
    let expected_mosaic_program = Pubkey::new_from_array(mosaic::ID);
    if mosaic_program != expected_mosaic_program {
        bail!(
            "mosaic program keypair {} resolves to {}, but the compiled program declares id {}. Use the generated keypair at `mosaic/target/deploy/mosaic-keypair.json` or change `declare_id!` to match",
            expand_path(&cli.mosaic_program_keypair)?.display(),
            mosaic_program,
            expected_mosaic_program
        );
    }

    let record_program = load_keypair(&cli.record_program_keypair)?.pubkey();
    let (root_pda, _) = Pubkey::find_program_address(&[mosaic::ROOT_PDA], &mosaic_program);

    ensure_program_address_is_clean(&rpc, &mosaic_program, "Mosaic")?;
    ensure_program_address_is_clean(&rpc, &record_program, "record-spl")?;

    eprintln!("Deploying Mosaic");
    deploy_program(
        &op0.file.rpc_url,
        &op0.keypair_path,
        &cli.mosaic_binary,
        &cli.mosaic_program_keypair,
    )?;

    eprintln!("Deploying record-spl");
    deploy_program(
        &op0.file.rpc_url,
        &op0.keypair_path,
        &cli.record_program_binary,
        &cli.record_program_keypair,
    )?;

    wait_for_program_executable(&rpc, &mosaic_program, "Mosaic", Duration::from_secs(20))?;
    wait_for_program_executable(&rpc, &record_program, "record-spl", Duration::from_secs(20))?;

    update_config_program_id(&mut op0, &mosaic_program)?;
    update_config_program_id(&mut op1, &mosaic_program)?;

    if account_exists(&rpc, &root_pda)? {
        bail!(
            "root PDA {root_pda} already exists. Reset the local validator before preparing a clean A-Z manual test"
        );
    }

    let (storage_keypair, storage_keypair_path) = create_fresh_keypair(&cli.storage_keypair)?;
    let storage_account = storage_keypair.pubkey();
    let rent = rpc
        .get_minimum_balance_for_rent_exemption(cli.account_size)
        .with_context(|| {
            format!(
                "failed to fetch rent exemption for {} bytes",
                cli.account_size
            )
        })?;

    eprintln!("Creating record-spl storage account");
    let create_instruction = system_instruction::create_account(
        &op0_keypair.pubkey(),
        &storage_account,
        rent,
        cli.account_size as u64,
        &record_program,
    );
    send_instruction(
        &rpc,
        op0.commitment.clone(),
        &op0_keypair,
        &[&op0_keypair, &storage_keypair],
        create_instruction,
    )?;

    eprintln!("Initializing record-spl storage account");
    let initialize_instruction = Instruction {
        program_id: record_program,
        accounts: vec![
            AccountMeta::new(storage_account, false),
            AccountMeta::new_readonly(root_pda, false),
        ],
        data: vec![0],
    };
    send_instruction(
        &rpc,
        op0.commitment.clone(),
        &op0_keypair,
        &[&op0_keypair],
        initialize_instruction,
    )?;

    let storage_account_hex = encode_instruction_account_hex(&storage_account, true, false)?;
    let root_authority_account_hex = encode_instruction_account_hex(&root_pda, false, true)?;
    let config_op0_relative = PathBuf::from("./cli-config-op0.json");
    let config_op1_relative = PathBuf::from("./cli-config-op1.json");

    let output = BootstrapOutput {
        rpc_url: op0.file.rpc_url.clone(),
        operator_0: op0_keypair.pubkey().to_string(),
        operator_1: op1_keypair.pubkey().to_string(),
        mosaic_program_id: mosaic_program.to_string(),
        record_program_id: record_program.to_string(),
        root_pda: root_pda.to_string(),
        storage_account: storage_account.to_string(),
        storage_keypair_path: storage_keypair_path.display().to_string(),
        account_size: cli.account_size,
        storage_account_hex: storage_account_hex.clone(),
        root_authority_account_hex: root_authority_account_hex.clone(),
        init_session_accounts: vec![
            storage_account_hex.clone(),
            root_authority_account_hex.clone(),
        ],
        config_op0: config_op0_relative.display().to_string(),
        config_op1: config_op1_relative.display().to_string(),
        next_steps: vec![
            format!(
                "cargo run --manifest-path ../mosaic-cli/Cargo.toml -- --config {} init-root --operators \"{}\" --operators \"{}\" --threshold 2 --destination-program \"{}\"",
                config_op0_relative.display(),
                op0_keypair.pubkey(),
                op1_keypair.pubkey(),
                record_program
            ),
            format!(
                "cargo run --manifest-path ../mosaic-cli/Cargo.toml -- --config {} init-session --data {} --accounts {} --accounts {}",
                config_op0_relative.display(),
                AUTO_RECORD_SPL_DATA_HEX,
                storage_account_hex,
                root_authority_account_hex
            ),
            format!(
                "cargo run --manifest-path ../mosaic-cli/Cargo.toml -- --config {} list-sessions",
                config_op0_relative.display()
            ),
            format!(
                "cargo run --manifest-path ../mosaic-cli/Cargo.toml -- --config {} sign",
                config_op0_relative.display()
            ),
            format!(
                "cargo run --manifest-path ../mosaic-cli/Cargo.toml -- --config {} sign",
                config_op1_relative.display()
            ),
            format!(
                "cargo run --manifest-path ../mosaic-cli/Cargo.toml -- --config {} execute",
                config_op0_relative.display()
            ),
            format!(
                "cargo run --manifest-path ../mosaic-cli/Cargo.toml -- --config {} close-session",
                config_op0_relative.display()
            ),
        ],
    };

    match cli.mode {
        Mode::Manual => {
            print_bootstrap_output(&output, cli.output, true)?;
        }
        Mode::Auto => {
            let mosaic_cli_binary = build_mosaic_cli_binary(&cli.mosaic_cli_manifest)?;
            print_bootstrap_output(&output, OutputFormat::Text, false)?;
            run_auto_flow(
                &rpc,
                &AutoFlowContext {
                    mosaic_cli_binary,
                    config_op0: op0.path.clone(),
                    config_op1: op1.path.clone(),
                    storage_account,
                    pause: Duration::from_millis(cli.pause_ms),
                },
                &output,
            )?;
        }
    }

    Ok(())
}

fn print_bootstrap_output(
    output: &BootstrapOutput,
    format: OutputFormat,
    include_next_steps: bool,
) -> Result<()> {
    match format {
        OutputFormat::Text => println!("{}", render_bootstrap_text(output, include_next_steps)),
        OutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(output).context("failed to render JSON output")?
        ),
    }
    Ok(())
}

fn render_bootstrap_text(output: &BootstrapOutput, include_next_steps: bool) -> String {
    let mut text = String::new();

    let _ = writeln!(text, "Localnet Environment Ready");
    let _ = writeln!(text);

    let _ = writeln!(text, "Connection");
    let _ = writeln!(text, "  RPC: {}", output.rpc_url);
    let _ = writeln!(text);

    let _ = writeln!(text, "Programs");
    let _ = writeln!(text, "  Mosaic:     {}", output.mosaic_program_id);
    let _ = writeln!(text, "  record-spl: {}", output.record_program_id);
    let _ = writeln!(text, "  Root PDA:   {}", output.root_pda);
    let _ = writeln!(text);

    let _ = writeln!(text, "Operators");
    let _ = writeln!(text, "  Operator 0: {}", output.operator_0);
    let _ = writeln!(text, "  Operator 1: {}", output.operator_1);
    let _ = writeln!(text);

    let _ = writeln!(text, "Storage");
    let _ = writeln!(text, "  Account:      {}", output.storage_account);
    let _ = writeln!(text, "  Keypair file: {}", output.storage_keypair_path);
    let _ = writeln!(text, "  Size:         {} bytes", output.account_size);
    let _ = writeln!(text);

    let _ = writeln!(text, "Session Accounts For `init-session`");
    let _ = writeln!(text, "  Storage account [writable=true, signer=false]");
    let _ = writeln!(text, "    {}", output.storage_account_hex);
    let _ = writeln!(text, "  Root PDA authority [writable=false, signer=true]");
    let _ = writeln!(text, "    {}", output.root_authority_account_hex);
    let _ = writeln!(text);

    let _ = writeln!(text, "Configs Updated");
    let _ = writeln!(text, "  {}", output.config_op0);
    let _ = writeln!(text, "  {}", output.config_op1);
    let _ = writeln!(text);

    if include_next_steps {
        let _ = writeln!(text, "Next Steps");
        let steps = [
            ("Initialize root", format_init_root_command(output)),
            (
                "Create signing session",
                format_init_session_command(output),
            ),
            (
                "Inspect sessions",
                format_list_sessions_command(&output.config_op0),
            ),
            (
                "Sign with operator 0",
                format_sign_command(&output.config_op0),
            ),
            (
                "Sign with operator 1",
                format_sign_command(&output.config_op1),
            ),
            (
                "Execute approved session",
                format_execute_command(&output.config_op0),
            ),
            (
                "Close executed session",
                format_close_session_command(&output.config_op0),
            ),
        ];

        for (index, (label, command)) in steps.iter().enumerate() {
            let _ = writeln!(text, "  {}. {}", index + 1, label);
            for line in command.lines() {
                let _ = writeln!(text, "     {}", line);
            }
            let _ = writeln!(text);
        }
    }

    text.trim_end().to_string()
}

fn format_init_root_command(output: &BootstrapOutput) -> String {
    format!(
        "cargo run --manifest-path ../mosaic-cli/Cargo.toml -- --config {} init-root \\\n  --operators \"{}\" \\\n  --operators \"{}\" \\\n  --threshold 2 \\\n  --destination-program \"{}\"",
        output.config_op0, output.operator_0, output.operator_1, output.record_program_id
    )
}

fn format_init_session_command(output: &BootstrapOutput) -> String {
    format!(
        "cargo run --manifest-path ../mosaic-cli/Cargo.toml -- --config {} init-session \\\n  --data {} \\\n  --accounts {} \\\n  --accounts {}",
        output.config_op0,
        AUTO_RECORD_SPL_DATA_HEX,
        output.storage_account_hex,
        output.root_authority_account_hex
    )
}

fn format_list_sessions_command(config_path: &str) -> String {
    format!(
        "cargo run --manifest-path ../mosaic-cli/Cargo.toml -- --config {} list-sessions",
        config_path
    )
}

fn format_sign_command(config_path: &str) -> String {
    format!(
        "cargo run --manifest-path ../mosaic-cli/Cargo.toml -- --config {} sign",
        config_path
    )
}

fn format_execute_command(config_path: &str) -> String {
    format!(
        "cargo run --manifest-path ../mosaic-cli/Cargo.toml -- --config {} execute",
        config_path
    )
}

fn format_close_session_command(config_path: &str) -> String {
    format!(
        "cargo run --manifest-path ../mosaic-cli/Cargo.toml -- --config {} close-session",
        config_path
    )
}

fn run_auto_flow(rpc: &RpcClient, ctx: &AutoFlowContext, output: &BootstrapOutput) -> Result<()> {
    println!();
    println!("Auto Mode");
    println!("  The environment is ready. The tool will now drive the full Mosaic flow.");

    let steps = vec![
        AutoStep {
            title: "Initialize root",
            explanation: "Creating the Mosaic root PDA with two operators and threshold 2.",
            display_command: format_init_root_command(output),
            args: vec![
                "--config".into(),
                ctx.config_op0.display().to_string(),
                "init-root".into(),
                "--operators".into(),
                output.operator_0.clone(),
                "--operators".into(),
                output.operator_1.clone(),
                "--threshold".into(),
                "2".into(),
                "--destination-program".into(),
                output.record_program_id.clone(),
            ],
        },
        AutoStep {
            title: "Create signing session",
            explanation: "Storing the record-spl write instruction plus both required CPI accounts inside the session.",
            display_command: format_init_session_command(output),
            args: vec![
                "--config".into(),
                ctx.config_op0.display().to_string(),
                "init-session".into(),
                "--data".into(),
                AUTO_RECORD_SPL_DATA_HEX.into(),
                "--accounts".into(),
                output.storage_account_hex.clone(),
                "--accounts".into(),
                output.root_authority_account_hex.clone(),
            ],
        },
        AutoStep {
            title: "Sign with operator 0",
            explanation: "Operator 0 casts the first approval on the freshly created signing session.",
            display_command: format_sign_command(&output.config_op0),
            args: vec![
                "--config".into(),
                ctx.config_op0.display().to_string(),
                "sign".into(),
            ],
        },
        AutoStep {
            title: "Sign with operator 1",
            explanation: "Operator 1 provides the second approval, which should move the session into the approved phase.",
            display_command: format_sign_command(&output.config_op1),
            args: vec![
                "--config".into(),
                ctx.config_op1.display().to_string(),
                "sign".into(),
            ],
        },
        AutoStep {
            title: "Execute approved session",
            explanation: "Executing the stored CPI call against record-spl so the storage account gets mutated.",
            display_command: format_execute_command(&output.config_op0),
            args: vec![
                "--config".into(),
                ctx.config_op0.display().to_string(),
                "execute".into(),
            ],
        },
        AutoStep {
            title: "Inspect final sessions",
            explanation: "Listing sessions after execution so you can confirm the final phase on-chain.",
            display_command: format_list_sessions_command(&output.config_op0),
            args: vec![
                "--config".into(),
                ctx.config_op0.display().to_string(),
                "list-sessions".into(),
            ],
        },
    ];

    let total_steps = steps.len() + 1;
    for (index, step) in steps.iter().enumerate() {
        run_auto_step(ctx, index + 1, total_steps, step)?;
    }

    let storage_bytes = print_storage_raw_bytes(rpc, &ctx.storage_account)?;
    assert_storage_matches_input(&storage_bytes, AUTO_RECORD_SPL_DATA_HEX)?;
    println!();
    println!("Et Voila!");
    println!(
        "  The payload landed on-chain as expected. Cleaning up the executed session account."
    );

    let cleanup_step = AutoStep {
        title: "Close executed session",
        explanation: "Closing the executed signing session account to reclaim rent and leave the localnet state tidy.",
        display_command: format_close_session_command(&output.config_op0),
        args: vec![
            "--config".into(),
            ctx.config_op0.display().to_string(),
            "close-session".into(),
        ],
    };
    run_auto_step(ctx, total_steps, total_steps, &cleanup_step)?;

    Ok(())
}

struct AutoStep {
    title: &'static str,
    explanation: &'static str,
    display_command: String,
    args: Vec<String>,
}

fn run_auto_step(ctx: &AutoFlowContext, index: usize, total: usize, step: &AutoStep) -> Result<()> {
    println!();
    println!("Step {index}/{total}: {}", step.title);
    println!("  {}", step.explanation);
    println!("  Command:");
    for line in step.display_command.lines() {
        println!("    {line}");
    }

    let command_output = run_mosaic_cli_command(&ctx.mosaic_cli_binary, &step.args, step.title)?;
    if !command_output.trim().is_empty() {
        println!("  Output:");
        for line in command_output.lines() {
            println!("    {line}");
        }
    }

    if index < total && !ctx.pause.is_zero() {
        println!(
            "  Waiting {} ms before the next step...",
            ctx.pause.as_millis()
        );
        sleep(ctx.pause);
    }

    Ok(())
}

fn build_mosaic_cli_binary(manifest_path: &Path) -> Result<PathBuf> {
    let manifest_path = expand_path(manifest_path)?;
    let crate_dir = manifest_path.parent().ok_or_else(|| {
        anyhow!(
            "invalid mosaic-cli manifest path {}",
            manifest_path.display()
        )
    })?;

    let mut command = ProcessCommand::new("cargo");
    command
        .arg("build")
        .arg("--quiet")
        .arg("--manifest-path")
        .arg(&manifest_path);
    run_command(command, "build mosaic-cli binary")?;

    Ok(crate_dir.join("target/debug/mosaic-cli"))
}

fn run_mosaic_cli_command(
    binary_path: &Path,
    args: &[String],
    description: &str,
) -> Result<String> {
    let mut command = ProcessCommand::new(binary_path);
    command.args(args);
    run_command(command, &format!("run mosaic-cli step `{description}`"))
}

fn print_storage_raw_bytes(rpc: &RpcClient, storage_account: &Pubkey) -> Result<Vec<u8>> {
    let account = rpc
        .get_account(storage_account)
        .map_err(|error| anyhow!("failed to fetch storage account {storage_account}: {error}"))?;

    println!();
    println!("Storage Account Raw Bytes");
    println!("  Pubkey: {}", storage_account);
    println!("  Length: {} bytes", account.data.len());
    println!("  Bytes: {:?}", account.data);
    println!("  Hex: {}", encode_hex(&account.data));
    Ok(account.data)
}

fn assert_storage_matches_input(storage_bytes: &[u8], instruction_data_hex: &str) -> Result<()> {
    let instruction_data = decode_hex(instruction_data_hex)?;
    if instruction_data.len() < 13 {
        bail!(
            "cannot verify record-spl write payload `{instruction_data_hex}`: payload is too short"
        );
    }

    let discriminator = instruction_data[0];
    if discriminator != 1 {
        bail!(
            "cannot verify record-spl write payload `{instruction_data_hex}`: expected discriminator 1, got {discriminator}"
        );
    }

    let offset = u64::from_le_bytes(
        instruction_data[1..9]
            .try_into()
            .map_err(|_| anyhow!("failed to decode record-spl write offset"))?,
    ) as usize;
    let length = u32::from_le_bytes(
        instruction_data[9..13]
            .try_into()
            .map_err(|_| anyhow!("failed to decode record-spl write length"))?,
    ) as usize;
    let expected = instruction_data.get(13..13 + length).ok_or_else(|| {
        anyhow!("record-spl write payload length does not match its data section")
    })?;

    let start = RECORD_SPL_STORAGE_HEADER_LEN + offset;
    let end = start + length;
    let actual = storage_bytes.get(start..end).ok_or_else(|| {
        anyhow!(
            "storage account is too short to contain {} byte(s) at offset {} after the {}-byte header",
            length,
            offset,
            RECORD_SPL_STORAGE_HEADER_LEN
        )
    })?;

    if actual != expected {
        bail!(
            "storage assertion failed: expected bytes `{}` at record-spl data offset {}, found `{}`",
            encode_hex(expected),
            offset,
            encode_hex(actual)
        );
    }

    println!();
    println!("Storage Assertion");
    println!(
        "  Input payload bytes match the storage account at record-spl data offset {}.",
        offset
    );
    println!("  Expected: {}", encode_hex(expected));
    println!("  Actual:   {}", encode_hex(actual));
    Ok(())
}

fn load_config(path: &Path) -> Result<LoadedConfig> {
    let path = expand_path(path)?;
    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let file = File::open(&path)
        .with_context(|| format!("failed to open config file {}", path.display()))?;
    let config: ClientConfigFile = serde_json::from_reader(file)
        .with_context(|| format!("failed to parse config file {}", path.display()))?;

    Ok(LoadedConfig {
        path: path.clone(),
        keypair_path: resolve_config_path(base_dir, &config.keypair)?,
        commitment: config
            .commitment
            .unwrap_or(CommitmentArg::Confirmed)
            .into_config(),
        file: config,
    })
}

fn update_config_program_id(config: &mut LoadedConfig, program_id: &Pubkey) -> Result<()> {
    config.file.program_id = Some(program_id.to_string());
    let file = File::create(&config.path)
        .with_context(|| format!("failed to rewrite config file {}", config.path.display()))?;
    serde_json::to_writer_pretty(file, &config.file)
        .with_context(|| format!("failed to serialize config file {}", config.path.display()))?;
    Ok(())
}

fn resolve_config_path(base_dir: &Path, path: &Path) -> Result<PathBuf> {
    let path = expand_path(path)?;
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(base_dir.join(path))
    }
}

fn expand_path(path: &Path) -> Result<PathBuf> {
    let raw = path
        .to_str()
        .ok_or_else(|| anyhow!("path {} is not valid UTF-8", path.display()))?;
    let expanded = shellexpand::full(raw)
        .map_err(|error| anyhow!("failed to expand path {}: {error}", path.display()))?;
    Ok(PathBuf::from(expanded.as_ref()))
}

fn load_keypair(path: &Path) -> Result<Keypair> {
    let path = expand_path(path)?;
    read_keypair_file(&path)
        .map_err(|error| anyhow!("failed to read keypair file {}: {error}", path.display()))
}

fn create_fresh_keypair(path: &Path) -> Result<(Keypair, PathBuf)> {
    let path = expand_path(path)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory {}", parent.display()))?;
    }

    let keypair = Keypair::new();
    write_keypair_file(&keypair, &path)
        .map_err(|error| anyhow!("failed to write keypair file {}: {error}", path.display()))?;
    Ok((keypair, path))
}

fn airdrop(rpc_url: &str, amount_sol: u64, recipient: &Pubkey) -> Result<()> {
    let mut command = ProcessCommand::new("solana");
    command
        .arg("airdrop")
        .arg(amount_sol.to_string())
        .arg(recipient.to_string())
        .arg("--url")
        .arg(rpc_url);
    run_command(command, &format!("airdrop {amount_sol} SOL to {recipient}"))?;
    Ok(())
}

fn build_mosaic_program(manifest_path: &Path) -> Result<()> {
    let manifest_path = expand_path(manifest_path)?;
    let mut command = ProcessCommand::new("cargo");
    command
        .arg("build-sbf")
        .arg("--manifest-path")
        .arg(&manifest_path);
    run_command(command, "build Mosaic SBF binary")?;
    Ok(())
}

fn deploy_program(
    rpc_url: &str,
    signer_keypair: &Path,
    program_binary: &Path,
    program_keypair: &Path,
) -> Result<()> {
    let signer_keypair = expand_path(signer_keypair)?;
    let program_binary = expand_path(program_binary)?;
    let program_keypair = expand_path(program_keypair)?;

    let mut command = ProcessCommand::new("solana");
    command
        .arg("program")
        .arg("deploy")
        .arg(&program_binary)
        .arg("--program-id")
        .arg(&program_keypair)
        .arg("--keypair")
        .arg(&signer_keypair)
        .arg("--fee-payer")
        .arg(&signer_keypair)
        .arg("--url")
        .arg(rpc_url);

    run_command(
        command,
        &format!(
            "deploy program {} with id {}",
            program_binary.display(),
            program_keypair.display()
        ),
    )?;
    Ok(())
}

fn run_command(mut command: ProcessCommand, description: &str) -> Result<String> {
    let output = command
        .output()
        .with_context(|| format!("failed to start command for {description}"))?;

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let details = match (stdout.is_empty(), stderr.is_empty()) {
            (false, false) => format!("stdout:\n{stdout}\n\nstderr:\n{stderr}"),
            (false, true) => format!("stdout:\n{stdout}"),
            (true, false) => format!("stderr:\n{stderr}"),
            (true, true) => String::from("no command output"),
        };
        bail!("failed to {description}: {details}");
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn ensure_program_address_is_clean(
    rpc: &RpcClient,
    program_id: &Pubkey,
    label: &str,
) -> Result<()> {
    if let Some(account) = get_optional_account(rpc, program_id)? {
        if !account.executable {
            bail!(
                "{label} program id {program_id} already exists as a non-executable account. Reset the local validator before bootstrapping a fresh test environment"
            );
        }
    }
    Ok(())
}

fn wait_for_program_executable(
    rpc: &RpcClient,
    program_id: &Pubkey,
    label: &str,
    timeout: Duration,
) -> Result<()> {
    let started_at = Instant::now();
    let poll_interval = Duration::from_millis(500);
    let mut last_state = String::from("account not found yet");

    while started_at.elapsed() < timeout {
        match get_optional_account(rpc, program_id)? {
            Some(account) if account.executable => return Ok(()),
            Some(account) => {
                last_state = format!(
                    "account exists but executable=false (owner={})",
                    account.owner
                );
            }
            None => {
                last_state = String::from("account not found yet");
            }
        }

        sleep(poll_interval);
    }

    bail!(
        "{label} program {program_id} did not become executable within {}s after deploy; last observed state: {last_state}",
        timeout.as_secs()
    );
}

fn account_exists(rpc: &RpcClient, pubkey: &Pubkey) -> Result<bool> {
    Ok(get_optional_account(rpc, pubkey)?.is_some())
}

fn get_optional_account(
    rpc: &RpcClient,
    pubkey: &Pubkey,
) -> Result<Option<solana_account::Account>> {
    match rpc.get_account(pubkey) {
        Ok(account) => Ok(Some(account)),
        Err(error) => {
            let error_text = error.to_string();
            if error_text.contains("AccountNotFound") {
                Ok(None)
            } else {
                Err(anyhow!("failed to fetch account {pubkey}: {error}"))
            }
        }
    }
}

fn send_instruction(
    rpc: &RpcClient,
    commitment: CommitmentConfig,
    payer: &Keypair,
    signers: &[&dyn Signer],
    instruction: Instruction,
) -> Result<String> {
    let blockhash = rpc
        .get_latest_blockhash()
        .context("failed to fetch latest blockhash")?;
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        signers,
        blockhash,
    );
    let signature = rpc
        .send_and_confirm_transaction_with_spinner_and_commitment(&transaction, commitment)
        .context("failed to send transaction")?;
    Ok(signature.to_string())
}

fn encode_instruction_account_hex(pubkey: &Pubkey, writable: bool, signer: bool) -> Result<String> {
    let account = InstructionAccount {
        pubkey: *pubkey.as_array(),
        signer,
        writable,
    };
    let serialized = account
        .serialize()
        .map_err(|error| anyhow!("failed to serialize instruction account {pubkey}: {error:?}"))?;
    Ok(encode_hex(&serialized.0))
}

fn encode_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

fn decode_hex(value: &str) -> Result<Vec<u8>> {
    let trimmed = value.trim();
    let hex = trimmed.strip_prefix("0x").unwrap_or(trimmed);

    if hex.len() % 2 != 0 {
        bail!("hex data must have an even number of characters");
    }

    let mut output = Vec::with_capacity(hex.len() / 2);
    let bytes = hex.as_bytes();
    for index in (0..bytes.len()).step_by(2) {
        let chunk = std::str::from_utf8(&bytes[index..index + 2]).unwrap_or_default();
        let byte = u8::from_str_radix(chunk, 16)
            .with_context(|| format!("invalid hex byte `{chunk}` at offset {index}"))?;
        output.push(byte);
    }

    Ok(output)
}
