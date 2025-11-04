#!/usr/bin/env bash
set -euo pipefail

export SCAVENGER_API="https://scavenger.prod.gd.midnighttge.io"

RUST_LOG=info \
./target/release/scavenger-miner \
    --network mainnet \
    --workers 8 \
    --keystore keystore_mainnet \
    mine
# --enable-donate \
# --donate-to "addr1q8cn7l3uu076wtkgvjzejgv7hjvudvsvgm3hzyq9qqmwjnlapd43a4vqsx85tx56kktz90jj4k3ss7drd8skalunq79sm2jptd" \