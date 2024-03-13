extern crate alloc;

use std::sync::Arc;
use parking_lot::Mutex;
use core::convert::TryInto;

use sp_runtime::serde::Deserialize;
use jsonrpsee::{
	core::{Error as JsonRpseeError, RpcResult},
	proc_macros::rpc,
	types::error::{CallError, ErrorObject},
};

use schnorrkel::{PublicKey, Signature};
use sp_std::str::FromStr;
use sp_core::ByteArray;

use codec::Encode;

use sp_runtime::{
	generic::BlockId,
	traits::Block as BlockT,
};
use runtime::AccountId;
use sp_core::{H256, U256};
use sp_core::offchain::OffchainStorage;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sc_client_api::backend::Backend;
use sp_consensus_poscan::{MiningPoolApi, CheckMemberError, POSCAN_ALGO_GRID2D_V3A, REJECT_OLD_ALGO_SINCE, POSCAN_ALGO_GRID2D_V3_1};

use alloc::string::String;
use crate::pool::{MiningPool, ShareProposal, LOG_TARGET};
use crate::pool::PoolError;

// const MAX_QUEUE_LEN: usize = 20;

#[allow(non_camel_case_types)]
#[derive(Clone, Copy)]
enum RpcRes {
	RES_OK = 0,
	RES_NOT_ACCEPTED = 1,
	RES_DECRYPT_FAILED = 2,
	RES_POOL_NOT_FOUND = 3,
	RES_POOL_SUSPENDED = 4,
	RES_MEMBER_NOT_FOUND = 5,
}
impl RpcRes {
	pub fn to_u64(&self) -> u64 {
		*self as u64
	}
}

impl From<CheckMemberError> for RpcRes {
	fn from(ms: CheckMemberError) -> Self {
		match ms {
			CheckMemberError::NoPool => RpcRes::RES_POOL_NOT_FOUND,
			CheckMemberError::PoolSuspended => RpcRes::RES_POOL_SUSPENDED,
			CheckMemberError::NoMember => RpcRes::RES_MEMBER_NOT_FOUND,
		}
	}
}

type ParamsResp = (H256, H256, U256, U256, U256);

#[rpc(client, server)]
pub trait PoscanPoolRpcApi<BlockHash> {

	#[method(name = "poscan_getMiningParams")]
	fn get_work_params(
		&self,
		pool_id: AccountId,
	) -> RpcResult<ParamsResp>;

	#[method(name = "poscan_pushMiningObjectToPool")]
	fn push_to_pool(
		&self,
		payload: Vec<u8>,
		member_id: String,
		sign: String,
	) -> RpcResult<u64>;
}

#[derive(Deserialize)]
pub(crate) struct Payload {
	pub(crate) pool_id:     AccountId,
	pub(crate) member_id:   AccountId,
	pub(crate) pre_hash:    H256,
	pub(crate) parent_hash: H256,
	pub(crate) algo:        String,
	pub(crate) dfclty:      U256,
	pub(crate) hash:        H256,
	pub(crate) obj_id:      u64,
	pub(crate) obj:         Vec<u8>,
}

pub struct MiningPoolRpc<C, Block, B>
where
	B: Backend<Block>,
	Block: BlockT,
{
	client:   Arc<C>,
	pool: 	  Arc<Mutex<MiningPool>>,
	backend:  Arc<B>,
	_marker:  std::marker::PhantomData<Block>,
}

impl<C, Block, B> MiningPoolRpc<C, Block, B>
where
	B: Backend<Block>,
	Block: BlockT,
{
	fn push_pow_data(
		&self,
		// at: Option<<Block as BlockT>::Hash>,
		pool_id: AccountId,
		member_id: AccountId,
		pre_hash: H256,
		parent_hash: H256,
		algo_type: String,
		share_dfclty: U256,
		hash: H256,
		_obj_id: u64,
		obj: Vec<u8>,
	) -> RpcResult<u64> {

		let parent_num= self.backend.blockchain().info().best_number;
		let patch_rot = parent_num >= REJECT_OLD_ALGO_SINCE.into();

		if patch_rot {
			if algo_type != "Grid2dV3a" {
				return Err(JsonRpseeError::Custom("Accept Grid2dV3a algorithm only".to_string()))
			}
		}
		else if algo_type != "Grid2dV3.1" && algo_type != "Grid2dV3a" {
			return Err(JsonRpseeError::Custom("Accept Grid2dV3.1 or Grid2dV3a algorithms".to_string()))
		}
		let algo = if algo_type == "Grid2dV3.1" { POSCAN_ALGO_GRID2D_V3_1 } else { POSCAN_ALGO_GRID2D_V3A };

		let is_valid = poscan_algo::check_obj(&algo, &obj, &vec![hash]);

		if !is_valid {
			log::debug!(target: LOG_TARGET, "Object is not valid (pool)");
			return Err(JsonRpseeError::Custom("Object is not valid (pool)".to_string()))
		} else {
			log::debug!(target: LOG_TARGET, "Object is valid (pool)");
		}

		let shp = ShareProposal {
			member_id: member_id.clone(),
			algo_type: algo,
			hash,
			pre_hash,
			parent_hash,
			share_dfclty,
			pre_obj: obj,
		};
		let res = (*self.pool.lock()).try_push(shp, patch_rot);

		match res {
			Ok(_) => {
				log::debug!(target: LOG_TARGET, "push_to_pool. Recieved");
				let mut store = self.backend.offchain_storage().unwrap();
				let prefix = sp_offchain::STORAGE_PREFIX;

				let key = format!("stat::{}::{}", hex::encode(&pool_id.encode()), hex::encode(&member_id.encode()));

				log::debug!(target: LOG_TARGET, "RPC write stat to local storage: key={}", &key);
				log::debug!(target: LOG_TARGET, "RPC write stat to local storage: key={}", hex::encode(&key.as_bytes()));

				let maybe_stat = store.get(prefix, key.as_bytes());
				let val = if let Some(val) = maybe_stat { val } else { vec![0, 0, 0, 0] };
				let val = u32::from_le_bytes(val[..4].try_into().unwrap());
				let val = val + 1;
				store.set(prefix, key.as_bytes(), &val.to_le_bytes());

				log::debug!(target: LOG_TARGET, "push_to_pool. stat submitted");
				Ok(RpcRes::RES_OK.to_u64())
			},
			_ => Ok(RpcRes::RES_NOT_ACCEPTED.to_u64())
		}
	}

	fn error(e: PoolError) -> JsonRpseeError {
		const BASE_ERROR: i32 = 4000;

		match e {
			PoolError::NotAccepted =>
				CallError::Custom(ErrorObject::owned(BASE_ERROR + 0, "No pool", None::<()>)).into(),
			PoolError::CheckMemberError(e) =>
				match e {
					CheckMemberError::NoPool =>
						CallError::Custom(ErrorObject::owned(BASE_ERROR + 1, "No pool", None::<()>))
							.into(),
					CheckMemberError::NoMember =>
						CallError::Custom(ErrorObject::owned(BASE_ERROR + 2, "No member in pool", None::<()>))
							.into(),
					CheckMemberError::PoolSuspended =>
						CallError::Custom(ErrorObject::owned(BASE_ERROR + 3, "Pool suspended", None::<()>))
							.into(),
				},
			PoolError::InvalidMemberError =>
				CallError::Custom(ErrorObject::owned(BASE_ERROR + 4, "Invalid member", None::<()>)).into(),
			PoolError::SignatureError =>
				CallError::Custom(ErrorObject::owned(BASE_ERROR + 5, "Invalid signature", None::<()>)).into(),
			PoolError::SignatureMemberError =>
				CallError::Custom(ErrorObject::owned(BASE_ERROR + 6, "Strange signature", None::<()>)).into(),
		}
	}
}

impl<C, Block, B> MiningPoolRpc<C, Block, B>
where
	B: Backend<Block>,
	Block: BlockT,
{
	pub fn new(client: Arc<C>, backend: Arc<B>, pool: Arc<Mutex<MiningPool>>) -> Self {
		Self {
			client,
			pool: pool.clone(),
			backend: backend.clone(),
			_marker: Default::default(),
		}
	}

	pub (crate) fn decrypt(&self, encrypted: &Vec<u8>) -> Result<Payload, String> {
		let lock = self.pool.lock();
		let secret = ecies_ed25519::SecretKey::from_bytes(&(*lock).secret.to_bytes()).unwrap();
		drop(lock);

		let decrypted = ecies_ed25519::decrypt(&secret, encrypted);
		match decrypted {
			Ok(data) => {
				let payload: Payload = serde_json::from_slice(&data).unwrap();
				Ok(payload)
			},
			Err(ref e) => {
				println!("Decrypt failed: {}", e);
				Err("Decrypt failed".to_string())
			},
		}
	}
}

impl<C, Block, B> PoscanPoolRpcApiServer<<Block as BlockT>::Hash> for MiningPoolRpc<C, Block, B>
	where
		Block: BlockT,
		// Account: Send + Sync + codec::Decode + codec::Encode + 'static,
		C: Send + Sync + 'static,
		// C: ProvideRuntimeApi<Block>,
		C::Api: MiningPoolApi<Block, AccountId>,
		C: ProvideRuntimeApi<Block> + Send + Sync + HeaderBackend<Block>,
		B: Backend<Block> + Send + Sync + 'static,
{
	fn push_to_pool(
		&self,
		payload: Vec<u8>,
		member_id: String,
		sign: String,
	) -> RpcResult<u64> {
		const SIGN_CTX: &[u8] = b"Mining pool";

		let member_acc = AccountId::from_str(&member_id).map_err(|_| Self::error(PoolError::InvalidMemberError))?;
		let pk = PublicKey::from_bytes(member_acc.as_slice()).map_err(|_| Self::error(PoolError::InvalidMemberError))?;
		let sign = hex::decode(sign).map_err(|_| Self::error(PoolError::SignatureError))?;
		let sig = Signature::from_bytes(&sign[..]).map_err(|_| Self::error(PoolError::SignatureError))?;
		pk.verify_simple(SIGN_CTX, &payload[..], &sig).map_err(|_| Self::error(PoolError::SignatureError))?;

		let payload = self.decrypt(&payload);

		if let Ok(payload) = payload {
			let block = BlockId::Hash(self.client.info().best_hash);
			let member_status = self.client
				.runtime_api()
				.member_status(&block, &payload.pool_id, &payload.member_id)
				.unwrap();
				// .into();

			match member_status {
				Ok(_) if payload.member_id != member_acc => Err(Self::error(PoolError::SignatureMemberError)),
				Ok(_) =>
					self.push_pow_data(
						payload.pool_id,
						payload.member_id,
						payload.pre_hash,
						payload.parent_hash,
						payload.algo,
						payload.dfclty,
						payload.hash,
						payload.obj_id,
						payload.obj,
					),
				Err(e) => Err(Self::error(PoolError::CheckMemberError(e)))
			}
		}
		else {
			println!("decrypt failed");
			Ok(RpcRes::RES_DECRYPT_FAILED.to_u64())
		}
	}

	fn get_work_params(
		&self,
		pool_id: AccountId,
	) -> RpcResult<ParamsResp> {
		log::debug!(target: LOG_TARGET, "MiningPool::get_work_params called");
		let lock = self.pool.lock();
		if let Some(mp) = (*lock).curr_meta.clone() {
			let pre_hash = mp.pre_hash;
			let parent_hash = mp.parent_hash;
			let win_dfclty = mp.difficulty;

			let block = BlockId::Hash(self.client.info().best_hash);
			log::debug!(target: LOG_TARGET, "win_dfclty: {:?}", &win_dfclty);
			let pow_dfclty: U256 = self.client.runtime_api().difficulty(&block, &pool_id).unwrap().into();
			log::debug!(target: LOG_TARGET, "pow_dfclty: {:?}", &pow_dfclty);
			let key = (*lock).public();

			log::debug!(target: LOG_TARGET, "key: {}", &hex::encode(key.encode()));
			Ok((pre_hash, parent_hash, win_dfclty, pow_dfclty, (*lock).public()))
		}
		else {
			Err(JsonRpseeError::Custom("No data".to_string()))
		}
	}
}
