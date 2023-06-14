import {WsProvider} from "@polkadot/api";

// main net ss58 format
export const ss58Format = 71;
export const one_p3d = 1_000_000_000_000n;

// target/release/poscan-consensus --base-path ~/3dp-chains/test --chain testnetSpecRaw.json --no-prometheus --no-mdns --rpc-cors all --pruning=archive
export const provider = new WsProvider("ws://127.0.0.1:9944");
