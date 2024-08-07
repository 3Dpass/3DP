#!/bin/bash
# importing mining key
./p3d import-mining-key "$MEMO_SEED" --base-path /var/chain --chain mainnetSpecRaw.json
# deriving GRANDPA key 
GRANDPA_KEY="$(./p3d key inspect --scheme Ed25519 "$MEMO_SEED" | sed -n 's/.*Secret seed://p')"
# inserting GRANDPA key
./p3d key insert --base-path /var/chain --chain mainnetSpecRaw.json --scheme Ed25519 --key-type gran --suri $GRANDPA_KEY
# inserting ImOnline key
./p3d key insert --scheme Sr25519 --base-path /var/chain --chain mainnetSpecRaw.json --key-type imon --suri $GRANDPA_KEY
# running the Node
./p3d --chain mainnetSpecRaw.json --unsafe-ws-external --unsafe-rpc-external --rpc-cors=all --no-mdns --validator --base-path /var/chain --author "$ADDRESS" --telemetry-url "wss://submit.telemetry.3dpscan.io/submit 0" --name MY_NODE_NAME
