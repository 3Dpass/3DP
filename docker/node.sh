#!/bin/bash
./p3d import-mining-key "$MEMO_SEED" --base-path /var/chain --chain mainnetSpecRaw.json
./p3d --unsafe-ws-external --unsafe-rpc-external --rpc-cors=all --no-mdns \
  --validator --base-path /var/chain --author "$ADDRESS"
