use log::*;
use jsonrpc_core::Result;
use jsonrpc_derive::rpc;
use sp_std::vec::Vec;
// use sp_core::Bytes;
extern crate alloc;
use alloc::string::String;
use crate::service::{MiningProposal, DEQUE};

#[rpc]
pub trait PoscanMiningRpc {
	#[rpc(name = "push_mining_object")]
	fn push(&self, obj_id: u64, obj: String) -> Result<u64>;
}

/// A struct that implements the `SillyRpc`
pub struct MiningRpc;

impl PoscanMiningRpc for MiningRpc {
	fn push(&self, obj_id: u64, obj: String) -> Result<u64> {
		info!(">>> push_minig_obj recieved object obj_id={}", obj_id);
		let mut lock = DEQUE.lock();
		(*lock).push_back(MiningProposal {id: 1, pre_obj: obj.as_bytes().to_vec()});

		Ok(0)
	}
}
