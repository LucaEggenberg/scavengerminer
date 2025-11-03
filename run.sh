#!/usr/bin/env bash

export SCAVENGER_API="https://scavenger.prod.gd.midnighttge.io"
export ENABLE_DONATE=false
export DONATE_TO="addr1q8cn7l3uu076wtkgvjzejgv7hjvudvsvgm3hzyq9qqmwjnlapd43a4vqsx85tx56kktz90jj4k3ss7drd8skalunq79sm2jptd"
RUST_LOG=info ./target/release/scavenger-miner mine