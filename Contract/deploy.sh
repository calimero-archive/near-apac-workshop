#!/bin/bash

if [ "$#" -ne 1 ]; then
    echo "Illegal number of parameters (shard_id)"
    exit 1
fi
destination_master_account="$1"

near deploy \
  --accountId "chat-simple.$destination_master_account" \
  --wasmFile target/wasm32-unknown-unknown/release/curb.wasm \
  --initFunction new --initArgs '{"name": "Calimero"}' \
  --nodeUrl "https://api.staging.calimero.network/api/v1/shards/$1-calimero-testnet/neard-rpc" \
  --networkId "$1-calimero-testnet"
