#!/usr/bin/env bash
set -euo pipefail

export SCAVENGER_API="https://scavenger.prod.gd.midnighttge.io"
export KEYSTORE="/home/luca/keystore"

RUST_LOG=info \
./target/release/scavenger-miner \
    --network mainnet \
    --keystore /home/luca/keystore \
    --enable-donate \
    --donate-to "addr1qyr2wwyg0y626adc6klrrem0l2t0mrjwlkcs4qw9eshf4g0apd43a4vqsx85tx56kktz90jj4k3ss7drd8skalunq79sjyxzad" \
    mine
