#!/usr/bin/env bash
set -euo pipefail

export SCAVENGER_API="https://scavenger.prod.gd.midnighttge.io"
export KEYSTORE="/home/luca/keystore"

RUST_LOG=info \
./target/release/scavenger-miner \
    --network mainnet \
    --keystore /home/luca/keystore \
    --enable-donate \
    --donate-to "addr1q8cn7l3uu076wtkgvjzejgv7hjvudvsvgm3hzyq9qqmwjnlapd43a4vqsx85tx56kktz90jj4k3ss7drd8skalunq79sm2jptd" \
    mine
