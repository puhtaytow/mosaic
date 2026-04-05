# `mosaic-cli`

`mosaic-cli` talks to the deployed Mosaic program and uses a local JSON config file for RPC and signer settings.

## Config file

Pass a config path with `--config PATH`. If omitted, the CLI looks for `config.json` in the current working directory.

Example:

```json
{
  "rpc_url": "http://127.0.0.1:8899",
  "keypair": "~/.config/solana/id.json",
  "program_id": "<YOUR_MOSAIC_PROGRAM_ID>",
  "commitment": "confirmed"
}
```

Fields:

- `rpc_url` is required.
- `keypair` is required and should point to the payer/operator keypair file.
- `program_id` is optional. If omitted, the CLI uses the built-in Mosaic program id from the crate.
- `commitment` is optional. Supported values are `processed`, `confirmed`, and `finalized`. The default is `confirmed`.

Relative paths inside the config file are resolved relative to the config file location.

## Root setup

The governed destination program is not stored in the CLI config file. Pass it when creating the Mosaic root:

```bash
cargo run --manifest-path mosaic-cli/Cargo.toml -- \
  --config mosaic-cli/config.json \
  init-root \
  --operators <OPERATOR_PUBKEY> \
  --threshold 1 \
  --destination-program <DESTINATION_PROGRAM_ID>
```

## Common commands

```bash
# show the current root
cargo run -- --config mosaic-cli/config.json show-root

# list sessions
cargo run -- --config mosaic-cli/config.json list-sessions

# show the latest session
cargo run -- --config mosaic-cli/config.json show-session

# sign the latest session
cargo run -- --config mosaic-cli/config.json sign
```

## Session spec format

`init-session --spec PATH` expects JSON in this shape:

```json
{
  "data": "01020304",
  "accounts": [
    {
      "pubkey": "<ACCOUNT_PUBKEY>",
      "writable": true,
      "signer": false
    }
  ]
}
```
