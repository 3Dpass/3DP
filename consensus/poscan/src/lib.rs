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

mod worker;

pub use crate::worker::{MiningWorker, MiningMetadata, MiningBuild};

use std::{
	sync::Arc, any::Any, borrow::Cow, collections::HashMap, marker::PhantomData,
	cmp::Ordering, time::Duration,
};
use futures::{prelude::*, future::Either};
use parking_lot::Mutex;
use sc_client_api::{BlockOf, backend::AuxStore, BlockchainEvents, BlockBackend};
use sp_blockchain::{HeaderBackend, ProvideCache, well_known_cache_keys::Id as CacheKeyId};
use sp_block_builder::BlockBuilder as BlockBuilderApi;
use sp_runtime::{Justification, RuntimeString};
use sp_runtime::generic::{BlockId, Digest, DigestItem};
use sp_runtime::traits::{Block as BlockT, Header as HeaderT};
use sp_api::ProvideRuntimeApi;
use sp_consensus_poscan::{Seal, TotalDifficulty, POSCAN_ENGINE_ID};
use sp_inherents::{InherentDataProviders, InherentData};
use sp_consensus::{
	BlockImportParams, BlockOrigin, ForkChoiceStrategy, SyncOracle, Environment, Proposer,
	SelectChain, Error as ConsensusError, CanAuthorWith, RecordProof, BlockImport,
	BlockCheckParams, ImportResult,
};
use sp_consensus::import_queue::{
	BoxBlockImport, BasicQueue, Verifier, BoxJustificationImport,
};
use codec::{Encode, Decode};
use prometheus_endpoint::Registry;
use sc_client_api;
use log::*;
use sp_core::{H256, U256};
use sp_timestamp::{InherentError as TIError, TimestampInherentData};

use crate::worker::UntilImportedOrTimeout;
use sp_consensus_poscan::{Difficulty, DifficultyApi};

pub mod app {
	use sp_application_crypto::{app_crypto, sr25519};
	use sp_core::crypto::KeyTypeId;

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
	CheckInherents(String),
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
pub struct PoscanData {
	pub hashes: Vec<H256>,
	pub obj: Vec<u8>,
}

#[derive(Clone, Encode, Decode, PartialEq, Eq, Debug)]
pub struct PoscanData2 {
	pub hashes: H256,
	pub obj: u8,
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
		parent: &BlockId<B>,
		pre_hash: &B::Hash,
		pre_digest: Option<&[u8]>,
		seal: &Seal,
		difficulty: Self::Difficulty,
		poscan_data: &PoscanData,
	) -> Result<bool, Error<B>>;
}

/// A block importer for PoW.
pub struct PowBlockImport<B: BlockT, I, C, S, Algorithm, CAW> {
	algorithm: Algorithm,
	inner: I,
	select_chain: S,
	client: Arc<C>,
	inherent_data_providers: sp_inherents::InherentDataProviders,
	check_inherents_after: <<B as BlockT>::Header as HeaderT>::Number,
	can_author_with: CAW,
}

impl<B: BlockT, I: Clone, C, S: Clone, Algorithm: Clone, CAW: Clone> Clone
	for PowBlockImport<B, I, C, S, Algorithm, CAW>
{
	fn clone(&self) -> Self {
		Self {
			algorithm: self.algorithm.clone(),
			inner: self.inner.clone(),
			select_chain: self.select_chain.clone(),
			client: self.client.clone(),
			inherent_data_providers: self.inherent_data_providers.clone(),
			check_inherents_after: self.check_inherents_after.clone(),
			can_author_with: self.can_author_with.clone(),
		}
	}
}

impl<B, I, C, S, Algorithm, CAW> PowBlockImport<B, I, C, S, Algorithm, CAW> where
	B: BlockT,
	I: BlockImport<B, Transaction = sp_api::TransactionFor<C, B>> + Send + Sync,
	I::Error: Into<ConsensusError>,
	C: ProvideRuntimeApi<B> + Send + Sync + HeaderBackend<B> + AuxStore + ProvideCache<B> + BlockOf + BlockBackend<B>,
	C::Api: BlockBuilderApi<B, Error = sp_blockchain::Error>,
	Algorithm: PowAlgorithm<B>,
	CAW: CanAuthorWith<B>,
{
	/// Create a new block import suitable to be used in PoW
	pub fn new(
		inner: I,
		client: Arc<C>,
		algorithm: Algorithm,
		check_inherents_after: <<B as BlockT>::Header as HeaderT>::Number,
		select_chain: S,
		inherent_data_providers: sp_inherents::InherentDataProviders,
		can_author_with: CAW,
	) -> Self {
		Self {
			inner,
			client,
			algorithm,
			check_inherents_after,
			select_chain,
			inherent_data_providers,
			can_author_with,
		}
	}

	fn check_inherents(
		&self,
		block: B,
		block_id: BlockId<B>,
		inherent_data: InherentData,
		timestamp_now: u64,
	) -> Result<(), Error<B>> {
		const MAX_TIMESTAMP_DRIFT_SECS: u64 = 60;

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

		let inherent_res = self.client.runtime_api().check_inherents(
			&block_id,
			block,
			inherent_data,
		).map_err(Error::Client)?;

		if !inherent_res.ok() {
			inherent_res
				.into_errors()
				.try_for_each(|(i, e)| match TIError::try_from(&i, &e) {
					Some(TIError::ValidAtTimestamp(timestamp)) => {
						if timestamp > timestamp_now + MAX_TIMESTAMP_DRIFT_SECS {
							return Err(Error::TooFarInFuture);
						}

						Ok(())
					},
					Some(TIError::Other(e)) => Err(Error::Runtime(e)),
					None => Err(Error::CheckInherents(
						self.inherent_data_providers.error_to_string(&i, &e)
					)),
				})
		} else {
			Ok(())
		}
	}
}

impl<B, I, C, S, Algorithm, CAW> BlockImport<B> for PowBlockImport<B, I, C, S, Algorithm, CAW> where
	B: BlockT,
	I: BlockImport<B, Transaction = sp_api::TransactionFor<C, B>> + Send + Sync,
	I::Error: Into<ConsensusError>,
	S: SelectChain<B>,
	C: ProvideRuntimeApi<B> + Send + Sync + HeaderBackend<B> + AuxStore + ProvideCache<B> + BlockOf + BlockBackend<B>,
	C::Api: BlockBuilderApi<B, Error = sp_blockchain::Error>,
	C::Api: DifficultyApi<B, Difficulty>,
	Algorithm: PowAlgorithm<B>,
	Algorithm::Difficulty: From<U256> +'static,
	CAW: CanAuthorWith<B>,
{
	type Error = ConsensusError;
	type Transaction = sp_api::TransactionFor<C, B>;

	fn check_block(
		&mut self,
		block: BlockCheckParams<B>,
	) -> Result<ImportResult, Self::Error> {
		self.inner.check_block(block).map_err(Into::into)
	}

	fn import_block(
		&mut self,
		mut block: BlockImportParams<B, Self::Transaction>,
		new_cache: HashMap<CacheKeyId, Vec<u8>>,
	) -> Result<ImportResult, Self::Error> {
		let best_header = self.select_chain.best_chain()
			.map_err(|e| format!("Fetch best chain failed via select chain: {:?}", e))?;
		let best_hash = best_header.hash();

		let parent_hash = *block.header.parent_hash();
		info!(">>> parent_hash: {:x?}", &parent_hash);
		let best_aux = PowAux::read::<_, B>(self.client.as_ref(), &best_hash)?;
		let mut aux = PowAux::read::<_, B>(self.client.as_ref(), &parent_hash)?;

		if let Some(inner_body) = block.body.take() {
			let inherent_data = self.inherent_data_providers
				.create_inherent_data().map_err(|e| e.into_string())?;
			let timestamp_now = inherent_data.timestamp_inherent_data().map_err(|e| e.into_string())?;

			let check_block = B::new(block.header.clone(), inner_body);

			self.check_inherents(
				check_block.clone(),
				BlockId::Hash(parent_hash),
				inherent_data,
				timestamp_now
			)?;

			block.body = Some(check_block.deconstruct().1);
		}

		let ll = block.post_digests.len();
		info!(">>> block.post_digests.len() = {}", &ll);

		let mut dpos: usize = 0;
		let pre_digest;
		if ll == 4 {
			pre_digest = fetch_seal::<B>(block.post_digests.get(dpos), block.header.hash())?;
			info!(">>> hz len: {}", pre_digest.len());
			info!(">>> hz: {:x?}", pre_digest);
			dpos += 1;
		}
		else {
			pre_digest = find_pre_digest::<B>(&block.header)?.unwrap(); // .as_ref().map(|v| &v[..]);

		}


		let inner_seal = fetch_seal::<B>(block.post_digests.get(dpos), block.header.hash())?;
		info!(">>> seal from header len: {}", inner_seal.len());

		let pscan_hashes = fetch_seal::<B>(block.post_digests.get(dpos + 1), block.header.hash())?;
		info!(">>> seal hashes: {:x?}", &pscan_hashes);
		info!(">>> seal hashes len: {:x?}", pscan_hashes.len());

		let pscan_obj = fetch_seal::<B>(block.post_digests.get(dpos + 2), block.header.hash())?;
		info!(">>> pscan_obj len: {}", pscan_obj.len());

		// let hz = fetch_seal::<B>(block.post_digests.get(3), block.header.hash())?;
		// info!(">>> hz len: {}", hz.len());

		let hs: Vec<H256> = pscan_hashes.chunks(32).map(|h| H256::from_slice(h)).collect();
		for i in hs.iter() {
			info!(">>> hashe: {}", i.to_string());
		}

		let psdata = PoscanData{ hashes: hs.clone(), obj: pscan_obj };

		let intermediate = block.take_intermediate::<PowIntermediate::<Algorithm::Difficulty>>(
			INTERMEDIATE_KEY
		)?;

		// let obj = (*intermediate).obj;

		let difficulty = match intermediate.difficulty {
			Some(difficulty) => difficulty,
			None => self.algorithm.difficulty(parent_hash)?,
		};

		let pre_hash = block.header.hash();
		info!("--- pre_hash: {:x?}", &pre_hash);

		// let pre_digest = find_pre_digest::<B>(&block.header)?;
		if !self.algorithm.verify(
			&BlockId::hash(parent_hash),
			&pre_hash,
			Some(pre_digest.as_slice()),
			&inner_seal,
			difficulty,
			&psdata,
		)? {
			error!(">>> InvalidSeal after self.algorithm.verify");
			return Err(Error::<B>::InvalidSeal.into())
		}

		let mut prev_hash = Some(parent_hash);
		while prev_hash.is_some() {
			let num: BlockId<B> = BlockId::hash(prev_hash.unwrap());
			match self.client.block(&num) {
				Ok(maybe_prev_block) => {
					match maybe_prev_block {
						Some(signed_block) => {
							let h = signed_block.block.header();
							let n = h.digest().logs().len();
							info!(">>> parent hash digest len: {}", n);
							if n >= 3 {
								let di = h.digest().logs()[2].clone();
								if let DigestItem::Other(v) = di {
									let hashes: Vec<H256> = v.chunks(32).map(|h| H256::from_slice(h)).collect();
									for hh in hashes[..1].iter() {
										if hs[..1].contains(hh) {
											info!(">>>>>> duplicated hash found");
											info!(">>>>>> {:x?}", hs);
											info!(">>>>>> {:x?}", hashes);
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

		self.inner.import_block(block, new_cache).map_err(Into::into)
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
	) -> Result<(B::Header, Vec<DigestItem<B::Hash>>), Error<B>> where
		Algorithm: PowAlgorithm<B>,
	{
		let hash = header.hash();
		let mut digests: Vec<DigestItem<B::Hash>> = Vec::new();
		let n = header.digest().logs().len();
		info!(">>> digests len {}", n);
		let mut pre_run: Option<DigestItem<_>> = None;
		for _ in 0..n {
			match header.digest_mut().pop() {
				Some(DigestItem::Seal(id, seal)) => {
					if id == POSCAN_ENGINE_ID {
						info!(">>> Header in check_header: Seal");
						digests.push(DigestItem::Seal(id, seal.clone()))
					} else {
						return Err(Error::WrongEngine(id))
					}
				},
				Some(DigestItem::Other(item)) => {
					info!(">>> Header in check_header: Other");
					digests.push(DigestItem::Other(item.clone()))
				},
				Some(DigestItem::PreRuntime(id, pre_runtime)) => {
					info!(">>> Header in check_header: PreRuntime");
					pre_run = Some(DigestItem::PreRuntime(id, pre_runtime.clone()));
					digests.push(DigestItem::PreRuntime(id, pre_runtime.clone()))
				},
				_ => {
					error!(">>> Header unsealed in check_header");
					return Err(Error::HeaderUnsealed(hash))
				},
			};
		}

		if let Some(r) = pre_run {
			header.digest_mut().push(r);
		}

		// let pre_hash = header.hash();
		//
		// if !self.algorithm.preliminary_verify(&pre_hash, &inner_seal)?.unwrap_or(true) {
		// 	return Err(Error::FailedPreliminaryVerify);
		// }

		Ok((header, digests))
	}
}

impl<B: BlockT, Algorithm> Verifier<B> for PowVerifier<B, Algorithm> where
	Algorithm: PowAlgorithm<B> + Send + Sync,
	Algorithm::Difficulty: 'static,
{
	fn verify(
		&mut self,
		origin: BlockOrigin,
		header: B::Header,
		justification: Option<Justification>,
		body: Option<Vec<B::Extrinsic>>,
	) -> Result<(BlockImportParams<B, ()>, Option<Vec<(CacheKeyId, Vec<u8>)>>), String> {
		let (checked_header, items) = self.check_header(header)?;
		let _pre_runtime = items[3].clone();
		let seal = items[2].clone();
		let poscan_hashes = items[1].clone();
		let poscan_obj = items[0].clone();

		let intermediate = PowIntermediate::<Algorithm::Difficulty> {
			difficulty: None,
		};

		let mut import_block = BlockImportParams::new(origin, checked_header);

		match poscan_obj {
			DigestItem::Other(ref s) => info!(">>> In verify: pscan_obj len: {}", s.len()),
			_ => {}
		}

		import_block.post_digests.push(seal);
		import_block.post_digests.push(poscan_hashes);
		import_block.post_digests.push(poscan_obj);
		import_block.body = body;
		import_block.justification = justification;
		import_block.intermediates.insert(
			Cow::from(INTERMEDIATE_KEY),
			Box::new(intermediate) as Box<dyn Any>
		);
		// import_block.post_hash = Some(post_hash);

		Ok((import_block, None))
	}
}

/// Register the PoW inherent data provider, if not registered already.
pub fn register_pow_inherent_data_provider(
	inherent_data_providers: &InherentDataProviders,
) -> Result<(), sp_consensus::Error> {
	if !inherent_data_providers.has_provider(&sp_timestamp::INHERENT_IDENTIFIER) {
		inherent_data_providers
			.register_provider(sp_timestamp::InherentDataProvider)
			.map_err(Into::into)
			.map_err(sp_consensus::Error::InherentData)
	} else {
		Ok(())
	}
}

/// The PoW import queue type.
pub type PowImportQueue<B, Transaction> = BasicQueue<B, Transaction>;

/// Import queue for PoW engine.
pub fn import_queue<B, Transaction, Algorithm>(
	block_import: BoxBlockImport<B, Transaction>,
	justification_import: Option<BoxJustificationImport<B>>,
	algorithm: Algorithm,
	inherent_data_providers: InherentDataProviders,
	spawner: &impl sp_core::traits::SpawnNamed,
	registry: Option<&Registry>,
) -> Result<
	PowImportQueue<B, Transaction>,
	sp_consensus::Error
> where
	B: BlockT,
	Transaction: Send + Sync + 'static,
	Algorithm: PowAlgorithm<B> + Clone + Send + Sync + 'static,
{
	register_pow_inherent_data_provider(&inherent_data_providers)?;

	let verifier = PowVerifier::new(algorithm);

	Ok(BasicQueue::new(
		verifier,
		block_import,
		justification_import,
		spawner,
		registry,
	))
}

/// Start the mining worker for PoW. This function provides the necessary helper functions that can
/// be used to implement a miner. However, it does not do the CPU-intensive mining itself.
///
/// Two values are returned -- a worker, which contains functions that allows querying the current
/// mining metadata and submitting mined blocks, and a future, which must be polled to fill in
/// information in the worker.
///
/// `pre_runtime` is a parameter that allows a custom additional pre-runtime digest to be inserted
/// for blocks being built. This can encode authorship information, or just be a graffiti.
pub fn start_mining_worker<Block, C, S, Algorithm, E, SO, CAW>(
	block_import: BoxBlockImport<Block, sp_api::TransactionFor<C, Block>>,
	client: Arc<C>,
	select_chain: S,
	algorithm: Algorithm,
	mut env: E,
	mut sync_oracle: SO,
	pre_runtime: Option<Vec<u8>>,
	inherent_data_providers: sp_inherents::InherentDataProviders,
	timeout: Duration,
	build_time: Duration,
	can_author_with: CAW,
) -> (Arc<Mutex<MiningWorker<Block, Algorithm, C>>>, impl Future<Output = ()>) where
	Block: BlockT,
	C: ProvideRuntimeApi<Block> + BlockchainEvents<Block> + 'static,
	C::Api: DifficultyApi<Block, Difficulty>,
	S: SelectChain<Block> + 'static,
	Algorithm: PowAlgorithm<Block> + Clone,
	Algorithm::Difficulty: From<U256> + 'static,
	E: Environment<Block> + Send + Sync + 'static,
	E::Error: std::fmt::Debug,
	E::Proposer: Proposer<Block, Transaction = sp_api::TransactionFor<C, Block>>,
	SO: SyncOracle + Clone + Send + Sync + 'static,
	CAW: CanAuthorWith<Block> + Clone + Send + 'static,
{
	if let Err(_) = register_pow_inherent_data_provider(&inherent_data_providers) {
		warn!("Registering inherent data provider for timestamp failed");
	}

	let timer = UntilImportedOrTimeout::new(client.import_notification_stream(), timeout);
	let worker = Arc::new(Mutex::new(MiningWorker::<Block, Algorithm, C> {
		build: None,
		algorithm: algorithm.clone(),
		block_import,
	}));
	let worker_ret = worker.clone();

	let task = timer.for_each(move |()| {
		let worker = worker.clone();

		if sync_oracle.is_major_syncing() {
			debug!(target: "pow", "Skipping proposal due to sync.");
			worker.lock().on_major_syncing();
			return Either::Left(future::ready(()))
		}

		let best_header = match select_chain.best_chain() {
			Ok(x) => x,
			Err(err) => {
				warn!(
					target: "pow",
					"Unable to pull new block for authoring. \
					 Select best chain error: {:?}",
					err
				);
				return Either::Left(future::ready(()))
			},
		};
		let best_hash = best_header.hash();

		if let Err(err) = can_author_with.can_author_with(&BlockId::Hash(best_hash)) {
			warn!(
				target: "pow",
				"Skipping proposal `can_author_with` returned: {} \
				 Probably a node update is required!",
				err,
			);
			return Either::Left(future::ready(()))
		}

		if worker.lock().best_hash() == Some(best_hash) {
			return Either::Left(future::ready(()))
		}

		// The worker is locked for the duration of the whole proposing period. Within this period,
		// the mining target is outdated and useless anyway.

		let difficulty = match algorithm.difficulty(best_hash) {
			Ok(x) => {
				warn!(
					target: "pow",
					"Difficulty: {:?}",
					x.encode(),
				);
				x
			},
			Err(err) => {
				warn!(
					target: "pow",
					"Unable to propose new block for authoring. \
					 Fetch difficulty failed: {:?}",
					err,
				);
				return Either::Left(future::ready(()))
			},
		};

		let awaiting_proposer = env.init(&best_header);
		let inherent_data = match inherent_data_providers.create_inherent_data() {
			Ok(x) => x,
			Err(err) => {
				warn!(
					target: "pow",
					"Unable to propose new block for authoring. \
					 Creating inherent data failed: {:?}",
					err,
				);
				return Either::Left(future::ready(()))
			},
		};
		let mut inherent_digest = Digest::<Block::Hash>::default();
		if let Some(pre_runtime) = &pre_runtime {
			inherent_digest.push(DigestItem::PreRuntime(POSCAN_ENGINE_ID, pre_runtime.to_vec()));
		}

		let pre_runtime = pre_runtime.clone();

		Either::Right(async move {
			let proposer = match awaiting_proposer.await {
				Ok(x) => x,
				Err(err) => {
					warn!(
						target: "pow",
						"Unable to propose new block for authoring. \
						 Creating proposer failed: {:?}",
						err,
					);
					return
				},
			};

			let proposal = match proposer.propose(
				inherent_data,
				inherent_digest,
				build_time.clone(),
				RecordProof::No,
			).await {
				Ok(x) => x,
				Err(err) => {
					warn!(
						target: "pow",
						"Unable to propose new block for authoring. \
						 Creating proposal failed: {:?}",
						err,
					);
					return
				},
			};

			let build = MiningBuild::<Block, Algorithm, C> {
				metadata: MiningMetadata {
					best_hash,
					pre_hash: proposal.block.header().hash(),
					pre_runtime: pre_runtime.clone(),
					difficulty,
				},
				proposal,
			};

			worker.lock().on_build(build);
		})
	});

	(worker_ret, task)
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
	digest: Option<&DigestItem<B::Hash>>,
	hash: B::Hash,
) -> Result<Vec<u8>, Error<B>> {
	match digest {
		Some(DigestItem::Seal(id, seal)) => {
			if id == &POSCAN_ENGINE_ID {
				info!(">>> Header sealed ok in fetch_seal (Seal)");
				Ok(seal.clone())
			} else {
				return Err(Error::<B>::WrongEngine(*id).into())
			}
		},
		Some(DigestItem::Other(item)) => {
			Ok(item.clone())
		},
		Some(DigestItem::PreRuntime(id, pre_runtime)) => {
			if id == &POSCAN_ENGINE_ID {
				info!(">>> Header sealed ok in fetch_seal (PreRuntime)");
				Ok(pre_runtime.clone())
			}
			else {
				return Err(Error::<B>::WrongEngine(*id).into())
			}
		},
		Some(DigestItem::ChangesTrieRoot(_h)) => {
			info!(">>> DigestItem::ChangesTrieRoot");
			return Err(Error::<B>::HeaderUnsealed(hash).into())
		},
		Some(DigestItem::ChangesTrieSignal(_h)) => {
			info!(">>> DigestItem::ChangesTrieSignal");
			return Err(Error::<B>::HeaderUnsealed(hash).into())
		},
		Some(DigestItem::Consensus(_id, _v)) => {
			info!(">>> DigestItem::Consensus");
			return Err(Error::<B>::HeaderUnsealed(hash).into())
		},
		_ => {
			error!(">>> Header unsealed in fetch_seal");
			return Err(Error::<B>::HeaderUnsealed(hash).into())
		},
	}
}
