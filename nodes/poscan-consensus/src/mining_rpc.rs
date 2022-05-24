use jsonrpc_core::Result;
use jsonrpc_derive::rpc;
extern crate alloc;
use alloc::string::String;
use crate::service::{MiningProposal, DEQUE};

const MAX_QUEUE_LEN: usize = 5;

const RES_OK: u64 = 0;
const RES_QUEUE_FULL: u64 = 1;

#[rpc]
pub trait PoscanMiningRpc {
	#[rpc(name = "push_mining_object")]
	fn push(&self, obj_id: u64, obj: String) -> Result<u64>;
}

/// A struct that implements the `SillyRpc`
pub struct MiningRpc;

impl PoscanMiningRpc for MiningRpc {
	fn push(&self, _obj_id: u64, obj: String) -> Result<u64> {
		let mut lock = DEQUE.lock();
		if lock.len() >= MAX_QUEUE_LEN {
			return Ok(RES_QUEUE_FULL)
		}
		(*lock).push_back(MiningProposal {id: 1, pre_obj: obj.as_bytes().to_vec()});

		Ok(RES_OK)
	}
}
