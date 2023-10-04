use std::sync::Arc;

use jsonrpsee::{
	core::{Error as JsonRpseeError, RpcResult},
	proc_macros::rpc,
	types::error::{CallError, ErrorCode, ErrorObject},
};

use sp_runtime::{
	generic::BlockId,
	traits::Block as BlockT,
};
use runtime::{AccountId, BlockNumber};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_consensus_poscan::{PoscanApi, ObjIdx, ObjData};

const RES_NOT_FOUND: i32 = 1;


#[rpc(client, server)]
pub trait PoscanRpcApi<BlockHash> {

	#[method(name = "poscan_getPoscanObject")]
	fn get_poscan_object(
		&self,
		obj_idx: ObjIdx,
	) -> RpcResult<Option<ObjData<AccountId, BlockNumber>>>;
}

pub struct PoscanRpc<C, Block>
where
	Block: BlockT,
{
	client:   Arc<C>,
	_marker:  std::marker::PhantomData<Block>,
}

impl<C, Block> PoscanRpc<C, Block>
where
	Block: BlockT,
{
	pub fn new(client: Arc<C>) -> Self {
		Self {
			client,
			_marker: Default::default(),
		}
	}
}

impl<C, Block> PoscanRpcApiServer<<Block as BlockT>::Hash> for PoscanRpc<C, Block>
	where
		Block: BlockT,
		C: Send + Sync + 'static,
		C::Api: PoscanApi<Block, AccountId, BlockNumber>,
		C: ProvideRuntimeApi<Block> + Send + Sync + HeaderBackend<Block>,
{
	fn get_poscan_object(
		&self,
		obj_idx: ObjIdx,
	) -> RpcResult<Option<ObjData<AccountId, BlockNumber>>> {
		let block_id = BlockId::Hash(self.client.info().best_hash);
		let resp = self.client.runtime_api().get_poscan_object(&block_id, obj_idx);
		match resp {
			Ok(maybe_obj) => Ok(maybe_obj),
			Err(e) => {
				Err(JsonRpseeError::Call(CallError::Custom(ErrorObject::owned(
					ErrorCode::ServerError(RES_NOT_FOUND).code(),
					format!("Error: {}", e),
					None::<()>,
				))))
			},
		}
	}
}
