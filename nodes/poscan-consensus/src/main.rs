//! Hybrid Consensus CLI library.

#![warn(missing_docs)]
#![warn(unused_extern_crates)]

mod chain_spec;
#[macro_use]
pub(crate) mod service;
#[macro_use]
extern crate lazy_static;
mod cli;
mod command;
mod rpc;
mod mining_rpc;
mod poscan_rpc;
mod pool_rpc;
mod pool;

fn main() -> sc_cli::Result<()> {
	command::run()
}
