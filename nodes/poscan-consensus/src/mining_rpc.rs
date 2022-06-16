use std::sync::Arc;
use jsonrpc_derive::rpc;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use sp_api::ProvideRuntimeApi;
use sp_consensus_poscan::PoscanApi;

extern crate alloc;
use alloc::string::String;
use crate::service::{MiningProposal, DEQUE};
use sp_consensus_poscan::MAX_MINING_OBJ_LEN;

const MAX_QUEUE_LEN: usize = 5;

const RES_OK: u64 = 0;
const RES_QUEUE_FULL: u64 = 1;
const RES_OBJ_MAX_LEN: u64 = 2;

#[rpc]
pub trait PoscanMiningRpc<BlockHash> {
	#[rpc(name = "push_mining_object")]
	fn push(&self, obj_id: u64, obj: String) -> Result<u64>;

	#[rpc(name = "get_mining_object")]
	fn get_obj(&self, at: BlockHash) -> Result<Vec<u8>>;
}

/// A struct that implements the `SillyRpc`
pub struct MiningRpc<C, Block> {
	client: Arc<C>,
	_marker: std::marker::PhantomData<Block>,

}
impl<C, Block> MiningRpc<C, Block> {
	pub fn new(client: Arc<C>) -> Self {
		Self {
			client,
			_marker: Default::default(),
		}
	}
}

impl<C, Block> PoscanMiningRpc<<Block as BlockT>::Hash> for MiningRpc<C, Block>
	where
		Block: BlockT,
		C: Send + Sync + 'static,
		C: ProvideRuntimeApi<Block>,
		// C: HeaderBackend<Block>,
		C::Api: sp_consensus_poscan::PoscanApi<Block>,
{
	fn push(&self, _obj_id: u64, obj: String) -> Result<u64> {
		let mut lock = DEQUE.lock();
		if lock.len() >= MAX_QUEUE_LEN {
			return Ok(RES_QUEUE_FULL)
		}
		if obj.len() > MAX_MINING_OBJ_LEN {
			return Ok(RES_OBJ_MAX_LEN)
		}
		(*lock).push_back(MiningProposal {id: 1, pre_obj: obj.as_bytes().to_vec()});

		Ok(RES_OK)
	}

	fn get_obj(&self, at: <Block as BlockT>::Hash) -> Result<Vec<u8>> {
		let api = self.client.runtime_api();

		let h = BlockId::Hash(at.into());
		let runtime_api_result = api.get_obj(&h);
		runtime_api_result.map_err(|e| RpcError {
			code: ErrorCode::ServerError(9876), // No real reason for this value
			message: "Something wrong".into(),
			data: Some(format!("{:?}", e).into()),
		})
	}
}
