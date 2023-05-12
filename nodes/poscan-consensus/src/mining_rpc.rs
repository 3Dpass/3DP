use std::sync::Arc;
use jsonrpsee::{
	core::{Error as JsonRpseeError, RpcResult},
	proc_macros::rpc,
	types::error::{CallError, ErrorCode, ErrorObject},
};

use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use sp_api::ProvideRuntimeApi;
use sp_runtime::generic::DigestItem;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Header;
use poscan_algo::decompress_obj;

extern crate alloc;

use alloc::string::String;
use crate::service::{MiningProposal, DEQUE};
use sp_consensus_poscan::MAX_MINING_OBJ_LEN;

const MAX_QUEUE_LEN: usize = 20;

const RES_OK: u64 = 0;
const RES_QUEUE_FULL: u64 = 1;
const RES_OBJ_MAX_LEN: u64 = 2;

// #[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo)]
// #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
// struct WorkParams {
// 	difficulty: Difficulty,
// 	pre_hash: Hash,
// 	parent_hash: Hash,
// };

#[rpc(client, server)]
pub trait PoscanMiningRpcApi<BlockHash> {
	#[method(name = "poscan_pushMiningObject")]
	fn push(&self, obj_id: u64, obj: String) -> RpcResult<u64>;

	#[method(name = "poscan_getMiningObject")]
	fn get_obj(&self, at: BlockHash) -> RpcResult<String>;
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

impl<C, Block> PoscanMiningRpcApiServer<<Block as BlockT>::Hash> for MiningRpc<C, Block>
	where
		Block: BlockT,
		C: Send + Sync + 'static,
		C: ProvideRuntimeApi<Block>,
		C: ProvideRuntimeApi<Block> + Send + Sync + HeaderBackend<Block>,
{
	fn push(&self, _obj_id: u64, obj: String) -> RpcResult<u64> {
		let mut lock = DEQUE.lock();
		if lock.len() >= MAX_QUEUE_LEN {
			return Ok(RES_QUEUE_FULL);
		}
		if obj.len() > MAX_MINING_OBJ_LEN {
			return Ok(RES_OBJ_MAX_LEN);
		}
		(*lock).push_back(MiningProposal { id: 1, pre_obj: obj.as_bytes().to_vec() });

		Ok(RES_OK)
	}

	fn get_obj(&self, at: <Block as BlockT>::Hash) -> RpcResult<String> {
		let block_id = BlockId::Hash(at.into());
		let h = self.client.header(block_id)
			.map_err(|_e|
		        JsonRpseeError::Call(CallError::Custom(ErrorObject::owned(
					ErrorCode::InvalidParams.code(),
					format!("Unknown header: {}", at.to_string()),
					None::<()>,
				)))
			)?;

		let h = match h {
			Some(h) => h,
			None => return
				Err(JsonRpseeError::Call(CallError::Custom(ErrorObject::owned(
					ErrorCode::ServerError(1).code(),
					format!("Empty header"),
					None::<()>,
				)))),
		};

		let di = match h.digest().logs.last() {
			Some(di) => di,
			None => return
				Err(JsonRpseeError::Call(CallError::Custom(ErrorObject::owned(
					ErrorCode::ServerError(2).code(),
					format!("Empty digest log"),
					None::<()>,
				)))),
		};

		if let DigestItem::Other(obj) = di {
			let mut obj = obj.to_vec();
			if obj[..4] == vec![b'l', b'z', b's', b's'] {
				obj = decompress_obj(&obj[4..]);
			}
			let s = String::from_utf8(obj)
				.map_err(|_e|
					 JsonRpseeError::Call(CallError::Custom(ErrorObject::owned(
						 ErrorCode::ServerError(3).code(),
						 format!("Can't convert object to utf8"),
						 None::<()>,
				))))?;
			Ok(s)
		}
		else {
			Err(JsonRpseeError::Call(CallError::Custom(ErrorObject::owned(
				ErrorCode::ServerError(4).code(),
				format!("Invalid digest type"),
				None::<()>,
			))))
		}
	}
}
