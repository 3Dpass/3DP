//! RPC for TC candidates: generates the set_code proof (state proof for ReferendumInfoFor)
//! so clients can call `TcCandidates::submit_candidacy`.

use std::sync::Arc;

use codec::Encode;
use jsonrpsee::{
	core::{Error as JsonRpseeError, RpcResult},
	proc_macros::rpc,
	types::error::{CallError, ErrorCode, ErrorObject},
};
use sc_client_api::{Backend, ProofProvider, StorageProvider};
use sp_api::HeaderT;
use sp_blockchain::HeaderBackend;
use sp_runtime::{
	generic::BlockId,
	traits::Block as BlockT,
};

/// ReferendumInfoFor storage key: twox_128(pallet) ++ twox_128("ReferendumInfoFor") ++ blake2_128_concat(index).
fn referendum_info_storage_key(referendum_index: u32) -> Vec<u8> {
	let pallet_hash = sp_io::hashing::twox_128(b"Referenda");
	let storage_hash = sp_io::hashing::twox_128(b"ReferendumInfoFor");
	let index_encoded = referendum_index.encode();
	let index_hash = sp_io::hashing::blake2_128(&index_encoded);
	let mut key = Vec::with_capacity(
		pallet_hash.len() + storage_hash.len() + index_hash.len() + index_encoded.len(),
	);
	key.extend_from_slice(&pallet_hash);
	key.extend_from_slice(&storage_hash);
	key.extend_from_slice(&index_hash);
	key.extend_from_slice(&index_encoded);
	key
}

#[rpc(client, server)]
pub trait TcCandidatesRpcApi<BlockHash> {
	/// Return the storage key for ReferendumInfoFor(referendum_index).
	/// Use with standard `state_getReadProof([key], block_hash)` to get the proof.
	#[method(name = "tc_candidates_getReferendumInfoStorageKey")]
	fn get_referendum_info_storage_key(&self, referendum_index: u32) -> RpcResult<Vec<u8>>;

	/// Return state root and set_code_proof for the given block and referendum index.
	/// The proof can be passed to `TcCandidates::submit_candidacy(state_root, referendum_index, proposal_hash, set_code_proof)`.
	#[method(name = "tc_candidates_getSetCodeProof")]
	fn get_set_code_proof(
		&self,
		block_hash: BlockHash,
		referendum_index: u32,
	) -> RpcResult<TcCandidatesProof>;
}

/// State root and encoded StorageProof for submit_candidacy.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TcCandidatesProof {
	/// State root of the block (for proof verification).
	pub state_root: Vec<u8>,
	/// Encoded StorageProof (SCALE-encoded) to pass as set_code_proof.
	pub set_code_proof: Vec<u8>,
}

pub struct TcCandidatesRpc<C, Block, B> {
	client: Arc<C>,
	_marker: std::marker::PhantomData<(Block, B)>,
}

impl<C, Block, B> TcCandidatesRpc<C, Block, B>
where
	Block: BlockT,
{
	pub fn new(client: Arc<C>) -> Self {
		Self {
			client,
			_marker: std::marker::PhantomData,
		}
	}
}

impl<C, Block, B> TcCandidatesRpcApiServer<<Block as BlockT>::Hash> for TcCandidatesRpc<C, Block, B>
where
	Block: BlockT,
	B: Backend<Block> + 'static,
	C: Send + Sync + 'static + HeaderBackend<Block> + StorageProvider<Block, B> + ProofProvider<Block>,
{
	fn get_referendum_info_storage_key(&self, referendum_index: u32) -> RpcResult<Vec<u8>> {
		Ok(referendum_info_storage_key(referendum_index))
	}

	fn get_set_code_proof(
		&self,
		block_hash: <Block as BlockT>::Hash,
		referendum_index: u32,
	) -> RpcResult<TcCandidatesProof> {
		let block_id = BlockId::Hash(block_hash);
		let header = self
			.client
			.header(block_id)
			.map_err(|e| {
				JsonRpseeError::Call(CallError::Custom(ErrorObject::owned(
					ErrorCode::ServerError(1).code(),
					format!("Failed to get block header: {}", e),
					None::<()>,
				)))
			})?
			.ok_or_else(|| {
				JsonRpseeError::Call(CallError::Custom(ErrorObject::owned(
					ErrorCode::ServerError(2).code(),
					"Block not found",
					None::<()>,
				)))
			})?;
		let state_root = header.state_root();
		let key = referendum_info_storage_key(referendum_index);
		let proof = self.client.read_proof(&block_id, &mut std::iter::once(key.as_slice()));
		let proof = proof.map_err(|e| {
			JsonRpseeError::Call(CallError::Custom(ErrorObject::owned(
				ErrorCode::ServerError(3).code(),
				format!("Failed to create read proof: {}", e),
				None::<()>,
			)))
		})?;
		Ok(TcCandidatesProof {
			state_root: state_root.as_ref().to_vec(),
			set_code_proof: proof.encode(),
		})
	}
}
