use std::sync::Arc;
use jsonrpc_derive::rpc;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use sp_api::ProvideRuntimeApi;
use sp_runtime::generic::DigestItem;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Header;
use sp_consensus_poscan::decompress_obj;

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
	#[rpc(name = "poscan_pushMiningObject")]
	fn push(&self, obj_id: u64, obj: String) -> Result<u64>;

	#[rpc(name = "poscan_getMiningObject")]
	fn get_obj(&self, at: BlockHash) -> Result<String>;
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
		C: ProvideRuntimeApi<Block> + Send + Sync + HeaderBackend<Block>,
{
	fn push(&self, _obj_id: u64, obj: String) -> Result<u64> {
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

	fn get_obj(&self, at: <Block as BlockT>::Hash) -> Result<String> {
		let block_id = BlockId::Hash(at.into());
		let h = self.client.header(block_id)
			.map_err(|_e| RpcError {
				code: ErrorCode::ServerError(1),
				message: "Unknown header".to_string(), data: None,
			})?;

		let h = match h {
			Some(h) => h,
			None => return Err(RpcError {
				code: ErrorCode::ServerError(2),
				message: "Empty header".to_string(),
				data: None,
			}),
		};

		let di = match h.digest().logs.last() {
			Some(di) => di,
			None => { return Err(jsonrpc_core::Error {
					code: ErrorCode::ServerError(3),
					message: "Empty header".to_string(),
					data: None,
				})
			},
		};

		if let DigestItem::Other(obj) = di {
			let mut obj = obj.to_vec();
			if obj[..4] == vec![b'l', b'z', b's', b's'] {
				obj = decompress_obj(&obj[4..]);
			}
			let s = String::from_utf8(obj)
				.map_err(|_e| RpcError {
					code: ErrorCode::ServerError(3),
					message: "Can't convert object to utf8".to_string(), data: None,
				})?;
			Ok(s)
		}
		else {
			Err(RpcError {
				code: ErrorCode::ServerError(4),
				message: "Invalid digest type".to_string(),
				data: None,
			})
		}
	}
}
