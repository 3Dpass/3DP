#!/bin/bash
./p3d import-mining-key "$MEMO_SEED" --base-path /var/chain --chain mainnetSpecRaw.json
GRANDPA_KEY="$(./p3d key inspect --scheme Ed25519 "$MEMO_SEED" | sed -n 's/.*Secret seed://p')"
./p3d key insert --base-path /var/chain --chain mainnetSpecRaw.json --scheme Ed25519 --key-type gran --suri $GRANDPA_KEY
./p3d --chain mainnetSpecRaw.json --unsafe-ws-external --unsafe-rpc-external --rpc-cors=all --no-mdns \
  --threads 8 --validator --base-path /var/chain --author "$ADDRESS" --name MyNodeName --telemetry-url "wss://submit.telemetry.3dpscan.io/submit 0"
