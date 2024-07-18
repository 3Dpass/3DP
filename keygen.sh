#! /bin/bash
# generating new account
MEMO_SEED="$(./target/release/poscan-consensus generate-mining-key --base-path ~/3dp-chain/ --chain mainnetSpecRaw.json | sed -n 's/.*Secret seed://p')"
ACCOUNT="$(./target/release/poscan-consensus key inspect --scheme Sr25519 "$MEMO_SEED")"
echo "----Mining account----"
echo "$ACCOUNT"
# importing mining key
./target/release/poscan-consensus import-mining-key "$MEMO_SEED" --base-path ~/3dp-chain/ --chain mainnetSpecRaw.json
# deriving GRANDPA key from the seed phrase
GRANDPA_KEY="$(./target/release/poscan-consensus key inspect --scheme Ed25519 "$MEMO_SEED" | sed -n 's/.*Secret seed://p')"
GRANDPA_ACCOUNT="$(./target/release/poscan-consensus key inspect --scheme Ed25519 "$MEMO_SEED")"
echo "---- GRANDPA key derived ----:"
echo "$GRANDPA_ACCOUNT"
# inserting GRANDPA key
./target/release/poscan-consensus key insert --scheme Ed25519 --base-path ~/3dp-chain/ --chain mainnetSpecRaw.json --key-type gran --suri $GRANDPA_KEY
# inserting ImOnline key
./target/release/poscan-consensus key insert --scheme Sr25519 --base-path ~/3dp-chain/ --chain mainnetSpecRaw.json --key-type imon --suri $GRANDPA_KEY
echo "These are the keys in your keystore ~/3dp-chain/chains/3dpass/keystore/..."
# checking up the keys in the keystore
ls "../3dp-chain/chains/3dpass/keystore/"
