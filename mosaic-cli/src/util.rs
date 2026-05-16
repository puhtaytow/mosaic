use anyhow::{Context, Result, anyhow, bail};
use borsh::to_vec;
use pinocchio::Address;
use serde::Serialize;
use solana_instruction::AccountMeta;
use solana_pubkey::Pubkey;
use std::{
    fmt::Write as _,
    path::{Path, PathBuf},
};

use mosaic::state::signing_session::SigningSessionPhase;

pub(crate) fn instruction_data<T: borsh::BorshSerialize>(
    opcode: u8,
    payload: &T,
) -> Result<Vec<u8>> {
    let mut data = Vec::with_capacity(1);
    data.push(opcode);
    data.extend_from_slice(&to_vec(payload).context("failed to serialize instruction data")?);
    Ok(data)
}

pub(crate) fn pubkey_to_address(pubkey: &Pubkey) -> Address {
    Address::new_from_array(*pubkey.as_array())
}

pub(crate) fn address_to_pubkey(address: &Address) -> Pubkey {
    Pubkey::new_from_array(*address.as_array())
}

pub(crate) fn address_to_string(address: &Address) -> String {
    address_to_pubkey(address).to_string()
}

pub(crate) fn addresses_to_strings(addresses: &[Address]) -> Vec<String> {
    addresses.iter().map(address_to_string).collect()
}

pub(crate) fn pubkeys_to_strings(pubkeys: &[Pubkey]) -> Vec<String> {
    pubkeys.iter().map(ToString::to_string).collect()
}

pub(crate) fn account_meta(pubkey: Pubkey, writable: bool, signer: bool) -> AccountMeta {
    match (writable, signer) {
        (true, true) => AccountMeta::new(pubkey, true),
        (true, false) => AccountMeta::new(pubkey, false),
        (false, true) => AccountMeta::new_readonly(pubkey, true),
        (false, false) => AccountMeta::new_readonly(pubkey, false),
    }
}

pub(crate) fn phase_to_string(phase: SigningSessionPhase) -> &'static str {
    match phase {
        SigningSessionPhase::Uninitialized => "uninitialized",
        SigningSessionPhase::Active => "active",
        SigningSessionPhase::Approved => "approved",
        SigningSessionPhase::Executed => "executed",
    }
}

pub(crate) fn encode_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

pub(crate) fn decode_hex(value: &str) -> Result<Vec<u8>> {
    let trimmed = value.trim();
    let hex = trimmed.strip_prefix("0x").unwrap_or(trimmed);

    if hex.len() % 2 != 0 {
        bail!("hex instruction data must have an even number of characters");
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

pub(crate) fn print_json<T: Serialize>(value: &T) -> Result<()> {
    println!(
        "{}",
        serde_json::to_string_pretty(value).context("failed to render JSON output")?
    );
    Ok(())
}

pub(crate) fn expand_path(path: &Path) -> Result<PathBuf> {
    let raw = path
        .to_str()
        .ok_or_else(|| anyhow!("path {} is not valid UTF-8", path.display()))?;
    let expanded = shellexpand::full(raw)
        .map_err(|error| anyhow!("failed to expand path {}: {error}", path.display()))?;
    Ok(PathBuf::from(expanded.as_ref()))
}
