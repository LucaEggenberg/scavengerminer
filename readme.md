# Quickstart (Fully Functional)

## Enter dev shell
nix develop

## Build
cargo build --release

## Run miner on preprod (default)
RUST_LOG=info \
SCAVENGER_API=https://scavenger.prod.gd.midnighttge.io \
NETWORK=preprod \
./target/release/scavenger-miner mine

## Other commands
./target/release/scavenger-miner challenge
./target/release/scavenger-miner gen-addr

---

### Notes matching the API docs
- Address format: **Shelley enterprise** (key payment credential), bech32 `addr` (mainnet) or `addr_test` (preprod). Pubkey is 32 bytes (64 hex).
- Registration signature: **CIP-8 / COSE_Sign1 Ed25519** over the exact T&C `message` string. The miner builds a compliant COSE_Sign1 and hex-encodes it for `/register`.
- Preimage for PoW: `nonce_hex + address + challenge_id + difficulty + no_pre_mine + latest_submission + no_pre_mine_hour` (exact order). Hashed with **AshMaize** configured per the docs, and difficulty applied to the left-most 4 bytes.
- ROM init: uses `no_pre_mine` per challenge day as specified.

### What you can tweak
- `--workers` to scale threads per challenge
- `--network mainnet` when ready for mainnetd
- `--keystore ./keystore` location for saved keys