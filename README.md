
<img width="1199" alt="LoT-min" src="https://github.com/user-attachments/assets/36474db1-598e-49ac-9bca-385e0858939b" />

# The Ledger of Things Node

"The Ledger of Things" is a revolutionary open source Layer 1 blockchain platform for the tokenization of objects. [White Paper](https://3dpass.org/coin#white-papper). Current list of the object categories is presented as follows: 3D objects, 2D drawings, Music, Biometrics, Radio signals, Movements, Texts.

[Proof of Scan](https://3dpass.org/proof-of-scan) is a hybrid decentralized protocol "[PoW](https://github.com/3Dpass/3DP/wiki/Proof-of-Scan:-PoW-component-(ASIC%E2%80%90resistant)) `ASIC-resistant, CPU-oriented` + [PoA](https://github.com/3Dpass/3DP/wiki/Proof-of-Scan:-PoA-component) (Proof of Authority)", which is based on recognition technology. Every object, transformed by 3DPass, obtains its own unique and sustainable identity ([HASH ID](https://3dpass.org/features#recognition-hash-id)) the object can be recognized by. This will prevent the copying of digital assets and thus open a door for the entire blockchain space to potentially trillions in deals all over the globe.

[Grid2d](https://3dpass.org/grid2d) is 3D shape recognition algorithm, which is utilized as one of hash functions in the Proof of Scan protocol. The implementations of the algorithm are the [pass3d](https://github.com/3Dpass/pass3d) recognition toolkit and its WASM analog [p3d](https://github.com/3Dpass/p3d).

[3DPRC-2](https://github.com/3Dpass/whitepaper/blob/main/3DPRC-2.md) (3Dpass Request for Comments) is a standard p2p protocol for the tokenization of the User objects operating within “The Ledger of Things”, by which the most useful aspect of the "Proof of Scan" consensus is getting uncovered. 3DPRC-2 provides decentralized [PoScan API](https://github.com/3Dpass/3DP/wiki/3DPRC%E2%80%902-PoScan-API) available for customers.

The scope of potential 3Dpass applications goes way beyond 3D object recognition and not limited to. Being naturally organized and still cultivating this community driven spirit, 3Dpass is here to encourage developers from all around the globe to upgrade the pass3d open source toolkit with new fascinating recognition algorithms and make it even more useful for human civilization. Learn more about the [algorithm requirements](http://localhost:3000/proof-of-scan#object).

[3DPass Coin (P3D)](https://3dpass.org/coin) is a native utility token, operating within "The Ledger of Things" eco-system, which aims to incentivize the efforts of community members maintaining the network infrastructure. Such aspects as: Storage fee, Gas fee, The object authentication fee, Transaction fee, The validator collaterals, Penalties - are all being counted in P3D.

<<<<<<< HEAD
 [AI dev deepWiKi](https://deepwiki.com/3Dpass/3DP) | [Contribution Grant Program](https://3dpass.org/grants)  |  [Contributing guidelines](https://github.com/3Dpass/3DP/blob/main/CONTRIBUTING.md)  |  [Discord](https://discord.gg/u24WkXcwug)
=======
[Contribution Grant Program](https://3dpass.org/grants)  |  [Contributing guidelines](https://github.com/3Dpass/3DP/blob/main/CONTRIBUTING.md)  |  [Discord](https://discord.gg/u24WkXcwug)
>>>>>>> b6e057cb (Update README.md)

## Getting started with 3Dpass Node

### Download the latest release
```sh
wget https://github.com/3Dpass/3DP/releases/download/v28/poscan-consensus-linux-x86_64.tar.gz
tar xzf poscan-consensus-linux-x86_64.tar.gz
```

### Rust Setup
If you need to build the Node on your own you have to set up the environment.
First, complete the [basic Rust setup instructions](https://github.com/substrate-developer-hub/substrate-node-template/blob/main/docs/rust-setup.md). You can also use this command to clone 3DP folder and set up Rust:

```sh
cd ~
git clone https://github.com/3Dpass/3DP.git
cd 3DP
curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain nightly-2023-05-05
source $HOME/.cargo/env
rustup target add wasm32-unknown-unknown --toolchain nightly-2023-05-05
sudo apt-get install -y libclang-dev libssl-dev clang
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
./target/release/poscan-consensus --base-path ~/3dp-chain/ --chain mainnetSpecRaw.json --name MyNodeName --validator --telemetry-url "wss://submit.3dpass.network/submit 0" --author <your mining address or pub key> --threads 2 --no-mdns
```
Make sure you can see your Node on the telemetry server: [https://3dpass.network](https://3dpass.network/). Explore additional details with this [tutorial](https://3dpass.org/mainnet#linux-mac-run).

### Run miner
1. Install [Bun](https://bun.sh/)
2. Install and run miner:
```bash
bun install
bun miner.js --host 127.0.0.1 --port 9933
```

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


## Connect to the web3 wallet Front-end
Open the wallet page: https://wallet.3dpass.org/. In order to connect your Node to the wallet in local you need to set up your local API endpoint as `ws://127.0.0.1:9944` in the Settings.
Follow this [guidelines](https://3dpass.org/mainnet#wallet) for more details.

## EVM and cross-platform options
The Node is equipped with EVM compatibility layer comprised of the [EVM pallet](/pallets/evm) (Solidity code executor) as well as eth blockchain emulator with full client on top of the Substrate based master chain.

### Connect to Metamask and Remix in Local
- Run the Node with the `--rpc-port 9978` flag (any port can be set up)
- Add a custom chain to the Metamask wallet:
   - Name: `3dpass - The Ledger of Things (testnet)`
   - Testnet chain id: `1333`
   - RPC provider: `https://127.0.0.1:9978`
- Open [Remix](https://remix.ethereum.org) and connect it through the Metamask to be able to deploy and run Solidity smart contracts

### EVM accounts
There is a cross-platform mapping in place between the Substrate (H256) and EVM (H160) accounts, which allows for cross
platfom actions - native token transfers, assets methods, etc. E.g. you can transfer tokens from EVM to any Substrate account and vice versa.
And every account must have its `H256` version.

- H256 -> H160 (derived by cutting the tail 20 bytes)
  - e.g. `0xc6006fea43ccce14f9432f8e6b9403113c935cc17160cd438af2e7a02624124c` -> `0xc6006fea43ccce14f9432f8e6b9403113c935cc1`
- H160 (private key) -> H256 address (mapped, **irreversible**)
  - e.g. `0xc6006fea43ccce14f9432f8e6b9403113c935cc1` -> `0xceb75620b9f3bc3a039b8e78efed58fa3c7422d18c97f1fd44abf3f2499d0760` (prefix 72: d7fJyRyHivFfDXsDs14oYgtqCt3scfHgaLT5NxHs5f6e5ycsN)

Use this [converter](https://hoonsubin.github.io/evm-substrate-address-converter/) to create a cross-platform alias for either Substrate or EVM account (testnet address prefix:`72`).

### Custom precompiles
Although the EVM exploits the [standard ethereum precompiles](https://github.com/3Dpass/3DP/tree/test/pallets/evm/precompile), there is a buch of
[custom precompiles](/precompiles/), of which each one serves as a cross-platform `EVM -> Substrate` interface, so that native substrate functions can be called from Solidity.

- Interaction with native token (P3Dt): [balances-erc20](/precompiles/balances-erc20) precompile, Contract address: `0x0000000000000000000000000000000000000802`
- Interaction with Local assets 3DPRC2 - [assets-erc20](https://github.com/3Dpass/3DP/tree/test/precompiles/assets-erc20) precompile,
Contract address format: `0xFBFBFBFA + <assetid in hex>`, where the `assetid` is the asset index in [poscanAssets]() runtime module.
  e.g. You have created an asset with the `assetid 222`. The hex value of `222` is `DE`  . So, the `H160` address to run the contract at is going to be as follows: `0xFBFBFBFA000000000000000000000000000000DE`.

## Assets
The [poScan](/pallets/poscan/) module serves as a mean of users' [objects authenication](https://3dpass.org/features#3dprc-2) within The Ledger of Things.

Additionaly, the Node is equipped with the [poscanAssets](/pallets/poscan-assets/) module, which allows for Real World Objects (RWA) tokenization. Either [3DPRC2](https://3dpass.org/assets#3dprc-2) tokens or conventional [fungible tokens](https://3dpass.org/assets#conventional-assets) and NFTs can be issued.

## Assets conversion (DEX)
The [assetConversion](/pallets/asset-conversion) module is a custom version of a decentralized exchange based on Uniswap v2 protocol rules and integrated into The Ledger of Things runtime. Explore the module [API](https://github.com/3Dpass/3DP/wiki/DEX-module-API) and its [Web UI](https://github.com/3Dpass/3DP/tree/main/pallets/asset-conversion).

## "Ink" smart contracts
The Node supports native Substrate Smart contract trait using [ink](https://use.ink/), a Rust-based embedded domain specific language (eDSL) for writing [WebAssembly](https://webassembly.org/) smart contracts. Learn [how ink can be commpared to Solidity](https://use.ink/ink-vs-solidity/). Follow these [guiudelines](https://3dpass.org/assets#smart-contracts) to run your smart contract on LoT.

## Development mode

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
# Integration

# Integration

- NODE - the Node is based on [Substrate](https://substrate.io/) framework and implemented as two-piece design the `Rust native` part and the `Runtime` component ([WASM](https://webassembly.org/)-based), which is upgradable online and allows for multiple useful modules to operate (see more [forkless upgrade](https://3dpass.org/proof-of-scan#forkless-upgrade)).
- RPC (remote procedure call) - the capabilities that allow blockchain users to interact with the network. The NODE provides HTTP and [WebSocket](https://github.com/3Dpass/3DP/wiki/Set-up-WSS-for-Remote-Connections) RPC servers.
- CORE Networking - the [`libp2p`](https://libp2p.io/) is used as as networking stack for the Nodes to communicate with each other.
- [3Dpass light wallet](https://github.com/3Dpass/wallet) - desktop users and 3D printing labs integration
- [Pass3d mobile](https://github.com/3Dpass/threedpass) - smartphone and tablets users integration

 <img width="719" alt="Node_integration" src="https://github.com/user-attachments/assets/93f186eb-c9db-4fd8-96b7-efc3b451a6b8">


## Responsibility disclaimer
This is an open source free p2p software. Use it at your own risk. 3dpass platform is non-profit and community-supported.

Copyright (C) 2002-2025 3Dpass https://3dpass.org/
