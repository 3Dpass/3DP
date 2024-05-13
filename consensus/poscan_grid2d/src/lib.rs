use std::marker::PhantomData;
use std::fmt::Debug;
use std::sync::Arc;
use std::convert::TryInto;
use parity_scale_codec::{Decode, Encode};
use sc_consensus_poscan::{Error, PoscanData, PoscanDataV1, PoscanDataV2, PowAlgorithm};
use sha3::{Digest, Sha3_256};
use sp_blockchain::HeaderBackend;
use sc_client_api::BlockBackend;
use sp_api::{HeaderT, ProvideRuntimeApi, Core, RuntimeVersion};
use sp_consensus_poscan::{Seal as RawSeal, SCALE_DIFF_SINCE, SCALE_DIFF_BY, CONS_V2_SPEC_VER};
use sp_consensus_poscan::{DifficultyApi, PoscanApi};
use sp_core::{H256, U256, crypto::Pair, hashing::blake2_256, ByteArray};
use sp_runtime::generic::BlockId;
use sp_runtime::traits::{Block as BlockT, Member};
use sp_runtime::scale_info::TypeInfo;
use sp_runtime::traits::SaturatedConversion;
use sc_consensus_poscan::app;
use sp_consensus_poscan::{POSCAN_ALGO_GRID2D_V3_1, POSCAN_ALGO_GRID2D_V3A, REJECT_OLD_ALGO_SINCE};
use poscan_algo::get_obj_hashes;
use log::*;
use randomx_rs::{RandomXFlag, RandomXCache, RandomXVM, RandomXError};

/// Determine whether the given hash satisfies the given difficulty.
/// The test is done by multiplying the two together. If the product
/// overflows the bounds of U256, then the product (and thus the hash)
/// was too high.
pub fn hash_meets_difficulty(hash: &H256, difficulty: U256) -> bool {
	let num_hash = U256::from(&hash[..]);
	let (_, overflowed) = num_hash.overflowing_mul(difficulty);

	!overflowed
}

/// A Seal struct that will be encoded to a Vec<u8> as used as the
/// `RawSeal` type.
#[derive(Clone, PartialEq, Eq, Encode, Decode, Debug)]
pub struct SealV1 {
	pub difficulty: U256,
	pub work: H256,
	pub poscan_hash: H256,
	pub signature: app::Signature,
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, Debug)]
pub struct SealV2 {
	pub difficulty: U256,
	pub work: H256,
	pub poscan_hash: H256,
	pub orig_hash: H256,
	pub hist_hash: H256,
	pub signature: app::Signature,
}

/// A not-yet-computed attempt to solve the proof of work. Calling the
/// compute method will compute the hash and return the seal.
#[derive(Clone, PartialEq, Eq, Encode, Decode, Debug)]
pub struct ComputeV1 {
	pub difficulty: U256,
	pub pre_hash: H256,
	pub poscan_hash: H256,
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, Debug)]
pub struct ComputeV2 {
	pub difficulty: U256,
	pub pre_hash: H256,
	pub poscan_hash: H256,
	pub orig_hash: H256,
	pub hist_hash: H256,
}

// #[derive(Clone, PartialEq, Eq, Encode, Decode, Debug)]
// pub enum Compute {
// 	ComputeV1(ComputeV1),
// 	ComputeV2(ComputeV2),
// }

impl ComputeV1 {
	pub fn seal(&self, signature: app::Signature) -> SealV1 {
		let work = self.get_work();

		SealV1 {
			difficulty: self.difficulty,
			work,
			poscan_hash: self.poscan_hash,
			signature,
		}
	}

	pub fn get_work(&self) -> H256 {
		let buf = self.encode();
		H256::from_slice(Sha3_256::digest(&buf).as_slice())
	}

	fn signing_message(&self) -> [u8; 32] {
		let calculation =
			ComputeV1 {
				difficulty: self.difficulty,
				pre_hash: self.pre_hash,
				poscan_hash: self.poscan_hash,
			}.encode();

		blake2_256(&calculation)
	}

	pub fn sign(&self, pair: &app::Pair) -> app::Signature {
		let hash = self.signing_message();
		pair.sign(&hash[..])
	}

	pub fn verify(
		&self,
		signature: &app::Signature,
		public: &app::Public,
	) -> bool {
		let hash = self.signing_message();
		app::Pair::verify(
			signature,
			&hash[..],
			public,
		)
	}

}

impl ComputeV2 {
	pub fn seal(&self, signature: app::Signature) -> SealV2 {
		let work = self.get_work();
		SealV2 {
			difficulty: self.difficulty,
			work,
			poscan_hash: self.poscan_hash,
			orig_hash: self.orig_hash,
			hist_hash: self.hist_hash,
			signature,
		}
	}

	pub fn get_work(&self) -> H256 {
		let buf = self.encode();
		H256::from_slice(Sha3_256::digest(&buf).as_slice())
	}

	fn signing_message(&self) -> [u8; 32] {
		let calculation =
				ComputeV2 {
					difficulty: self.difficulty,
					pre_hash: self.pre_hash,
					poscan_hash: self.poscan_hash,
					orig_hash: self.orig_hash,
					hist_hash: self.hist_hash,
				}.encode();

		blake2_256(&calculation)
	}

	pub fn sign(&self, pair: &app::Pair) -> app::Signature {
		let hash = self.signing_message();
		pair.sign(&hash[..])
	}

	pub fn verify(
		&self,
		signature: &app::Signature,
		public: &app::Public,
	) -> bool {
		let hash = self.signing_message();
		app::Pair::verify(
			signature,
			&hash[..],
			public,
		)
	}

}


#[derive(Clone, PartialEq, Eq, Encode, Decode, Debug)]
pub struct DoubleHash {
	pub pre_hash: H256,
	pub obj_hash: H256,
}

impl DoubleHash {
	pub fn calc_hash(self) -> H256 {
		H256::from_slice(Sha3_256::digest(&self.encode()[..]).as_slice())
	}

	pub fn calc_hash_randomx(self) -> Result<H256, RandomXError> {
		Ok(H256::from_slice(&randomx(&self.encode()[..])?.encode()))
	}
}

pub struct PoscanAlgorithm<Block, C, AccountId, BlockNumber> {
	client: Arc<C>,
	_phantom: PhantomData<(Block, AccountId, BlockNumber)>,
}

impl<Block, C, AccountId, BlockNumber> PoscanAlgorithm<Block, C, AccountId, BlockNumber> {
	pub fn new(client: Arc<C>) -> Self {
		Self { client, _phantom: PhantomData }
	}
}

impl<Block, C, AccountId, BlockNumber> Clone for PoscanAlgorithm<Block, C, AccountId, BlockNumber> {
	fn clone(&self) -> Self {
		Self::new(self.client.clone())
	}
}

impl<B: BlockT<Hash = H256>, C, AccountId, BlockNumber> PowAlgorithm<B> for PoscanAlgorithm<B, C, AccountId, BlockNumber>
where
	C: ProvideRuntimeApi<B> + HeaderBackend<B> + BlockBackend<B>,
	C::Api: DifficultyApi<B, U256>,
	C::Api: PoscanApi<B, AccountId, BlockNumber>,
	C::Api: Core<B>,
	AccountId: Clone + Eq + Sync + Send + Debug + Encode + Decode + TypeInfo + Member,
	BlockNumber: Clone + Eq + Sync + Send + Debug + Encode + Decode + TypeInfo + Member + Into<u32>,
{
	type Difficulty = U256;

	fn difficulty(&self, parent: B::Hash) -> Result<Self::Difficulty, Error<B>> {
		let parent_id = BlockId::<B>::hash(parent);
		let parent_num = self.client.block_number_from_id(&parent_id).unwrap().unwrap();
		self.client
			.runtime_api()
			.difficulty(&parent_id)
			.map(|d| if parent_num >= SCALE_DIFF_SINCE.into() { d / SCALE_DIFF_BY } else { d })
			.map_err(|err| {
				sc_consensus_poscan::Error::Environment(format!(
					"Fetching difficulty from runtime failed: {:?}",
					err
				))
			})
	}

	fn verify(
		&self,
		parent: &H256,
		pre_hash: &H256,
		pre_digest: Option<&[u8]>,
		seal: &RawSeal,
		difficulty: Self::Difficulty,
		poscan_data: &PoscanData,
	) -> Result<bool, Error<B>> {
		let parent_id = BlockId::<B>::hash(*parent);

		let ver = self.client
			.runtime_api()
			.version(&parent_id)
			// .map(|d| if parent_num >= SCALE_DIFF_SINCE.into() { d / SCALE_DIFF_BY } else { d })
			.map_err(|err| {
				sc_consensus_poscan::Error::<B>::Environment(format!(
					">>> get version call failed: {:?}",
					err
				))
			})?;

		match poscan_data {
			PoscanData::V1(psdata) if ver.spec_version < CONS_V2_SPEC_VER =>
				self.verify_v1(ver, parent, pre_hash, pre_digest, seal, difficulty, psdata),
			PoscanData::V2(psdata) if ver.spec_version >= CONS_V2_SPEC_VER =>
				self.verify_v2(ver, parent, pre_hash, pre_digest, seal, difficulty, psdata),
			_ => Err(Error::<B>::Environment(format!(">>> Invalid poscan data for spec_ver {}", ver.spec_version))),
		}
	}
}

impl<B: BlockT<Hash = H256>, C, AccountId, BlockNumber> PoscanAlgorithm<B, C, AccountId, BlockNumber>
	where
 	C: ProvideRuntimeApi<B> + HeaderBackend<B> + BlockBackend<B>,
	C::Api: DifficultyApi<B, U256>,
	C::Api: PoscanApi<B, AccountId, BlockNumber>,
	C::Api: Core<B>,
	AccountId: Clone + Eq + Sync + Send + Debug + Encode + Decode + TypeInfo + Member,
	BlockNumber: Clone + Eq + Sync + Send + Debug + Encode + Decode + TypeInfo + Member + Into<u32>,
{
	fn verify_v1(
		&self,
		ver: RuntimeVersion,
		parent: &H256,
		pre_hash: &H256,
		pre_digest: Option<&[u8]>,
		seal: &RawSeal,
		difficulty: U256,
		poscan_data: &PoscanDataV1,
	) -> Result<bool, Error<B>>
	{
		// Try to construct a seal object by decoding the raw seal given
		let seal = match SealV1::decode(&mut &seal[..]) {
			Ok(seal) => seal,
			Err(_) => {
				info!(">>> verify: no seal");
				return Ok(false)
			},
		};

		let parent_id = BlockId::<B>::hash(*parent);
		let parent_num = self.client.block_number_from_id(&parent_id).unwrap().unwrap();
		if parent_num >= REJECT_OLD_ALGO_SINCE.into() {
			if poscan_data.hashes.len() == 0 {
				info!(">>> verify: no poscan hashes");
				return Ok(false);
			}

			let dh = DoubleHash { pre_hash: *pre_hash, obj_hash: poscan_data.hashes[0] };
			if dh.calc_hash() != seal.poscan_hash {
				info!(">>> verify: poscan hash doesnt equal to seal one");
				return Ok(false);
			}
		}

		let algo_id = poscan_data.alg_id.clone();
		let obj = poscan_data.obj.clone().to_vec();
		let hashes = poscan_data.hashes.clone();

		// let ver = self.client
		// 	.runtime_api()
		// 	.version(&parent_id)
		// 	// .map(|d| if parent_num >= SCALE_DIFF_SINCE.into() { d / SCALE_DIFF_BY } else { d })
		// 	.map_err(|err| {
		// 		sc_consensus_poscan::Error::<B>::Environment(format!(
		// 			">>> get version call failed: {:?}",
		// 			err
		// 		))
		// 	})?;

		if ver.spec_version >= 123 {
			info!("poscan: start validate");

			let is_valid = self.client
				.runtime_api()
				.check_object(&parent_id, &algo_id, &obj, &hashes)
				.map_err(|err| {
					sc_consensus_poscan::Error::<B>::Environment(format!(
						">>> check_object call failed: {:?}",
						err
					))
				})?;

			if !is_valid {
				info!(">>> object is not valid (runtime)");
				return Ok(false)
			} else {
				info!(">>> object is valid (runtime)");
			}

			let is_valid = poscan_algo::check_obj(&algo_id, &obj, &hashes);
			info!(">>> poscan: end validate");

			if !is_valid {
				info!(">>> object is not valid");
				return Ok(false)
			} else {
				info!(">>> object is valid");
			}
		}

		// See whether the hash meets the difficulty requirement. If not, fail fast.
		if !hash_meets_difficulty(&seal.work, difficulty) {
			info!(">>> verify: hash_meets_difficulty - false");
			info!(">>> work:{} poscan_hash:{} difficulty: {}", &seal.work, &seal.poscan_hash, difficulty);
			return Ok(false);
		}

		// Make sure the provided work actually comes from the correct pre_hash
		let compute = ComputeV1 {
			difficulty,
			pre_hash: *pre_hash,
			poscan_hash: seal.poscan_hash,
		};

		if compute.seal(seal.signature.clone()) != seal {
			info!(">>> verify: compute.compute() != seal");
			return Ok(false);
		}

		let pre_digest = match pre_digest {
			Some(pre_digest) => pre_digest,
			None => {
				info!(">>> verify: no pre_digest");
				return Ok(false)
			},
		};

		let author = match app::Public::decode(&mut &pre_digest[..]) {
			Ok(author) => author,
			Err(_) => {
				info!(">>> verify: decode author failed");
				return Ok(false)
			},
		};


		if !compute.verify(&seal.signature, &author) {
			// use sp_core::Public;
			// info!(">>> pre_hash: {:x?}", &compute.pre_hash);
			//info!(">>> seal.difficulty: {}", &seal.difficulty);
			//info!(">>> seal.work: {}", &seal.work);
			// info!(">>> seal.poscan_hash: {}", &seal.poscan_hash);
			// info!(">>> seal signature is {:x?}", &seal.signature.to_vec());

			info!(">>> verify: miner signature is invalid");
			info!(">>> verify: miner author is {:x?}", &author.to_raw_vec());
			return Ok(false)
		}

		let h =
			if poscan_data.alg_id == POSCAN_ALGO_GRID2D_V3_1
				|| poscan_data.alg_id == POSCAN_ALGO_GRID2D_V3A {
				pre_hash
			} else {
				parent
			};

		let parent_id = BlockId::<B>::hash(*parent);
		let parent_num = self.client.block_number_from_id(&parent_id).unwrap().unwrap();
		let patch_rot = parent_num >= REJECT_OLD_ALGO_SINCE.into();

		let hashes = get_obj_hashes(&poscan_data.alg_id, &poscan_data.obj, h, patch_rot);
		if hashes != poscan_data.hashes {
			info!(">>> verify: hashes != poscan_data.hashes");
			return Ok(false)
		}

		Ok(true)
	}

	fn verify_v2(
		&self,
		ver: RuntimeVersion,
		parent: &H256,
		pre_hash: &H256,
		pre_digest: Option<&[u8]>,
		seal: &RawSeal,
		difficulty: U256,
		poscan_data: &PoscanDataV2,
	) -> Result<bool, Error<B>>
	{
		// Try to construct a seal object by decoding the raw seal given
		let seal = match SealV2::decode(&mut &seal[..]) {
			Ok(seal) => seal,
			Err(_) => {
				info!(">>> verify: no seal");
				return Ok(false)
			},
		};

		let parent_id = BlockId::<B>::hash(*parent);
		let parent_num = self.client.block_number_from_id(&parent_id).unwrap().unwrap();
		if parent_num >= REJECT_OLD_ALGO_SINCE.into() {
			if poscan_data.hashes.len() == 0 {
				info!(">>> verify: no poscan hashes");
				return Ok(false);
			}

			let dh = DoubleHash { pre_hash: *pre_hash, obj_hash: poscan_data.hashes[0] };
			let rndx = dh.calc_hash_randomx()
				.map_err(|e| Error::Other(format!("{:#?}", e)))?;
			if rndx != seal.poscan_hash {
				info!(">>> verify: poscan hash doesnt equal to seal one");
				return Ok(false);
			}
		}

		let algo_id = poscan_data.alg_id.clone();
		let obj = poscan_data.obj.clone().to_vec();
		let hashes = poscan_data.hashes.clone();

		if ver.spec_version >= 123 {
			info!("poscan: start validate");

			let is_valid = self.client
				.runtime_api()
				.check_object(&parent_id, &algo_id, &obj, &hashes)
				.map_err(|err| {
					sc_consensus_poscan::Error::<B>::Environment(format!(
						">>> check_object call failed: {:?}",
						err
					))
				})?;

			if !is_valid {
				info!(">>> object is not valid (runtime)");
				return Ok(false)
			}
			else {
				info!(">>> object is valid (runtime)");
			}

			let is_valid = poscan_algo::check_obj(&algo_id, &obj, &hashes);
			info!(">>> poscan: end validate");

			if !is_valid {
				info!(">>> object is not valid");
				return Ok(false)
			} else {
				info!(">>> object is valid");
			}
		}

		// See whether the hash meets the difficulty requirement. If not, fail fast.
		if !hash_meets_difficulty(&seal.work, difficulty) {
			info!(">>> verify: hash_meets_difficulty - false");
			info!(">>> work:{} poscan_hash:{} difficulty: {}", &seal.work, &seal.poscan_hash, difficulty);
			return Ok(false);
		}

		let obj_orig_hashes = if ver.spec_version >= CONS_V2_SPEC_VER {
			self.client
				.runtime_api()
				.get_obj_hashes_wasm(&parent_id, &poscan_data.alg_id, &poscan_data.obj, &H256::default(), 10, false)
				.map_err(|err| {
					sc_consensus_poscan::Error::<B>::Environment(format!(
						">>> get_obj_hashes_wasm: {:?}",
						err
					))
				})?
		}
		else {
			get_obj_hashes(&poscan_data.alg_id, &poscan_data.obj, &H256::default(), false)
		};

		if obj_orig_hashes.len() == 0 {
			info!(">>> verify: obj_pre_hashes.len() == 0");
			return Ok(false)
		}

		if obj_orig_hashes != poscan_data.orig_hashes {
			info!(">>> verify: obj_orig_hashes != poscan_data.orig_hashes");
			return Ok(false)
		}

		let v = obj_orig_hashes[0].encode();
		let mut buf = pre_hash.encode();
		buf.append(v.clone().as_mut());

		let rotation_hash = randomx(&buf)
			.map_err(|e| Error::Other(format!("{:#?}", e)))?;

		let hist_hash = calc_hist(self.client.clone(), &rotation_hash, &parent_id);

		if seal.hist_hash != hist_hash {
			info!(">>> verify: seal.hist_hash != hist_hash");
			return Ok(false);
		}

		let rndx = randomx(&obj_orig_hashes[0].0.as_slice())
			.map_err(|e| Error::Other(format!("{:#?}", e)))?;
		if seal.orig_hash != rndx {
			info!(">>> verify: seal.orig_hash != randomx(orig_hashes[0])");
			return Ok(false);
		}

		let compute = ComputeV2 {
			difficulty,
			pre_hash: *pre_hash,
			poscan_hash: seal.poscan_hash,
			orig_hash: seal.orig_hash,
			hist_hash: seal.hist_hash,
		};

		if compute.seal(seal.signature.clone()) != seal {
			info!(">>> verify: compute.compute() != seal");
			return Ok(false);
		}

		let pre_digest = match pre_digest {
			Some(pre_digest) => pre_digest,
			None => {
				info!(">>> verify: no pre_digest");
				return Ok(false)
			},
		};

		let author = match app::Public::decode(&mut &pre_digest[..]) {
			Ok(author) => author,
			Err(_) => {
				info!(">>> verify: decode author failed");
				return Ok(false)
			},
		};


		if !compute.verify(&seal.signature, &author) {
			info!(">>> verify: miner signature is invalid");
			return Ok(false)
		}

		// TODO:
		let _h =
			if poscan_data.alg_id == POSCAN_ALGO_GRID2D_V3_1
				|| poscan_data.alg_id == POSCAN_ALGO_GRID2D_V3A {
				pre_hash
			}
			else {
				parent
			};

		let parent_id = BlockId::<B>::hash(*parent);
		let parent_num = self.client.block_number_from_id(&parent_id).unwrap().unwrap();
		let patch_rot = parent_num >= REJECT_OLD_ALGO_SINCE.into();

		let hashes = if ver.spec_version >= CONS_V2_SPEC_VER {
			self.client
				.runtime_api()
				.get_obj_hashes_wasm(&parent_id, &poscan_data.alg_id, &poscan_data.obj, &rotation_hash, 10, patch_rot)
				.map_err(|err| {
					sc_consensus_poscan::Error::<B>::Environment(format!(
						">>> get_obj_hashes_wasm: {:?}",
						err
					))
				})?
		}
		else {
			get_obj_hashes(&poscan_data.alg_id, &poscan_data.obj, &rotation_hash, patch_rot)
		};

		if hashes != poscan_data.hashes {
			info!(">>> verify: hashes != poscan_data.hashes");
			return Ok(false)
		}

		use std::collections::HashSet;

		let ha = poscan_data.hashes.iter().collect::<HashSet<_>>();
		let hb = obj_orig_hashes.iter().collect::<HashSet<_>>();
		if ha.intersection(&hb).collect::<Vec<_>>().len() > 0 {
			info!(">>> verify: some hashes after rotation are in original hashes");
			return Ok(false)
		}

		Ok(true)
	}
}


pub fn calc_hist<C, B>(client: Arc<C>, pre_hash: &H256, parent_block: &BlockId::<B>) -> H256
where
	C: ProvideRuntimeApi<B> + HeaderBackend<B> + BlockBackend<B>,
	C::Api: DifficultyApi<B, U256>,
	B: BlockT,
{
	let mut selected_blocks = Vec::<u32>::new();

	let n_blocks: u32 = client
		.runtime_api()
		.hist_steps(&parent_block)
		.unwrap_or_default();

	let parent_num = client.block_number_from_id(parent_block).unwrap().unwrap();
	let mut hh = *pre_hash;
	let num_blocks = parent_num.saturated_into::<u32>();
	if num_blocks > 1 {
		for _ in 0..n_blocks {
			let n = u32::from_le_bytes(hh[..4].try_into().unwrap());
			let number: u32 = n % (num_blocks - 1) + 1;

			let digest = client
				.header(BlockId::Number(number.into()))
				.and_then(|maybe_header| match maybe_header {
					Some(header) => Ok(Some(header.digest().clone())),
					None => Ok(None),
				});

			if let Ok(None) = digest {
				debug!("--- Select: no digest");
				continue
			}

			let block_data: Vec<u8>;
			let block_id = BlockId::Number(number.into());
			match client.block(&block_id) {
				Ok(maybe_prev_block) => {
					match maybe_prev_block {
						Some(signed_block) => {
							block_data = signed_block.block.encode();
						},
						_ => continue,
					}
				},
				_ => continue,
			}

			let mut v = block_data.encode();
			let mut buf = hh.encode();
			buf.append(&mut v);
			hh = H256::from_slice(blake2_256(&buf).as_slice());
			selected_blocks.push(number);
		}
		debug!("--- Selected  blocks: {}",
			selected_blocks.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")
		);
	}
	hh
}

pub fn randomx(data: &[u8]) -> Result<H256, RandomXError> {
	let flags = RandomXFlag::get_recommended_flags();
	let cache = RandomXCache::new(flags, &data)?;
	let vm = RandomXVM::new(flags, Some(cache), None)?;

	vm.calculate_hash(&data).map(|res| H256::from_slice(&res))
}