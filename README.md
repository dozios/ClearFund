# ClearFund

Milestone-based charity transparency escrow, built on Stellar with Soroban smart contracts.

## Problem

Amara, a field coordinator for a flood-relief NGO in Nairobi, Kenya, receives $50,000 in wire
donations from diaspora donors in London and Toronto but has no way to prove to them exactly how
the money was spent. After one quarter with vague reporting, three major recurring donors cut
their giving by 40% the following year.

## Solution

Donors deposit USDC into a Soroban escrow contract tied to specific, pre-defined relief
milestones (e.g., "500 food kits delivered"). Funds sit locked on-chain until a whitelisted,
independent local auditor confirms milestone completion, at which point the contract releases
the exact tranche to the NGO's wallet — automatically, with no intermediary able to intercept or
misreport it. Stellar's sub-cent fees and ~5 second settlement make tracking dozens of small
milestones (instead of one lump-sum wire) economically viable, and Soroban's on-chain logic
removes the need for donors to simply trust the NGO's word.

## Timeline

- Hour 0–2: Contract design, storage schema, milestone data model
- Hour 2–6: `create_campaign`, `donate`, `approve_milestone`, `get_campaign` implementation
- Hour 6–8: Test suite (happy path, edge cases, state verification)
- Hour 8–10: Frontend wiring (Freighter wallet connect, campaign dashboard)
- Hour 10–12: Testnet deployment, demo rehearsal, README/pitch polish

## Stellar Features Used

- **USDC transfers** — the funding currency for all donations and payouts
- **Soroban smart contracts** — encode the escrow and milestone-release logic
- **Trustlines** — donors and the NGO wallet hold USDC via standard Stellar trustlines

## Vision and Purpose

Global giving runs on trust that is expensive to verify and easy to lose. ClearFund replaces
"trust me" reporting with a public, tamper-proof ledger of exactly which outcomes were funded and
when. The long-term vision is a library of reusable milestone templates (disaster relief,
scholarships, medical aid) that any NGO can spin up in minutes, giving every donor — from a $20
diaspora gift to a $500,000 institutional grant — the same level of verifiable accountability.

## Prerequisites

- Rust (stable, 1.79+) with the `wasm32-unknown-unknown` target:
  ```bash
  rustup target add wasm32-unknown-unknown
  ```
- Soroban CLI (v21+):
  ```bash
  cargo install --locked soroban-cli
  ```

## How to Build

```bash
soroban contract build
```

The compiled Wasm binary will be at `target/wasm32-unknown-unknown/release/clear_fund.wasm`.

## How to Test

```bash
cargo test
```

Runs all 5 tests: happy-path milestone release, insufficient-escrow-balance rejection,
post-donation state verification, double-release prevention, and event-emission verification.

## How to Deploy to Testnet

```bash
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/clear_fund.wasm \
  --source <YOUR_TESTNET_IDENTITY> \
  --network testnet
```

This prints the deployed `CONTRACT_ID` — use it in the CLI invocation below.

## Sample CLI Invocation (MVP function)

Create a campaign with one milestone ("food_kits", 500 units of the token):

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source <YOUR_TESTNET_IDENTITY> \
  --network testnet \
  -- \
  create_campaign \
  --ngo <NGO_ADDRESS> \
  --auditor <AUDITOR_ADDRESS> \
  --token <USDC_TOKEN_CONTRACT_ID> \
  --milestone_descriptions '["food_kits"]' \
  --milestone_amounts '[500]'
```

Donate to the campaign:

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source <DONOR_IDENTITY> \
  --network testnet \
  -- \
  donate \
  --donor <DONOR_ADDRESS> \
  --campaign_id 1 \
  --amount 500
```

Approve the milestone (auditor only) and trigger release to the NGO:

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source <AUDITOR_IDENTITY> \
  --network testnet \
  -- \
  approve_milestone \
  --campaign_id 1 \
  --milestone_index 0
```

## License

MIT
🔗 https://stellar.expert/explorer/testnet/tx/f75d11481574464df1258ce4dd7cdff4d4c36d33639927cde776fc35656db358
🔗 https://lab.stellar.org/r/testnet/contract/CBKDTLWKWRCQR3FIA4IUGTNHSFN5EB4D46N6OTEWGYOHRGS5W7OKVK3U
