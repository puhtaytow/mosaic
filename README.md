<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/mosaic-logo-transparent-bg-rqnsom-mode.png">
  <source media="(prefers-color-scheme: light)" srcset="assets/mosaic-logo-transparent-bg.png">
  <img alt="logo" src="__assets/mosaic-logo-transparent-bg.png">
</picture>

Mosaic is a multi-signature governance solution that enables threshold-based execution against solana programs.

_**Built on Pinocchio. Tested with Mollusk.**_

---

⚠️ Work in Progress & Security Notice

Mosaic is currently work in progress and has not been audited.
Due to the lack of a security audit and the evolving nature of the codebase, it is strongly recommended not to use mainnet.

Use only for development, testing, or experimental purposes until a stable release.

## Configure Mosaic With Your Program

Mosaic stores an instruction, collects approvals for it, and later executes that exact CPI against your program.

### Step by Step

1. Define the instruction in your program that should be governed by Mosaic.
   It should be a normal Solana instruction with explicit instruction data and account metas.

2. Decide which authority your program expects for that instruction.
   If Mosaic should approve and execute it, that authority should be the Mosaic root PDA, passed as one of the CPI accounts with the correct `signer` and `writable` flags.

3. Deploy your program and note its program id.
   This becomes `destination_program` in Mosaic and is locked into the root configuration.

4. Configure `mosaic-cli` for your environment.
   Put `rpc_url`, operator `keypair`, and optionally Mosaic `program_id` in `mosaic-cli/config.json`. The target `destination_program` is passed to `init-root` when you create the root. See [`mosaic-cli/README.md`](mosaic-cli/README.md) and [`mosaic-cli/config.example.json`](mosaic-cli/config.example.json) for the CLI config format.

5. Initialize the Mosaic root.
   Create the root with your operator set and threshold. From that point, only sessions targeting the configured `destination_program` can be executed through that root.

6. Create a signing session with your program instruction.
   Encode the exact instruction data your program expects and include every CPI account the instruction needs.

7. Collect threshold based approvals from operators.

8. Execute the approved session.
   Mosaic performs CPI into your program using the stored instruction data and stored account list. The destination program receives exactly those accounts, so missing or wrong account metas must be fixed at session creation time, not at execute time.

### What Your Program Must Expect

- The destination program id must match the one configured in the Mosaic root.
- All accounts required by the destination instruction must be provided up front in the signing session.
- If your instruction requires a signer authority controlled by Mosaic, use the Mosaic root PDA as that account and mark it correctly in the session account metas.
