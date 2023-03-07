
[![logo](https://3dpass.org/assets/img/3DPass_on_the_moon.png)](https://3dpass.org)

# 3Dpass Node - mainnet

3DPass is an OpenSource decentralized WEB 3.0 P2P Layer 1 blockchain platform for tokenization of real physical and virtual objects and its transformation into digital assets. The main idea of 3DPass is to make it possible for people to use real world objects in digital within smart-contracts and dApps and have full control over its copies (learn more about 3DPass [features](https://3dpass.org/features.html) ). This allows to establish 1:1 corespondence between an object and its asset and define ownership as an additional property of the asset providing zero-knowledge proof. Follow the [White Paper](https://3dpass.org/3DPass_white_paper.pdf) for the details.

Every object, transformed by 3DPass, has its own unique and sustainable identity called [HASH ID](https://3dpass.org/features.html#3D_object_recognition) the object might be recognized by. The algorithm is flexible enough to adjust the definition of error to certain level of scanning precision.

3DPass NODE is a Layer 1 blockchain based on [Substrate](https://www.substrate.io/) with brandnew consensus [Proof of Scan](https://3dpass.org/proof_of_scan.html) leveraging 3D object shape recognition algorithm called [Grid2d](https://3dpass.org/grid2d.html), which is implemented as [pass3d](https://github.com/3Dpass/pass3d) tool. 3Dpass NODEs are designed to provide object authenticity check. We call it "The Ledger of Unique Things". In order to encourage users to maintain the network and to solve issues there is a cryptocurrency 3DPass Coin.

[Contribution program](https://3dpass.org/distribution.html#contribution)  |  [Contributing guidelines](https://github.com/3Dpass/3DP/blob/main/CONTRIBUTING.md)  |  [Discord](https://discord.gg/u24WkXcwug)

# Integration

 This is an eco-system scheme, which represents general functional elements:

- 3DPass NODE (based on [Substrate](https://substrate.io/)) - wallets, dApps, smart-contracts, IoT devices integration using API and RPC
- [Pass3d](https://github.com/3Dpass/pass3d) and [p3d](https://github.com/3Dpass/p3d) recognition toolkit - recognition algorithms integration
- [Proof of Scan](https://3dpass.org/proof_of_scan.html) consensus - the logic, using 3D objects recognition toolkit, that allows network participants to agree on the state of the blockchain

- [3DPass light wallet](https://github.com/3Dpass/wallet) - desktop users and 3D printing labs integration
- [Pass3d mobile](https://github.com/3Dpass/threedpass) - smartphone and tablets users integration
- Smart contracts toolkit - Substrate based smart contract tools using [ink](https://paritytech.github.io/ink-docs/), a Rust-based embedded domain specific language (eDSL) for writing WebAssembly smart contracts. Learn more about [how it compares to Solidity](https://paritytech.github.io/ink-docs/ink-vs-solidity). As well, it allows unmodified EVM code to be executed in the 3DPass blockchain. Some special Substrate fetures are designed to closely emulate the functionality of executing contracts on the Ethereum mainnet within the 3DPass network.
- IPFS storage - embedded decentralized storage for assets
- RPC (remote procedure call) - the capabilities that allow blockchain users to interact with the network. The NODE provides HTTP and [WebSocket](https://github.com/3Dpass/3DP/wiki/Set-up-WSS-for-Remote-Connections) RPC servers.
- Networking: we use the [`libp2p`](https://libp2p.io/) networking stack to allow for the
  nodes in the network to communicate with one another.

[![Architecture](https://3dpass.org/assets/img/eco_system.png)](https://3dpass.org/features.html#integration)

## Getting started with 3DPass Node

### Download the latest release
```sh
wget https://github.com/3Dpass/3DP/releases/download/v0.0.6/poscan-consensus-x86_64-unknown-linux-gnu.tar.gz
tar xzf poscan-consensus-x86_64-unknown-linux-gnu.tar.gz
```

### Rust Setup
If you need to build the Node on your own you have to set up the environment.
First, complete the [basic Rust setup instructions](https://github.com/substrate-developer-hub/substrate-node-template/blob/main/docs/rust-setup.md). You can also use this command to clone 3DP folder and set up Rust:

```sh
cd ~
git clone https://github.com/3Dpass/3DP.git
cd 3DP
curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain nightly
source $HOME/.cargo/env
rustup update nightly
rustup target add wasm32-unknown-unknown --toolchain nightly
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

Generate youur mining address and keys:
```bash
./target/release/poscan-consensus generate-mining-key --base-path ~/3dp-chain/ --chain mainnetSpecRaw.json
```

Register your mining key in the keystore:
```bash
./target/release/poscan-consensus import-mining-key 'your secret phrase' --base-path ~/3dp-chain/ --chain mainnetSpecRaw.json
```

Generate your GRANDPA keys for finalization. Use the same secret phrase as it's used for mining address (The account is defined by the secret phrase):
```bash
./target/release/poscan-consensus import-mining-key 'your secret phrase' --base-path ~/3dp-chain/ --chain mainnetSpecRaw.json
## Development
```

Insert Grandpa key into the keystore:
```bash
./target/release/poscan-consensus key insert --base-path ~/3dp-chain/ --chain mainnetSpecRaw.json --scheme Ed25519 --suri <secret seed from Grandpa> --key-type gran
```
Make sure you have both keys in the keystore `~/3dp-chain/chains/3dpass/keystore`


Start the Node with the following command:
```bash
./target/release/poscan-consensus --base-path ~/3dp-chain/ --chain mainnetSpecRaw.json --name MyNodeName --validator --telemetry-url "wss://submit.telemetry.3dpass.org/submit 0" --author <your mining address or pub key> --threads 2 --no-mdns
```

Install miner (You have to install [NodeJS v16](https://nodejs.org/en/) and [pnpm](https://pnpm.io/installation) `corepack enable` before):
```bash
pnpm i
```

Run miner:
```bash
pnpm miner --interval 100
```
- `--interval 100` is the amount of time in miliseconds between the last and the next one objects being sent towards the Node. Dependidng on how much threads are you mining with, reduce the interval until you reach desired proc load.

Make sure you can see your node in the [list](https://telemetry.3dpass.org/). Use this [tutorial](https://3dpass.org/mainnet.html#mining_docker) for more details.

## Mining with Docker
This procedure will build and run the Node and Miner automatically with Docker.

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
          - MEMO_SEED=[PLACE MEMO SEED HERE]
          - ADDRESS=[PLACE MINER ADDRESS HERE]
          - THREADS=2
          - INTERVAL=100
```
- `THREADS=2` is the amount of threads you are about to use for mining
- `INTERVAL=100` is the amount of time in miliseconds between the last and the next one objects being sent towards the Node. Dependidng on how much threads are you mining with, reduce the interval until you reach desired proc load.

You can generate your ADDRESS and MEMO_SEED phrase in the [wallet](https://wallet.3dpass.org/) (add new address). Make sure you can see your node in the [list](https://telemetry.3dpass.org/). Use this [tutorial](https://3dpass.org/mainnet.html#mining_docker) for more details.


## Connect to the wallet Front-end
Open the wallet page: https://wallet.3dpass.org/. In order to connect your Node to the wallet in local you need to set up your local API endpoint as `ws://127.0.0.1:9944` in the Settings.
Follow this [guidelines](https://3dpass.org/mainnet.html#how_to_use_web3_wallet) for more details.

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

Copyright (C) 2022 3DPass https://3dpass.org/
