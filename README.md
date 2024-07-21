
[![logo](https://github.com/3Dpass/3DP/assets/107915078/11de380f-7f77-4cf6-831d-e6ac8d7ab1ec)](https://3dpass.org)


# The Ledger of Things Node

"The Ledger of Things" is a revolutionary open source Layer 1 blockchain platform for the tokenization of objects. [White Paper](https://3dpass.org/coin#white-papper). Current list of the object categories is presented as follows: 3D objects, 2D drawings, Music, Biometrics, Radio signals, Movements, Texts.

[Proof of Scan](https://github.com/3Dpass/whitepaper/blob/main/3DPass_white_paper_v2.pdf) is a decentralized protocol (PoW `ASIC resistant, CPU oriented` + PoS), which is based on recognition technology. Every object, transformed by 3DPass, obtains its own unique and sustainable identity called HASH ID the object could be recognized by. This will prevent the copying of digital assets and thus open a door for the entire blockchain space to potentially trillions in deals all over the globe.

[Grid2d](https://3dpass.org/grid2d) is the first 3D shape recognition algorithm, which is being utilized as the hash function in the Proof of Scan protocol. The implementations of the algorithm are the [pass3d](https://github.com/3Dpass/pass3d) recognition toolkit and its WASM analog [p3d](https://github.com/3Dpass/p3d). 

[3DPRC-2](https://github.com/3Dpass/whitepaper/blob/main/3DPRC-2.md) (3Dpass Request for Comments) is a standard p2p protocol for the tokenization of the User objects operating within “The Ledger of Things”, by which the most useful aspect of the "Proof of Scan" consensus is getting uncovered. 3DPRC-2 provides decentralized [PoScan API](https://github.com/3Dpass/3DP/wiki/3DPRC%E2%80%902-PoScan-API) available for customers.

The scope of potential 3Dpass applications goes way beyond 3D object recognition and not limited to. Being naturally organized and still cultivating this community driven spirit, 3Dpass is here to encourage developers from all around the globe to upgrade the pass3d open source toolkit with new fascinating recognition algorithms and make it even more useful for human civilization. Learn more about the [algorithm requirements](http://localhost:3000/proof-of-scan#object).

[3DPass Coin (P3D)](https://3dpass.org/coin) is a native utility token, operating on "The Ledger of Things", which serves to incentivize community members to maintain the network infrastructure. Such aspects as: Storage fee, Gas fee, The object authentication fee, Transaction fee, The validator collaterals, Penalties - are all being counted in P3D.

[Contribution program](https://3dpass.org/coin#distribution-contribution)  |  [Contribution guidelines](https://github.com/3Dpass/3DP/blob/main/CONTRIBUTING.md)  |  [Discord](https://discord.gg/u24WkXcwug)

# Integration

- NODE - the Node leverages [Substrate](https://substrate.io/) framework and implemented as two-piece design `Rust native` part and `Runtime` part ([WASM](https://webassembly.org/)-based), which is upgradable online and allows for multiple useful modules to operate (see more [forkless upgrade](https://3dpass.org/proof-of-scan#forkless-upgrade)). 
- RPC (remote procedure call) - the capabilities that allow blockchain users to interact with the network. The NODE provides HTTP and [WebSocket](https://github.com/3Dpass/3DP/wiki/Set-up-WSS-for-Remote-Connections) RPC servers.
- CORE Networking - the [`libp2p`](https://libp2p.io/) is used as as networking stack for the Nodes to communicate with each other.
- [3Dpass light wallet](https://github.com/3Dpass/wallet) - desktop users and 3D printing labs integration
- [Pass3d mobile](https://github.com/3Dpass/threedpass) - smartphone and tablets users integration

 <img width="719" alt="Node_integration" src="https://github.com/user-attachments/assets/93f186eb-c9db-4fd8-96b7-efc3b451a6b8">


## Getting started with 3Dpass Node

### Download the latest release
```sh
wget https://github.com/3Dpass/3DP/releases/download/v25/poscan-consensus-linux-x86_64.tar.gz
tar xzf poscan-consensus-linux-x86_64.tar.gz
```

### Rust Setup
If you need to build the Node on your own you have to set up the environment.
First, complete the [basic Rust setup instructions](https://github.com/substrate-developer-hub/substrate-node-template/blob/main/docs/rust-setup.md). You can also use this command to clone 3DP folder and set up Rust:

```sh
cd ~
git clone https://github.com/3Dpass/3DP.git
cd 3DP
curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain nightly-2023-05-20
source $HOME/.cargo/env
rustup target add wasm32-unknown-unknown --toolchain nightly-2023-05-20
sudo apt-get install -y libclang-dev libssl-dev clang
```

### Run a temporary node

The provided `cargo run` command will launch a temporary node and its state will be discarded after
you terminate the process. After the project has been built, there are other ways to launch the
node.

```sh
cargo run --release -- --dev --tmp
```

### Build

The `cargo run` command will perform an initial build. Use the following command to build the node
without launching it:

```sh
cargo build --release
```

### Embedded Docs

Once the project has been built, the following command can be used to explore all parameters and
subcommands:

```sh
./target/release/poscan-consensus -h
```

### Set up your keys

#### New account
- Generate new account and import all of your keys (Mining key, GRANDPA key and ImOnline key) at once with the script `keygen.sh`:
```sh
sh keygen.sh
```
The keys will be imported into `~/3dp-chain/chains/3dpass/keystore`

#### Existing account
- Have you already had an account, use the `keygen_seed.sh` script to generate the keys out of your Secret Seed phrase and import them all at once.

1. Put your Secret Seed phrase into the `~/3DP/keygen_seed.sh` file like this:
```bash
bash#! /bin/bash
# A keyset will be generated out of the seed phrase below
MEMO_SEED="PUT YOUR MEMO SEED HERE"
```
2. Save the the `keygen_seed.sh` and execute the script:
```sh
sh keygen_seed.sh
```
The keys will be imported into `~/3dp-chain/chains/3dpass/keystore`

#### Manual set up
- Follow [manual set up guidelines](https://3dpass.org/mainnet#manual), if you'd like to set up your keys manually. 
Learn more about [addresses and keys](https://3dpass.org/mainnet#addresses).

### Run the Node
Start the Node with the following command:
```bash
./target/release/poscan-consensus --base-path ~/3dp-chain/ --chain mainnetSpecRaw.json --name MyNodeName --validator --telemetry-url "wss://submit.telemetry.3dpscan.io/submit 0" --author <your mining address or pub key> --threads 2 --no-mdns
```
### Run miner
1. Install [Bun](https://bun.sh/)
2. Install and run miner:
```bash
bun install
bun miner.js --host 127.0.0.1 --port 9933
```

Make sure you can see your node in the [list](https://telemetry.3dpscan.io/). Use this [tutorial](https://3dpass.org/mainnet#linux-mac) for more details.

## Node and Mining with Docker
This procedure will build and run the Node and Miner automatically with Docker (Mac, Linux, or Windows).

First, install [Docker](https://docs.docker.com/get-docker/) and
[Docker Compose](https://docs.docker.com/compose/install/).

Run the following command:

```shell
cd ~
git clone https://github.com/3Dpass/3DP.git
cd 3DP
cp docker-compose.override.yml.example docker-compose.override.yml
// TODO: put your `MEMO_SEED` and `ADDRESS` in `docker-compose.override.yml`
docker compose build
docker compose up
```
`docker-compose.override.yml` example:

```shell
version: "3.9"

  services:
      node:
        environment:
          - MEMO_SEED=PLACE MEMO SEED HERE
          - ADDRESS=PLACE MINER ADDRESS HERE

```
You can generate your ADDRESS and MEMO_SEED phrase in the [wallet](https://wallet.3dpass.org/). 
Follow this [tutorial](https://3dpass.org/mainnet#docker) for more details.


## Connect to the wallet Front-end
Open the wallet page: https://wallet.3dpass.org/. In order to connect your Node to the wallet in local you need to set up your local API endpoint as `ws://127.0.0.1:9944` in the Settings.
Follow this [guidelines](https://3dpass.org/mainnet#wallet) for more details.

## Development

### Single-Node Development Chain

This command will start the single-node development chain with persistent state:

```bash
./target/release/poscan-consensus --dev
```

Purge the development chain's state:

```bash
./target/release/poscan-consensus purge-chain --dev
```

Start the development chain with detailed logging:

```bash
RUST_BACKTRACE=1 ./target/release/poscan-consensus -ldebug --dev
```

### Multi-Node Development Chain

Clear keystore for Alice and Bob:

```bash
rm -R /tmp/alice/ /tmp/bob/
```
Import mining key for Alice into the keystore:
```bash
target/release/poscan-consensus import-mining-key //Alice --base-path /tmp/alice
```
Run the first Node with the Alice's pub key:
```bash
target/release/poscan-consensus --base-path /tmp/alice --chain local --alice --port 30333 --ws-port 9944 --rpc-port 9933 --unsafe-rpc-external --node-key 0000000000000000000000000000000000000000000000000000000000000001 --validator -lposcan=debug --author 0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d
```
Run the second Node:
```bash
target/release/poscan-consensus --base-path /tmp/bob --chain local --bob --port 30334 --ws-port 9945 --rpc-port 9934  --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp --validator
```

Copyright (C) 2002-2023 3Dpass https://3dpass.org/
