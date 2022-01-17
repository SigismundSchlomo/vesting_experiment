#!/bin/bash
set -e

export REF_EXCHANGE_ADDRESS="ref-finance-101.testnet"
export REF_FARMING_ADDRESS="v2.ref-farming.testnet"
export REFERRAL_ACCOUNT="kuznietsov.testnet"

# This flags are needed to reduce size of compiled wasm file
RUSTFLAGS='-C link-arg=-s' cargo build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/vesting.wasm ./res/vesting.wasm
