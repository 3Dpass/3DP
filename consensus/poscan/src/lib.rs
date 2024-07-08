// This file is part of 3DPass

// Copyright (C) 2017-2021 Parity Technologies (UK) Ltd.
// Copyright (C) 2022 3DPass
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// 3DPass is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// 3DPass is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Proof of scan consensus for Substrate.
//!
//! To use this engine, you might need to have a struct that implements
//! `PoscanAlgorithm`. After that, pass an instance of the struct, along
//! with other necessary client references to `import_queue` to setup
//! the queue. Use the mining RPC request (mining_rpc.rs) for 3D models
//! to be pushed into the the mining queue for basic CPU mining.
//!
//! The auxiliary storage for PoScan engine not only stores the total difficulty,
//! but also 3D models (in .obj format) of the objects to recognize.

//#![no_std]
#![cfg_attr(not(feature = "std"), no_std)]

mod worker;

pub use crate::worker::{MiningHandle, MiningMetadata, MiningBuild};

use std::{
	sync::Arc, borrow::Cow, collections::HashMap, marker::PhantomData,
	cmp::Ordering, time::Duration
};
use futures::prelude::*;
use parking_lot::Mutex;
use sc_client_api::{BlockOf, backend::AuxStore, BlockchainEvents, BlockBackend};
use sc_consensus::{
	BlockImportParams, BlockCheckParams, ImportResult, ForkChoiceStrategy, BlockImport,
	import_queue::{
		BoxBlockImport, BasicQueue, Verifier, BoxJustificationImport,
	}
};
use sp_blockchain::{HeaderBackend, HeaderMetadata, well_known_cache_keys::Id as CacheKeyId};
// use sp_blockchain::{HeaderBackend, HeaderMetadata, ProvideCache, well_known_cache_keys::Id as CacheKeyId};
use sp_block_builder::BlockBuilder as BlockBuilderApi;
use sp_runtime::RuntimeString;
use sp_runtime::generic::{BlockId, Digest, DigestItem};
use sp_runtime::traits::{Block as BlockT, Header as HeaderT};
use sp_api::ProvideRuntimeApi;
use sp_finality_grandpa::GRANDPA_ENGINE_ID;
use sp_consensus_poscan::{
	Seal,
	TotalDifficulty,
	POSCAN_ENGINE_ID,
	CONS_V2_SPEC_VER,
	POSCAN_SEAL_V1_ID,
	POSCAN_SEAL_V2_ID,
};
use sp_inherents::{CreateInherentDataProviders, InherentDataProvider}; //, InherentData};
use sp_consensus::{
	SyncOracle, Environment, Proposer,
	SelectChain, Error as ConsensusError, CanAuthorWith,
};
use codec::{Encode, Decode};
use prometheus_endpoint::Registry;
use log::*;
use sp_core::{H256, U256};
use core::ops::Bound::{Excluded, Included};
use sp_std::collections::btree_set::BTreeSet;
use std::convert::TryInto;
use lazy_static::lazy_static;

use sp_core::ExecutionContext;

use crate::worker::UntilImportedOrTimeout;
use sp_consensus_poscan::{
	Difficulty,
	DifficultyApi,
	MAX_MINING_OBJ_LEN,
	SUPPORTED_ALGORITHMS,
	POSCAN_ALGO_GRID2D_V3A,
	REJECT_OLD_ALGO_SINCE,
};

lazy_static! {
    pub static ref CACHE: Mutex<BTreeSet<u64>> = {
        let m = BTreeSet::new();
        Mutex::new(m)
    };
}

pub mod app {
	use sp_application_crypto::{app_crypto, sr25519};
	use sp_core::crypto::KeyTypeId;
	use core::convert::TryFrom;

	pub const ID: KeyTypeId = KeyTypeId(*b"3dpp");
	app_crypto!(sr25519, ID);
}

#[derive(derive_more::Display, Debug)]
pub enum Error<B: BlockT> {
	#[display(fmt = "Header uses the wrong engine {:?}", _0)]
	WrongEngine([u8; 4]),
	#[display(fmt = "Header {:?} is unsealed", _0)]
	HeaderUnsealed(B::Hash),
	#[display(fmt = "PoScan validation error: invalid seal")]
	InvalidSeal,
	#[display(fmt = "PoScan validation error: preliminary verification failed")]
	FailedPreliminaryVerify,
	#[display(fmt = "Rejecting block too far in future")]
	TooFarInFuture,
	#[display(fmt = "Fetching best header failed using select chain: {:?}", _0)]
	BestHeaderSelectChain(ConsensusError),
	#[display(fmt = "Fetching best header failed: {:?}", _0)]
	BestHeader(sp_blockchain::Error),
	#[display(fmt = "Best header does not exist")]
	NoBestHeader,
	#[display(fmt = "Block proposing error: {:?}", _0)]
	BlockProposingError(String),
	#[display(fmt = "Fetch best hash failed via select chain: {:?}", _0)]
	BestHashSelectChain(ConsensusError),
	#[display(fmt = "Error with block built on {:?}: {:?}", _0, _1)]
	BlockBuiltError(B::Hash, ConsensusError),
	#[display(fmt = "Creating inherents failed: {}", _0)]
	CreateInherents(sp_inherents::Error),
	#[display(fmt = "Checking inherents failed: {}", _0)]
	CheckInherents(sp_inherents::Error),
	#[display(fmt = "Too far from finalized block")]
	CheckFinalized,
	#[display(
	fmt = "Checking inherents unknown error for identifier: {:?}",
	"String::from_utf8_lossy(_0)"
	)]
	CheckInherentsUnknownError(sp_inherents::InherentIdentifier),
	#[display(fmt = "Multiple pre-runtime digests")]
	MultiplePreRuntimeDigests,
	Client(sp_blockchain::Error),
	Codec(codec::Error),
	Environment(String),
	Runtime(RuntimeString),
	Other(String),
}

impl<B: BlockT> std::convert::From<Error<B>> for String {
	fn from(error: Error<B>) -> String {
		error.to_string()
	}
}

impl<B: BlockT> std::convert::From<Error<B>> for ConsensusError {
	fn from(error: Error<B>) -> ConsensusError {
		ConsensusError::ClientImport(error.to_string())
	}
}

/// Auxiliary storage prefix for PoW engine.
pub const POW_AUX_PREFIX: [u8; 4] = *b"PoSc";

/// Get the auxiliary storage key used by engine to store total difficulty.
fn aux_key<T: AsRef<[u8]>>(hash: &T) -> Vec<u8> {
	POW_AUX_PREFIX.iter().chain(hash.as_ref()).copied().collect()
}

/// Intermediate value passed to block importer.
#[derive(Encode, Decode, Clone, Debug, Default)]
pub struct PowIntermediate<Difficulty> {
	/// Difficulty of the block, if known.
	pub difficulty: Option<Difficulty>,
	// pub obj: Vec<u8>,
}

/// Intermediate key for PoW engine.
pub static INTERMEDIATE_KEY: &[u8] = b"posc";

/// Auxiliary storage data for PoW.
#[derive(Encode, Decode, Clone, Debug, Default)]
pub struct PowAux<Difficulty> {
	/// Difficulty of the current block.
	pub difficulty: Difficulty,
	/// Total difficulty up to current block.
	pub total_difficulty: Difficulty,
}

impl<Difficulty> PowAux<Difficulty> where
	Difficulty: Decode + Default,
{
	/// Read the auxiliary from client.
	pub fn read<C: AuxStore, B: BlockT>(client: &C, hash: &B::Hash) -> Result<Self, Error<B>> {
		let key = aux_key(&hash);

		match client.get_aux(&key).map_err(Error::Client)? {
			Some(bytes) => Self::decode(&mut &bytes[..]).map_err(Error::Codec),
			None => Ok(Self::default()),
		}
	}
}

#[derive(Clone, Encode, Decode, PartialEq, Eq, Debug)]
pub enum PoscanData {
	V1(PoscanDataV1),
	V2(PoscanDataV2),
}

#[derive(Clone, Encode, Decode, PartialEq, Eq, Debug)]
pub struct PoscanDataV1 {
	pub alg_id: [u8;16],
	pub hashes: Vec<H256>,
	pub obj: Vec<u8>,
}

#[derive(Clone, Encode, Decode, PartialEq, Eq, Debug)]
pub struct PoscanDataV2 {
	pub alg_id: [u8;16],
	pub hashes: Vec<H256>,
	pub orig_hashes: Vec<H256>,
	pub obj: Vec<u8>,
}

/// Algorithm used for proof of work.
pub trait PowAlgorithm<B: BlockT> {
	/// Difficulty for the algorithm.
	type Difficulty: TotalDifficulty + Default + Encode + Decode + Ord + Clone + Copy;

	/// Get the next block's difficulty.
	///
	/// This function will be called twice during the import process, so the implementation
	/// should be properly cached.
	fn difficulty(&self, parent: B::Hash) -> Result<Self::Difficulty, Error<B>>;
	/// Verify that the seal is valid against given pre hash when parent block is not yet imported.
	///
	/// None means that preliminary verify is not available for this algorithm.
	fn preliminary_verify(
		&self,
		_pre_hash: &B::Hash,
		_seal: &Seal,
	) -> Result<Option<bool>, Error<B>> {
		Ok(None)
	}
	/// Break a fork choice tie.
	///
	/// By default this chooses the earliest block seen. Using uniform tie
	/// breaking algorithms will help to protect against selfish mining.
	///
	/// Returns if the new seal should be considered best block.
	fn break_tie(
		&self,
		_own_seal: &Seal,
		_new_seal: &Seal,
	) -> bool {
		false
	}
	/// Verify that the difficulty is valid against given seal.
	fn verify(
		&self,
		parent: &B::Hash,
		pre_hash: &B::Hash,
		pre_digest: Option<&[u8]>,
		seal: &Seal,
		difficulty: Self::Difficulty,
		poscan_data: &PoscanData,
	) -> Result<bool, Error<B>>;
}

/// A block importer for PoW.
pub struct PowBlockImport<B: BlockT, I, C, S, Algorithm, CAW, CIDP, AccountId, BlockNumber> {
	algorithm: Algorithm,
	inner: I,
	select_chain: S,
	client: Arc<C>,
	create_inherent_data_providers: Arc<CIDP>,
	check_inherents_after: <<B as BlockT>::Header as HeaderT>::Number,
	can_author_with: CAW,
	last_cached: u32,
	_phantom: PhantomData<AccountId>,
	_phantom2: PhantomData<BlockNumber>,
}

impl<B: BlockT, I: Clone, C, S: Clone, Algorithm: Clone, CAW: Clone, CIDP, AccountId, BlockNumber> Clone
	for PowBlockImport<B, I, C, S, Algorithm, CAW, CIDP, AccountId, BlockNumber>
{
	fn clone(&self) -> Self {
		Self {
			algorithm: self.algorithm.clone(),
			inner: self.inner.clone(),
			select_chain: self.select_chain.clone(),
			client: self.client.clone(),
			create_inherent_data_providers: self.create_inherent_data_providers.clone(),
			check_inherents_after: self.check_inherents_after,
			can_author_with: self.can_author_with.clone(),
			last_cached: self.last_cached,
			_phantom: self._phantom,
			_phantom2: self._phantom2,
		}
	}
}

use sp_consensus_poscan::PoscanApi;
use sp_api::Core;

impl<B, I, C, S, Algorithm, CAW, CIDP, AccountId, BlockNumber> PowBlockImport<B, I, C, S, Algorithm, CAW, CIDP, AccountId, BlockNumber>
	where
		B: BlockT,
		I: BlockImport<B, Transaction = sp_api::TransactionFor<C, B>> + Send + Sync,
		I::Error: Into<ConsensusError>,
		C: ProvideRuntimeApi<B> + Send + Sync + HeaderBackend<B> + AuxStore + BlockOf,
		C::Api: BlockBuilderApi<B>,
		C::Api: PoscanApi<B, AccountId, BlockNumber>,
		C::Api: Core<B>,
		BlockNumber: Clone + Eq + Debug + Sync + Send + codec::Decode + codec::Encode + TypeInfo + Member,
		AccountId: Clone + Eq + Debug + Sync + Send + codec::Decode + codec::Encode + TypeInfo + Member,
		Algorithm: PowAlgorithm<B>,
		CAW: CanAuthorWith<B>,
		CIDP: CreateInherentDataProviders<B, ()>,
{
	/// Create a new block import suitable to be used in PoW
	pub fn new(
		inner: I,
		client: Arc<C>,
		algorithm: Algorithm,
		check_inherents_after: <<B as BlockT>::Header as HeaderT>::Number,
		select_chain: S,
		create_inherent_data_providers: CIDP,
		can_author_with: CAW,
	) -> Self {
		Self {
			inner,
			client,
			algorithm,
			check_inherents_after,
			select_chain,
			create_inherent_data_providers: Arc::new(create_inherent_data_providers),
			can_author_with,
			last_cached: 0,
			_phantom: PhantomData,
			_phantom2: PhantomData,
		}
	}

	async fn check_inherents(
		&self,
		block: B,
		block_id: BlockId<B>,
		inherent_data_providers: CIDP::InherentDataProviders,
		execution_context: ExecutionContext,
	) -> Result<(), Error<B>> {
		if *block.header().number() < self.check_inherents_after {
			return Ok(())
		}

		if let Err(e) = self.can_author_with.can_author_with(&block_id) {
			debug!(
				target: "pow",
				"Skipping `check_inherents` as authoring version is not compatible: {}",
				e,
			);

			return Ok(())
		}

		let inherent_data = inherent_data_providers
			.create_inherent_data()
			.map_err(|e| Error::CreateInherents(e))?;

		let inherent_res = self
			.client
			.runtime_api()
			.check_inherents_with_context(&block_id, execution_context, block, inherent_data)
			.map_err(|e| Error::Client(e.into()))?;

		if !inherent_res.ok() {
			for (identifier, error) in inherent_res.into_errors() {
				match inherent_data_providers.try_handle_error(&identifier, &error).await {
					Some(res) => res.map_err(Error::CheckInherents)?,
					None => return Err(Error::CheckInherentsUnknownError(identifier)),
				}
			}
		}

		Ok(())
	}

	fn update_cache(&mut self) {
		let info = self.client.info();
		let block_id = BlockId::Hash(info.finalized_hash);
		let fin_num = self.client.block_number_from_id(&block_id).unwrap().unwrap();
		let fin_num: u32 = u32::from_le_bytes(fin_num.encode()[0..4].try_into().unwrap());

		for number in (self.last_cached + 1)..=fin_num {
			let digest = self.client
				.header(BlockId::Number(number.into()))
				.and_then(|maybe_header| match maybe_header {
					Some(header) => Ok(Some(header.digest().clone())),
					None => Ok(None),
				});

			if let Ok(None) = digest {
				continue
			}
			let digest = digest.unwrap().unwrap();

			let n = digest.logs().len();
			if n >= 3 {
				let di = digest.logs()[n - 2].clone();
				if let DigestItem::Other(v) = di {
					let hashes: Vec<H256> = v[16..].chunks(32).map(H256::from_slice).collect();
					let item = [&number.encode()[..], &hashes[0].encode()[0..4]].concat();
					let item: u64 = u64::from_le_bytes(item.try_into().unwrap());
					CACHE.lock().insert(item);
					self.last_cached = number;
					debug!(target: "cache", "Update cache: block {}", number);
				}
			}
		}
	}

	fn check_exists(&self, hash: H256) -> bool {
		let short_hash = u32::from_le_bytes(hash.encode()[0..4].try_into().unwrap());
		let from: u64 = (short_hash as u64) << 32;
		let to: u64 = from + (1u64 << 32);

		for item in CACHE.lock().range((Included(from), Excluded(to))) {
			debug!(target: "cache", "Lookup in cache: item = {:?}", item.encode());
			let num = u32::from_le_bytes(item.encode()[0..4].try_into().unwrap());
			if let Some(block_hash) = self.get_poscan_hash(num) {
				if block_hash == hash {
					debug!(target: "cache", "Lookup in cache: found full hash in header");
					return true;
				}
				debug!(target: "cache", "Lookup in cache: found full hash but it differs");
			}
		}
		false
	}

	fn get_poscan_hash(&self, block_num: u32) -> Option<H256> {
		let digest = self.client
			.header(BlockId::Number(block_num.into()))
			.and_then(|maybe_header| match maybe_header {
				Some(header) => Ok(Some(header.digest().clone())),
				None => Ok(None),
			});

		if let Ok(None) = digest {
			debug!(target: "cache", "get_poscan_hash: no digest");

			return None
		}
		let digest = digest.unwrap().unwrap();

		let n = digest.logs().len();
		debug!(target: "cache", "digest len=: {}", n);

		if n >= 3 {
			let di = digest.logs()[n-2].clone();
			if let DigestItem::Other(v) = di {
				let hashes: Vec<H256> = v[16..].chunks(32).map(H256::from_slice).collect();
				return Some(hashes[0])
			}
		}
		None
	}
}

use sp_runtime::traits::Member;
use sp_std::fmt::Debug;
use scale_info::TypeInfo;

#[async_trait::async_trait]
impl<B, I, C, S, Algorithm, CAW, CIDP, AccountId, BlockNumber> BlockImport<B> for PowBlockImport<B, I, C, S, Algorithm, CAW, CIDP, AccountId, BlockNumber> where
	B: BlockT,
	I: BlockImport<B, Transaction = sp_api::TransactionFor<C, B>> + Send + Sync,
	I::Error: Into<ConsensusError>,
	S: SelectChain<B>,
	C: ProvideRuntimeApi<B> + Send + Sync + HeaderBackend<B> + AuxStore + BlockOf + BlockBackend<B>,
	C::Api: BlockBuilderApi<B>,
	C::Api: DifficultyApi<B, Difficulty>,
	C::Api: PoscanApi<B, AccountId, BlockNumber>,
	AccountId: Clone + Eq + Sync + Send + Debug + Encode + Decode + TypeInfo + Member,
	BlockNumber: Clone + Eq + Sync + Send + Debug + Encode + Decode + TypeInfo + Member,
	Algorithm: PowAlgorithm<B> + Send + Sync,
	Algorithm::Difficulty: From<U256> + Send + 'static,
	CAW: CanAuthorWith<B> + Send + Sync,
	CIDP: CreateInherentDataProviders<B, ()> + Send + Sync,
{
	type Error = ConsensusError;
	type Transaction = sp_api::TransactionFor<C, B>;

	async fn check_block(
		&mut self,
		block: BlockCheckParams<B>,
	) -> Result<ImportResult, Self::Error> {
		self.inner.check_block(block).await.map_err(Into::into)
	}

	async fn import_block(
		&mut self,
		mut block: BlockImportParams<B, Self::Transaction>,
		new_cache: HashMap<CacheKeyId, Vec<u8>>,
	) -> Result<ImportResult, Self::Error> {
		let best_header = self.select_chain.best_chain()
			.await
			.map_err(|e| format!("Fetch best chain failed via select chain: {:?}", e))?;
		let best_hash = best_header.hash();

		let info = self.client.info();
		let block_id = BlockId::Hash(info.finalized_hash);
		let fin_num = self.client.block_number_from_id(&block_id).unwrap().unwrap();

		let parent_hash = *block.header.parent_hash();
		let best_aux = PowAux::read::<_, B>(self.client.as_ref(), &best_hash)?;
		let mut aux = PowAux::read::<_, B>(self.client.as_ref(), &parent_hash)?;

		if let Some(inner_body) = block.body.take() {
			// let timestamp_now = inherent_data.timestamp_inherent_data().map_err(|e| e.into_string())?;

			let check_block = B::new(block.header.clone(), inner_body);

			self.check_inherents(
				check_block.clone(),
				BlockId::Hash(parent_hash),
				self.create_inherent_data_providers
					.create_inherent_data_providers(parent_hash, ())
					.await?,
				block.origin.into(),
				// timestamp_now,
			)
				.await?;

			block.body = Some(check_block.deconstruct().1);
		}

		let digest_size = block.post_digests.len();

		let pre_digest: Vec<u8> = find_pre_digest::<B>(&block.header)?.unwrap();

		let pscan_obj = fetch_seal::<B>(block.post_digests.get(digest_size - 1), block.header.hash())?;
		let pscan_hashes = fetch_seal::<B>(block.post_digests.get(digest_size - 2), block.header.hash())?;

		if pscan_obj.len() > MAX_MINING_OBJ_LEN {
			return Err(Error::<B>::Other("Mining object too large".to_string()).into());
		}

		let alg_id: &[u8; 16] = &pscan_hashes[0..16].try_into().unwrap();
		if !SUPPORTED_ALGORITHMS.contains(alg_id) {
			return Err(Error::<B>::Other("Unknown algorithm".to_string()).into());
		}

		let block_num = *block.header.number();
		if block_num > REJECT_OLD_ALGO_SINCE.into() && *alg_id != POSCAN_ALGO_GRID2D_V3A {
			return Err(Error::<B>::Other("Unsupported algorithm".to_string()).into());
		}

		let hs: Vec<H256> = pscan_hashes[16..].chunks(32).map(H256::from_slice).collect();

		let parent_id: BlockId<B> = BlockId::hash(parent_hash);
		let ver = self.client
			.runtime_api()
			.version(&parent_id)
			.map_err(|err|
				Error::<B>::Environment(format!(
					">>> get version call failed in import_block: {:?}",
					err
				))
			)?;

		let (inner_seal, psdata) = if ver.spec_version < CONS_V2_SPEC_VER {
			let inner_seal = fetch_seal::<B>(block.post_digests.get(digest_size - 3), block.header.hash())?;
			(inner_seal, PoscanData::V1(PoscanDataV1{ alg_id: *alg_id, hashes: hs.clone(), obj: pscan_obj }))
		}
		else {
			let inner_seal = fetch_seal::<B>(block.post_digests.get(digest_size - 4), block.header.hash())?;
			let orig_pscan_hashes = fetch_seal::<B>(block.post_digests.get(digest_size - 3), block.header.hash())?;
			let orig_hs: Vec<H256> = orig_pscan_hashes[16..].chunks(32).map(H256::from_slice).collect();
			(inner_seal, PoscanData::V2(PoscanDataV2 { alg_id: *alg_id, hashes: hs.clone(), orig_hashes: orig_hs.clone(), obj: pscan_obj }))
		};

		let intermediate = block.take_intermediate::<PowIntermediate::<Algorithm::Difficulty>>(
			INTERMEDIATE_KEY
		)?;

		let difficulty = match intermediate.difficulty {
			Some(difficulty) => difficulty,
			None => self.algorithm.difficulty(parent_hash)?,
		};

		let mut h = block.header.clone();
		if block.header.digest().logs().len() == 2 {
			let _ = h.digest_mut().pop();
		}

		let pre_hash = h.hash();

		// let pre_digest = find_pre_digest::<B>(&block.header)?;
		if !self.algorithm.verify(
			&parent_hash,
			&pre_hash,
			Some(pre_digest.as_slice()),
			&inner_seal,
			difficulty,
			&psdata,
		)? {
			error!(">>> InvalidSeal after self.algorithm.verify");
			return Err(Error::<B>::InvalidSeal.into())
		}

		self.update_cache();
		let mut prev_hash = Some(parent_hash);
		while prev_hash.is_some() {
			let block_id: BlockId<B> = BlockId::hash(prev_hash.unwrap());
			let num = self.client.block_number_from_id(&block_id).unwrap().unwrap();

			if num <= fin_num {
				debug!(target: "cache", "Lookup in cache");

				if self.check_exists(hs[0]) {
					info!(">>>>>> duplicated hash found in cache");
					return Err(Error::<B>::InvalidSeal.into());
				}
				debug!(target: "cache", "Lookup in cache: not found");
				break
			}
			match self.client.block(&block_id) {
				Ok(maybe_prev_block) => {
					match maybe_prev_block {
						Some(signed_block) => {
							let h = signed_block.block.header();
							let n = h.digest().logs().len();
							if n >= 3 {
								let di = h.digest().logs()[n - 2].clone();
								if let DigestItem::Other(v) = di {
									let hashes: Vec<H256> = v[16..].chunks(32).map(H256::from_slice).collect();
									for hh in hashes[..1].iter() {
										if hs[..1].contains(hh) {
											info!(">>>>>> duplicated hash found");
											return Err(Error::<B>::InvalidSeal.into());
										}
									}
								}
								else {
									error!(">>> No poscan hashes in header");
									return Err(Error::<B>::HeaderUnsealed(h.hash()).into())
								}
							}
							prev_hash = Some(*h.parent_hash());
						},
						None => {
							prev_hash = None
						},
					}
				},
				Err(_e) => {
					return Err(Error::<B>::InvalidSeal.into());
				},
			}
		}

		aux.difficulty = difficulty;
		aux.total_difficulty.increment(difficulty);

		let key = aux_key(&block.post_hash());
		block.auxiliary.push((key, Some(aux.encode())));
		if block.fork_choice.is_none() {
			block.fork_choice = Some(ForkChoiceStrategy::Custom(
				match aux.total_difficulty.cmp(&best_aux.total_difficulty) {
					Ordering::Less => false,
					Ordering::Greater => true,
					Ordering::Equal => {
						let best_inner_seal = fetch_seal::<B>(
							best_header.digest().logs.last(),
							best_hash,
						)?;

						self.algorithm.break_tie(&best_inner_seal, &inner_seal)
					},
				}
			));
		}

		self.inner.import_block(block, new_cache).await.map_err(Into::into)
	}
}

/// A verifier for PoW blocks.
pub struct PowVerifier<B: BlockT, Algorithm> {
	// TODO:
	_algorithm: Algorithm,
	_marker: PhantomData<B>,
}

impl<B: BlockT, Algorithm> PowVerifier<B, Algorithm> {
	pub fn new(
		algorithm: Algorithm,
	) -> Self {
		Self { _algorithm: algorithm, _marker: PhantomData }
	}

	fn check_header(
		&self,
		mut header: B::Header,
	) -> Result<(B::Header, Vec<DigestItem>), Error<B>> where
		Algorithm: PowAlgorithm<B>,
	{
		let mut digests: Vec<DigestItem> = Vec::new();

		for _ in 0..4 {
			match header.digest_mut().pop() {
				Some(DigestItem::Seal(id, seal)) => {
					if id == POSCAN_SEAL_V1_ID || id == POSCAN_SEAL_V2_ID {
						digests.push(DigestItem::Seal(id, seal.clone()));
						if id == POSCAN_SEAL_V1_ID {
							break
						}
					} else {
						return Err(Error::WrongEngine(id))
					}
				},
				Some(DigestItem::Other(item)) => {
					digests.push(DigestItem::Other(item.clone()))
				},
				_ => {
					let msg = ">>> Header invalid in check_header";
					error!("{}", msg);
					return Err(Error::Other(msg.to_string()))
				},
			};
		}
		for item in header.digest().logs().iter() {
			if let DigestItem::Consensus(id, _) = item {
				if *id != GRANDPA_ENGINE_ID {
					return Err(Error::WrongEngine(*id))
				}
			}
		}

		Ok((header, digests))
	}
}

#[async_trait::async_trait]
impl<B: BlockT, Algorithm> Verifier<B> for PowVerifier<B, Algorithm> where
	Algorithm: PowAlgorithm<B> + Send + Sync,
	Algorithm::Difficulty: 'static + Send,
{
	async fn verify(
			&mut self,
			block: BlockImportParams<B, ()>,
		) -> Result<(BlockImportParams<B, ()>, Option<Vec<(CacheKeyId, Vec<u8>)>>), String> {

		let (checked_header, items) = self.check_header(block.header)?;

		let mut seal: Option<DigestItem> = None;
		let mut orig_hashes: Option<DigestItem> = None;
		let mut poscan_hashes: Option<DigestItem> = None;
		let mut poscan_obj: Option<DigestItem> = None;

		if items.len() == 4 {
			seal = Some(items[3].clone());
			orig_hashes = Some(items[2].clone());
			poscan_hashes = Some(items[1].clone());
			poscan_obj = Some(items[0].clone());
		}
		else if items.len() == 3 {
			seal = Some(items[2].clone());
			poscan_hashes = Some(items[1].clone());
			poscan_obj = Some(items[0].clone());
		}

		let intermediate = PowIntermediate::<Algorithm::Difficulty> {
			difficulty: None,
		};

		let mut import_block = BlockImportParams::new(block.origin, checked_header);

		if let Some(seal) = seal { import_block.post_digests.push(seal) };
		if let Some(orig_hashes) = orig_hashes { import_block.post_digests.push(orig_hashes) };
		if let Some(poscan_hashes) = poscan_hashes { import_block.post_digests.push(poscan_hashes) };
		if let Some(poscan_obj) = poscan_obj { import_block.post_digests.push(poscan_obj) };
		import_block.body = block.body;
		import_block.justifications = block.justifications;
		import_block.intermediates.insert(
			Cow::from(INTERMEDIATE_KEY),
			Box::new(intermediate) as Box<_>
		);
		// import_block.post_hash = Some(post_hash);

		Ok((import_block, None))
	}
}

/// Register the PoW inherent data provider, if not registered already.
// pub fn register_pow_inherent_data_provider(
// 	inherent_data_providers: &InherentDataProviders,
// ) -> Result<(), sp_consensus::Error> {
// 	if !inherent_data_providers.has_provider(&sp_timestamp::INHERENT_IDENTIFIER) {
// 		inherent_data_providers
// 			.register_provider(sp_timestamp::InherentDataProvider)
// 			.map_err(Into::into)
// 			.map_err(sp_consensus::Error::InherentData)
// 	} else {
// 		Ok(())
// 	}
// }

/// The PoW import queue type.
pub type PowImportQueue<B, Transaction> = BasicQueue<B, Transaction>;

/// Import queue for PoW engine.
pub fn import_queue<B, Transaction, Algorithm>(
	block_import: BoxBlockImport<B, Transaction>,
	justification_import: Option<BoxJustificationImport<B>>,
	algorithm: Algorithm,
	// inherent_data_providers: InherentDataProviders,
	spawner: &impl sp_core::traits::SpawnEssentialNamed,
	registry: Option<&Registry>,
) -> Result<
	PowImportQueue<B, Transaction>,
	sp_consensus::Error
> where
	B: BlockT,
	Transaction: Send + Sync + 'static,
	Algorithm: PowAlgorithm<B> + Clone + Send + Sync + 'static,
	Algorithm::Difficulty: Send,
{
	// register_pow_inherent_data_provider(&inherent_data_providers)?;

	let verifier = PowVerifier::new(algorithm);

	Ok(BasicQueue::new(
		verifier,
		block_import,
		justification_import,
		spawner,
		registry,
	))
}

/// Find PoW pre-runtime.
fn find_pre_digest<B: BlockT>(header: &B::Header) -> Result<Option<Vec<u8>>, Error<B>> {
	let mut pre_digest: Option<_> = None;
	for log in header.digest().logs() {
		trace!(target: "pow", "Checking log {:?}, looking for pre runtime digest", log);
		match (log, pre_digest.is_some()) {
			(DigestItem::PreRuntime(POSCAN_ENGINE_ID, _), true) => {
				return Err(Error::MultiplePreRuntimeDigests)
			},
			(DigestItem::PreRuntime(POSCAN_ENGINE_ID, v), false) => {
				pre_digest = Some(v.clone());
			},
			(_, _) => trace!(target: "pow", "Ignoring digest not meant for us"),
		}
	}

	Ok(pre_digest)
}

/// Fetch PoW seal.
fn fetch_seal<B: BlockT>(
	digest: Option<&DigestItem>,
	hash: B::Hash,
) -> Result<Vec<u8>, Error<B>> {
	match digest {
		Some(DigestItem::Seal(id, seal)) => {
			if id == &POSCAN_SEAL_V1_ID || id == &POSCAN_SEAL_V2_ID{
				Ok(seal.clone())
			} else {
				Err(Error::<B>::WrongEngine(*id).into())
			}
		},
		Some(DigestItem::Other(item)) => {
			Ok(item.clone())
		},
		Some(DigestItem::PreRuntime(id, pre_runtime)) => {
			if id == &POSCAN_ENGINE_ID {
				Ok(pre_runtime.clone())
			}
			else {
				Err(Error::<B>::WrongEngine(*id).into())
			}
		},
		Some(DigestItem::Consensus(id, v)) => {
			if id == &GRANDPA_ENGINE_ID {
				Ok(v.clone())
			}
			else {
				Err(Error::<B>::WrongEngine(*id).into())
			}
		},
		_ => {
			error!(">>> Header unsealed in fetch_seal");
			Err(Error::<B>::HeaderUnsealed(hash).into())
		},
	}
}

//

/// Start the mining worker for PoW. This function provides the necessary helper functions that can
/// be used to implement a miner. However, it does not do the CPU-intensive mining itself.
///
/// Two values are returned -- a worker, which contains functions that allows querying the current
/// mining metadata and submitting mined blocks, and a future, which must be polled to fill in
/// information in the worker.
///
/// `pre_runtime` is a parameter that allows a custom additional pre-runtime digest to be inserted
/// for blocks being built. This can encode authorship information, or just be a graffiti.
pub fn start_mining_worker<Block, C, S, Algorithm, E, SO, L, CIDP, CAW>(
	block_import: BoxBlockImport<Block, sp_api::TransactionFor<C, Block>>,
	client: Arc<C>,
	select_chain: S,
	algorithm: Algorithm,
	mut env: E,
	mut sync_oracle: SO,
	justification_sync_link: L,
	pre_runtime: Option<Vec<u8>>,
	create_inherent_data_providers: CIDP,
	timeout: Duration,
	build_time: Duration,
	can_author_with: CAW,
) -> (
	MiningHandle<Block, Algorithm, C, L, <E::Proposer as Proposer<Block>>::Proof>,
	impl Future<Output = ()>,
)
	where
		Block: BlockT,
		C: ProvideRuntimeApi<Block> + BlockchainEvents<Block> + 'static + BlockBackend<Block> + HeaderBackend<Block> + HeaderMetadata<Block, Error = sp_blockchain::Error>,
		S: SelectChain<Block> + 'static,
		Algorithm: PowAlgorithm<Block> + Clone,
		Algorithm::Difficulty: Send + 'static,
		E: Environment<Block> + Send + Sync + 'static,
		E::Error: std::fmt::Debug,
		E::Proposer: Proposer<Block, Transaction = sp_api::TransactionFor<C, Block>>,
		SO: SyncOracle + Clone + Send + Sync + 'static,
		L: sc_consensus::JustificationSyncLink<Block>,
		CIDP: CreateInherentDataProviders<Block, ()>,
		CAW: CanAuthorWith<Block> + Clone + Send + 'static,
{
	let mut timer = UntilImportedOrTimeout::new(client.import_notification_stream(), timeout);
	let worker = MiningHandle::new(algorithm.clone(), block_import, justification_sync_link);
	let worker_ret = worker.clone();

	let task = async move {
		loop {
			if timer.next().await.is_none() {
				break;
			}

			if sync_oracle.is_major_syncing() {
				debug!(target: "pow", "Skipping proposal due to sync.");
				worker.on_major_syncing();
				continue;
			}

			let best_header = match select_chain.best_chain().await {
				Ok(x) => x,
				Err(err) => {
					warn!(
						target: "pow",
						"Unable to pull new block for authoring. \
						 Select best chain error: {:?}",
						err
					);
					continue;
				}
			};
			let best_hash = best_header.hash();

			if let Err(err) = can_author_with.can_author_with(&BlockId::Hash(best_hash)) {
				warn!(
					target: "pow",
					"Skipping proposal `can_author_with` returned: {} \
					 Probably a node update is required!",
					err,
				);
				continue;
			}

			if worker.best_hash() == Some(best_hash) {
				continue;
			}

			// The worker is locked for the duration of the whole proposing period. Within this
			// period, the mining target is outdated and useless anyway.

			let difficulty = match algorithm.difficulty(best_hash) {
				Ok(x) => x,
				Err(err) => {
					warn!(
						target: "pow",
						"Unable to propose new block for authoring. \
						 Fetch difficulty failed: {:?}",
						err,
					);
					continue;
				}
			};

			let inherent_data_providers = match create_inherent_data_providers
				.create_inherent_data_providers(best_hash, ())
				.await
			{
				Ok(x) => x,
				Err(err) => {
					warn!(
						target: "pow",
						"Unable to propose new block for authoring. \
						 Creating inherent data providers failed: {:?}",
						err,
					);
					continue;
				}
			};

			let inherent_data = match inherent_data_providers.create_inherent_data() {
				Ok(r) => r,
				Err(e) => {
					warn!(
						target: "pow",
						"Unable to propose new block for authoring. \
						 Creating inherent data failed: {:?}",
						e,
					);
					continue;
				}
			};

			let mut inherent_digest = Digest::default();
			if let Some(pre_runtime) = &pre_runtime {
				inherent_digest.push(DigestItem::PreRuntime(POSCAN_ENGINE_ID, pre_runtime.to_vec()));
			}

			let pre_runtime = pre_runtime.clone();

			let proposer = match env.init(&best_header).await {
				Ok(x) => x,
				Err(err) => {
					warn!(
						target: "pow",
						"Unable to propose new block for authoring. \
						 Creating proposer failed: {:?}",
						err,
					);
					continue;
				}
			};

			let proposal = match proposer
				.propose(inherent_data, inherent_digest, build_time, None)
				.await
			{
				Ok(x) => x,
				Err(err) => {
					warn!(
						target: "pow",
						"Unable to propose new block for authoring. \
						 Creating proposal failed: {:?}",
						err,
					);
					continue;
				}
			};

			let mut h = proposal.block.header().clone();
			if proposal.block.header().digest().logs().len() == 2 {
				let _ = h.digest_mut().pop();
			}

			let pre_hash = h.hash();
			let build = MiningBuild::<Block, Algorithm, C, _> {
				metadata: MiningMetadata {
					best_hash,
					pre_hash,
					pre_runtime: pre_runtime.clone(),
					difficulty,
				},
				proposal,
			};

			worker.on_build(build);
		}
	};

	(worker_ret, task)
}

