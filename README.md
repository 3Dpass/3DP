
[![logo](https://3dpass.org/assets/img/3DPass_on_the_moon.png)](https://3dpass.org)

# 3Dpass Node - mainnet

3DPass is an OpenSource decentralized WEB 3.0 P2P platform for tokenization of real physical and virtual objects and its transformation into digital assets. The main idea of 3DPass is to make it possible for people to use real world objects in digital within smart-contracts and deals and to take all advantages from that (learn more about 3DPass [features](https://3dpass.org/features.html) ). Follow the [White Paper](https://3dpass.org/3DPass_white_paper.pdf) for the details.

Every object, transformed by 3DPass, has its own unique and stable identity called [HASH ID](https://github.com/3Dpass/3DP/wiki/HASH-ID-vs-NFT-difference) the object might be recognized by. In order to encourage users to maintain the network and to solve issues there is a cryptocurrency 3DP Coin.

3D Pass NODE is a Layer 1 blockchain based on [Substrate](https://www.substrate.io/) with brandnew consensus [Proof of Scan](https://3dpass.org/proof_of_scan.html) using 3D object shape recognition algorithm called [Grid2d](https://3dpass.org/grid2d.html), which is implemented into the Node with the recognition tool [pass3d](https://github.com/3Dpass/pass3d). [Proof of Scan](https://3dpass.org/proof_of_scan.html) is a kind of non-conventional PoW consensus because of the computing power is used for 3D shape recognition. 3Dpass NODEs are designed to provide objects authenticity check. You might call it The Ledger of unique things, which allows to utilize them within smart-contracts and dApps. The network nodes will prevent assets from copying.

---How to contribute---

In order to contribute on solving the global digital transformation challenge feel free to implement or develop new recognition algorithms into pass3d toolkit (e.x.face recognition, fingerprint recognition, radio signal, 2D drawings, melody, voice, 3D objects, etc.) or make your suggessions about. It always encourages that developers contribute to 3DPass growth, would it be miner tools, network features, dApps, smart-contracts, impementations, new projects and ideas etc. Dive down into [3DPass contribution rewards program](https://3dpass.org/distribution.html#contribution)

Join 3Dass community:
- [Discord chat](https://discord.gg/u24WkXcwug)
- [Bitcointalk](https://bitcointalk.org/index.php?topic=5382009.0)
- [GutHub discussion](https://github.com/3Dpass/3DP/discussions/4)

---How to contribute---

# Integration

 This is an eco-system scheme, which represents general functional elements:

- 3DPass NODE (based on [Substrate](https://substrate.io/)) - wallets, dApps, smart-contracts, IoT devices integration using API and RPC
- [Pass3d](https://github.com/3Dpass/pass3d) and [p3d](https://github.com/3Dpass/p3d) recognition toolkit - recognition algorithms integration

This toolkit consists of stable recognition algorithms used for identification of things (3D objects and others, learn more >> ). Since the recognition technology is what the digital transformation process of anything is beginning from, and the result of the processing would always be its HASH ID, it implies every application, integrated into 3DPass eco-system, to have Pass3d or p3d toolkit implemented.

- [Proof of Scan](https://3dpass.org/proof_of_scan.html) consensus - the logic, using 3D objects recognition toolkit, that allows network participants to agree on the state of the blockchain

- 3DPass light wallet - desktop users and 3D printing labs integration
- [Pass3d mobile](https://github.com/3Dpass/threedpass) - smartphone and tablets users integration
- Smart contracts toolkit - Substrate based smart contract tools using [ink](https://paritytech.github.io/ink-docs/), a Rust-based embedded domain specific language (eDSL) for writing WebAssembly smart contracts. Learn more about [how it compares to Solidity](https://paritytech.github.io/ink-docs/ink-vs-solidity). As well, it allows unmodified EVM code to be executed in the 3DPass blockchain. Some special Substrate fetures are designed to closely emulate the functionality of executing contracts on the Ethereum mainnet within the 3DPass network.
- IPFS storage - embedded decentralized storage for assets
- RPC (remote procedure call) - the capabilities that allow blockchain users to interact with the network. The NODE provides HTTP and WebSocket RPC servers. [How to set up websocket](https://github.com/3Dpass/3DP/wiki/Set-up-WSS-for-Remote-Connections)
- Networking: we use the [`libp2p`](https://libp2p.io/) networking stack to allow the
  nodes in the network to communicate with one another.

[![Architecture](https://3dpass.org/assets/img/eco_system.png)](https://3dpass.org/features.html#integration)


## Getting Started

Follow the steps below to get started with the 3DPass Node:

### Rust Setup

First, complete the [basic Rust setup instructions](https://github.com/substrate-developer-hub/substrate-node-template/blob/main/docs/rust-setup.md).

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

## Mining with Docker
This procedure will build the Node and Miner automatically with Docker. 

First, install [Docker](https://docs.docker.com/get-docker/) and
[Docker Compose](https://docs.docker.com/compose/install/).

Run the following command:

```shell
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

## Mining: manual set up

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
Install miner (You have to install [NodeJS v16](https://nodejs.org/en/) and [Yarn](https://classic.yarnpkg.com/lang/en/docs/install/) before):
```bash
yarn
```
Run miner:
```bash
yarn miner --interval 100
```
- `--interval 100` is the amount of time in miliseconds between the last and the next one objects being sent towards the Node. Dependidng on how much threads are you mining with, reduce the interval until you reach desired proc load. 

Make sure you can see your node in the [list](https://telemetry.3dpass.org/). Use this [tutorial](https://3dpass.org/mainnet.html#mining_docker) for more details.

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

