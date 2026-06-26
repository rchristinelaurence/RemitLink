# RemitLink

> OFW remittances from abroad to Philippine families — settled in seconds, not days, at near-zero cost.

---

## Problem
An OFW in Riyadh sends weekly remittances home via Western Union. Their mother in Iloilo waits 2–3 business days and loses 6–9% to fees — ₱40–₱90 gone on every ₱500 sent.

## Solution
The OFW sends USDC directly to their family's Stellar wallet. The transfer settles in under 5 seconds at ~$0.00001. A Soroban escrow contract rate-locks the USDC for a 1-hour window, and a local Philippine anchor (e.g. PDAX) converts USDC to PHP on the recipient's side — zero middlemen, zero wire delays.

---

## Timeline
| Phase | Duration |
|-------|----------|
| Smart contract (Soroban) | Day 1–2 |
| Anchor integration (testnet) | Day 2–3 |
| Mobile frontend (React Native / PWA) | Day 3–4 |
| End-to-end demo polish | Day 4–5 |

---

## Stellar Features Used
- ✅ USDC transfers
- ✅ Trustlines (recipient sets USDC trustline before claiming)
- ✅ Soroban smart contracts (rate-lock escrow)
- ✅ Built-in DEX (USDC → PHP anchor swap)

---

## Vision and Purpose
RemitLink targets the 10+ million OFWs who collectively send $40B+ home to the Philippines annually. By eliminating remittance middlemen and leveraging Stellar's speed and cost, we aim to return billions of pesos per year to Filipino families. The paluwagan (cooperative savings) culture in the Philippines makes community onboarding natural.

---

## Prerequisites
- Rust `1.74+` with `wasm32-unknown-unknown` target
- Soroban CLI `22.0.0`
- Node.js `18+` (for frontend)

rustup target add wasm32-unknown-unknown
cargo install --locked soroban-cli@22.0.0

---

## Build

soroban contract build


Output: `target/wasm32-unknown-unknown/release/remit_link.wasm`

---

## Test


cargo test


Runs all 5 unit tests covering happy path, double-claim prevention, state verification, refund after expiry, and refund-during-window rejection.

---

## Deploy to Testnet

soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/remit_link.wasm \
  --source YOUR_SECRET_KEY \
  --network testnet


Save the returned contract address as `CONTRACT_ID`.

---

## Sample CLI Invocations

### Create a transfer (OFW sends 10 USDC to family)

soroban contract invoke \
  --id $CONTRACT_ID \
  --source OFW_SECRET_KEY \
  --network testnet \
  -- \
  create_transfer \
  --sender GOFW_PUBLIC_KEY \
  --recipient GFAMILY_PUBLIC_KEY \
  --token USDC_CONTRACT_ADDRESS \
  --amount 100000000


Returns: `transfer_id` (e.g. `1`)

### Claim (family member collects USDC)


soroban contract invoke \
  --id $CONTRACT_ID \
  --source FAMILY_SECRET_KEY \
  --network testnet \
  -- \
  claim \
  --transfer_id 1 \
  --token USDC_CONTRACT_ADDRESS


### Refund (OFW reclaims expired transfer)


soroban contract invoke \
  --id $CONTRACT_ID \
  --source OFW_SECRET_KEY \
  --network testnet \
  -- \
  refund \
  --transfer_id 1 \
  --token USDC_CONTRACT_ADDRESS


---

## License
MIT

## Deployed Contract

| Field | Value |
|-------|-------|
| Contract ID | `CB2HTY6UDCB3LDW3P3QKQMOIAVYZKPAGCYYWFRCSRJHGEI3Q5YH5BBER` |
| Network | testnet |
| Explorer | [View on stellar.expert](https://stellar.expert/explorer/testnet/contract/CB2HTY6UDCB3LDW3P3QKQMOIAVYZKPAGCYYWFRCSRJHGEI3Q5YH5BBER) |
| Deploy Tx | [View transaction](https://stellar.expert/explorer/testnet/tx/f7432467c246818b0790153338a7726046f37503346852564df80b0e1f63f288) |
| Deployed | 2026-06-26 07:22:44 UTC |
| Wallet | freighter (`GA33…BSSH`) |
