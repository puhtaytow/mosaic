# Mosaic Program Flow

Mosaic is basically a multisig control layer:
- a `Root` PDA stores governance configuration,
- `SigningSession` PDAs store proposals and approvals,
- once threshold is hit, Mosaic executes via CPI to your destination pgoram.

## Authority delegation model

This is the authority handoff.
External program points its privileged authority to Mosaic `Root` PDA.
After that, no single key can do admin stuff solo; multisig rules gate it.

```mermaid
flowchart LR
    A0[User program admin] --> A1[Set user program authority to Mosaic Root PDA]
    A1 --> A2[Mosaic Root PDA becomes privileged authority]
    A2 --> A3[Mosaic multisig approves action]
    A3 --> A4[Mosaic Execute performs CPI as Root PDA signer]
    A4 --> A5[User program accepts privileged action]
```

## Use case overview

Typical lifecycle:
one-time setup, then loop proposal -> sign -> execute for each governed action.

```mermaid
flowchart LR
    U0[Operators multisig group] --> U1[Initialize root configuration]
    U1 --> U2[Create signing session proposal]
    U2 --> U3[Collect signatures]
    U3 --> U4[Mark session approved]
    U4 --> U5[Execute governed CPI]
    U5 --> U6[Mark session executed]
    U6 --> U7[Destination program action applied]
```

## Core instruction flows (side by side)

Same flows as 2.2-2.5, just packed side by side.

```mermaid
flowchart LR
    O1[InitializeOperators] --> O2[InitializeSigningSession] --> O3[Sign] --> O4[Execute]

    subgraph ROW[ ]
        direction LR

        subgraph IRO[InitializeOperators]
            direction TB
            A0[Operator] --> A1[Create root PDA account]
            A1 --> A2[Write Root fields operators last_id threshold destination_program bump]
        end

        subgraph ISS[InitializeSigningSession]
            direction TB
            B0[Operator] --> B1[Load and validate Root PDA]
            B1 --> B2[Increment Root last_id]
            B2 --> B3[Derive and validate SigningSession PDA]
            B3 --> B4[Create signing session account]
            B4 --> B5[Write Root and SigningSession phase Active]
        end

        subgraph SIG[Sign]
            direction TB
            C0[Operator] --> C1[Load Root and SigningSession]
            C1 --> C2[Validate PDAs and active session]
            C2 --> C3[Add operator approval]
            C3 --> C4[If threshold reached set phase Approved]
            C4 --> C5[Write updated SigningSession]
        end

        subgraph EXE[Execute]
            direction TB
            E0[Operator] --> E1[Load Root and SigningSession]
            E1 --> E2[Validate PDAs session phase and destination program]
            E2 --> E3[Build CPI metas from stored instruction accounts]
            E3 --> E4[Lock session phase Executed and write state]
            E4 --> E5[Invoke CPI with Root PDA signer]
            E5 --> E6[Finish execution]
        end
    end

    O1 -.-> A0
    O2 -.-> B0
    O3 -.-> C0
    O4 -.-> E0
```

## Signing session lifecycle

Signing session lifecycle is linear.
Proposal moves forward only: create -> approve -> execute.

```mermaid
flowchart LR
    S0[Uninitialized] --> S1[Active]
    S1 --> S2[Approved]
    S2 --> S3[Executed]
```

## PDA formulas

These PDA formulas are the address truth source.

The `Root PDA` comes from the static `root_pda` seed plus its bump.
The `SigningSession PDA` comes from the root address, the session id, the `signing_session_pda` seed, and its bump.

What matters most is the binding: the root address is part of the signing session derivation.
So a signing session created for one root is automatically invalid for any other root.