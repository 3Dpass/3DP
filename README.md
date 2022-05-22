
[![logo](https://3dpass.org/assets/img/3DPass_on_the_moon.png)](https://3dpass.org) 

# 3Dpass Node

3DPass is an OpenSource decentralized WEB 3.0 P2P platform for tokenization of real physical and virtual things and its transformation into digital assets. The main idea of 3DPass is to make it possible for people to use real world objects in digital within smart-contracts and deals and to take all advantages from that (learn more about 3DPass [features](https://3dpass.org/features.html) ). Follow the [White Paper](https://3dpass.org/3DPass_white_paper.pdf) for the details.

Every object, transformed by 3DPass, has its own unique and stable identity called [HASH ID](https://github.com/3Dpass/3DP/wiki/HASH-ID-vs-NFT-difference) the object might be recognized by. In order to encourage users to maintain the network and to solve issues there is a cryptocurrency 3DP Coin which is also required for transactions related to user's assets. 

3D Pass NODE is one layer blockchain based on [Substrate](https://www.substrate.io/) with brandnew consensus [Proof of Scan](https://3dpass.org/proof_of_scan.html) using 3D object shape recognition algorithm called [Grid2d](https://3dpass.org/grid2d.html), which is implemented into the Node with the recognition tool [pass3d](https://github.com/3Dpass/pass3d). [Proof of Scan](https://3dpass.org/proof_of_scan.html) is a kind of non-conventional PoW consensus because of the computing power is used for 3D shape recognition. 3Dpass NODEs are designed to provide objects authenticity check. You might call it The Ledger of unique things, which allows to utilize them within smart-contracts and dApps. The network nodes will stand guard preventing assets from copy making. 

---How to contribute---

In order to contribute on solving the global digital transformation challenge feel free to implement or develop new recognition algorithms into pass3d toolkt (e.x.face recognition, fingerprint recognition, radio signal, 2D drawings, melody, voice, 3D objects, etc.) or make your suggessions about. It always encourages that developers contribute to 3DPass growth, would it be miner tools, network features, dApps, smart-contracts, impementations, new projects and ideas etc. Dive down into [3DPass contribution rewards program](https://3dpass.org/distribution.html#contribution)

Join 3Dass community: 
- [Discord chat](https://discord.gg/u24WkXcwug)
- [Bitcointalk](https://bitcointalk.org/index.php?topic=5382009.0)
- [GutHub discussion](https://github.com/3Dpass/3DP/discussions/4)

---How to contribute---

# Integration

 Given the fact that 3DPass is an open source and non-profit project, meaning anyone can add to development, there is an eco-system scheme (exposed down below) representing general functional elements: 

- 3DPass NODE (based on [Substrate](https://substrate.io/)) - wallets, dApps, smart-contracts, IoT devices integration using API and RPC
- [Pass3d](https://github.com/3Dpass/pass3d) and [p3d](https://github.com/3Dpass/p3d) recognition toolkit - recognition algorithms integration

This toolkit consists of stable recognition algorithms used for identification of things (3D objects and others, learn more >> ). Since the recognition technology is what the digital transformation process of anything is beginning from, and the result of the processing would always be its HASH ID, it implies every application, integrated into 3DPass eco-system, to have Pass3d or p3d toolkit implemented. 

- [Proof of Scan](https://3dpass.org/proof_of_scan.html) consensus - the logic, using 3D objects recognition toolkit, that allows network participants to agree on the state of the blockchain

- 3DPass light wallet - desktop users and 3D printing labs integration
- [Pass3d mobile](https://github.com/3Dpass/threedpass) - smartphone and tablets users integration
- Smart contracts toolkit - Substrate based smart contract tools using [ink](https://paritytech.github.io/ink-docs/), a Rust-based embedded domain specific language (eDSL) for writing WebAssembly smart contracts. Learn more about [how it compares to Solidity](https://paritytech.github.io/ink-docs/ink-vs-solidity). As well, it allows unmodified EVM code to be executed in the 3DPass blockchain. Some special Substrate fetures are designed to closely emulate the functionality of executing contracts on the Ethereum mainnet within the 3DPass network. 
- IPFS storage - embedded decentralized storage for assets 
- RPC (remote procedure call) - the capabilities that allow blockchain users to interact with the network. The NODE provides HTTP and WebSocket RPC servers
- Networking: we use the [`libp2p`](https://libp2p.io/) networking stack to allow the
  nodes in the network to communicate with one another.

[![Architecture](https://3dpass.org/assets/img/eco_system.png)](https://3dpass.org/features.html#integration) 


## Getting Started

Follow the steps below to get started with the 3DPass Node Template:

### Using Nix

Install [nix](https://nixos.org/) and optionally [direnv](https://github.com/direnv/direnv) and
[lorri](https://github.com/target/lorri) for a fully plug and play experience for setting up the
development environment. To get all the correct dependencies activate direnv `direnv allow` and
lorri `lorri shell`.

### Rust Setup

First, complete the [basic Rust setup instructions](./docs/rust-setup.md).

### Run

Use Rust's native `cargo` command to build and launch the template node:

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
./target/release/node-template -h
```

## Run

The provided `cargo run` command will launch a temporary node and its state will be discarded after
you terminate the process. After the project has been built, there are other ways to launch the
node.

### Single-Node Development Chain

This command will start the single-node development chain with persistent state:

```bash
./target/release/node-template --dev
```

Purge the development chain's state:

```bash
./target/release/node-template purge-chain --dev
```

Start the development chain with detailed logging:

```bash
RUST_BACKTRACE=1 ./target/release/node-template -ldebug --dev
```

### Multi-Node Local Testnet

If you want to see the multi-node consensus algorithm in action, refer to our
[Start a Private Network tutorial](https://docs.substrate.io/tutorials/v3/private-network).


### Specification

There are several files in the `node` directory - take special note of the following:

- [`chain_spec.rs`](./node/src/chain_spec.rs): A
  [chain specification](https://docs.substrate.io/v3/runtime/chain-specs) is a
  source code file that defines a chain's initial (genesis) state. Chain specifications
  are useful for development and testing, and critical when architecting the launch of a
  production chain. Take note of the `development_config` and `testnet_genesis` functions, which
  are used to define the genesis state for the local development chain configuration. These
  functions identify some
  [well-known accounts](https://docs.substrate.io/v3/tools/subkey#well-known-keys)
  and use them to configure the blockchain's initial state.
- [`service.rs`](./node/src/service.rs): This file defines the node implementation. Take note of
  the libraries that this file imports and the names of the functions it invokes. In particular,
  there are references to consensus-related topics, such as the
  [longest chain rule](https://docs.substrate.io/v3/advanced/consensus#longest-chain-rule),
  the [Proof of Scan consensus](https://3dpass.org/proof_of_scan.html) and the
  [GRANDPA](https://docs.substrate.io/v3/advanced/consensus#grandpa) finality
  gadget.

After the node has been [built](#build), refer to the embedded documentation to learn more about the
capabilities and configuration parameters that it exposes:

```shell
./target/release/node-template --help
```

### Run the Testnet in Docker

First, install [Docker](https://docs.docker.com/get-docker/) and
[Docker Compose](https://docs.docker.com/compose/install/).

Then run the following command to start two local nodes and a [miner](https://github.com/3Dpass/miner) in development chain:

```bash
git clone https://github.com/3Dpass/3DP
cd 3DP
git checkout dev_recipes
docker compose up --build
```
Learn more about [how does it work](https://3dpass.org/testnet.html).
