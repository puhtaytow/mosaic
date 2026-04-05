# mosaic-cli

CLI for interacting with the `mosaic` program.

## Build and Run

From the `mosaic-cli` directory:

```bash
cp config.example.json config.json
$EDITOR config.json
cargo build
target/debug/mosaic-cli --help
```

Or run directly through Cargo:

```bash
cargo run -- --help
```

From the repository root:

```bash
cargo run --manifest-path mosaic-cli/Cargo.toml -- --config mosaic-cli/config.json --help
```

## Configuration

`mosaic-cli` reads its connection and signer settings from a JSON config file.

Global option:

```text
--config <PATH>  default: config.json
```

The repository includes [`config.example.json`](/Users/redrum/Projects/puhtaytow/mosaic/mosaic-cli/config.example.json) as a template. Local `config.json` is ignored by git.

Example config:

```json
{
  "rpc_url": "http://127.0.0.1:8899",
  "keypair": "~/.config/solana/id.json",
  "program_id": "<YOUR_MOSAIC_PROGRAM_ID>",
  "destination_program": "<YOUR_TARGET_PROGRAM_ID>",
  "commitment": "confirmed"
}
```

Fields:

- `rpc_url`: Solana RPC endpoint.
- `keypair`: path to the payer/operator keypair.
- `program_id`: Mosaic program id. If omitted, the CLI falls back to the program id compiled into the crate.
- `destination_program`: target program used when creating the root. `init-root` reads it from config by default, but `--destination-program` can still override it.
- `commitment`: one of `processed`, `confirmed`, or `finalized`. If omitted, defaults to `confirmed`.

`keypair` may be absolute, use `~`, or be relative to the config file directory.

## Command Overview

```text
show-root
list-sessions
show-session
init-root
init-session
sign
execute
close-session
```

## Typical Workflow

### 1. Inspect the root account

```bash
target/debug/mosaic-cli show-root
```

### 2. Initialize a root

```bash
target/debug/mosaic-cli init-root \
  --operators <OPERATOR_1> \
  --operators <OPERATOR_2> \
  --threshold 1
```

`destination_program` is read from `config.json`. If you need a one-off override:

```bash
target/debug/mosaic-cli init-root \
  --operators <OPERATOR_1> \
  --operators <OPERATOR_2> \
  --threshold 1 \
  --destination-program <TARGET_PROGRAM_ID>
```

### 3. Create a signing session

```bash
target/debug/mosaic-cli init-session --spec ./session-spec.json
```

### 4. Inspect sessions

```bash
target/debug/mosaic-cli list-sessions
target/debug/mosaic-cli show-session
target/debug/mosaic-cli show-session --session-id 2
target/debug/mosaic-cli show-session --session <SESSION_PDA>
```

For `show-session`, `sign`, `execute`, and `close-session`, omitting both `--session` and `--session-id` targets the latest session.

### 5. Sign a session

```bash
target/debug/mosaic-cli sign
target/debug/mosaic-cli sign --session-id 2
```

### 6. Execute an approved session

```bash
target/debug/mosaic-cli execute --session-id 2
```

If the stored instruction requires additional outer signers:

```bash
target/debug/mosaic-cli execute \
  --session-id 2 \
  --additional-signer ./authority-a.json \
  --additional-signer ./authority-b.json
```

### 7. Close a session

```bash
target/debug/mosaic-cli close-session --session-id 2
```

## Session Spec Format

`init-session` expects a JSON file with this shape:

```json
{
  "data_encoding": "hex",
  "data": "0x01020304",
  "accounts": [
    {
      "pubkey": "11111111111111111111111111111111",
      "writable": false,
      "signer": false
    }
  ]
}
```
