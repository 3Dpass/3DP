use parity_scale_codec::{Decode, Encode};
use sc_consensus_poscan::{Error, PoscanData, PowAlgorithm};
use sha3::{Digest, Sha3_256};
use sp_api::ProvideRuntimeApi;
use sp_consensus_poscan::{DifficultyApi, Seal as RawSeal};
use sp_core::{H256, U256};
use sp_runtime::generic::BlockId;
use sp_runtime::traits::Block as BlockT;
use std::sync::Arc;
// use frame_support::sp_runtime::print as prn;
// use frame_support::runtime_print;

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
pub struct Seal {
	pub difficulty: U256,
	pub work: H256,
	// pub nonce: U256,
	pub poscan_hash: H256,
}

/// A not-yet-computed attempt to solve the proof of work. Calling the
/// compute method will compute the hash and return the seal.
#[derive(Clone, PartialEq, Eq, Encode, Decode, Debug)]
pub struct Compute {
	pub difficulty: U256,
	pub pre_hash: H256,
	// pub nonce: U256,
	pub poscan_hash: H256,
}

impl Compute {
	pub fn compute(self) -> Seal {
		let work = H256::from_slice(Sha3_256::digest(&self.encode()[..]).as_slice());

		Seal {
			poscan_hash: self.poscan_hash,
			difficulty: self.difficulty,
			work,
		}
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
}

// /// A minimal PoW algorithm that uses Sha3 hashing.
// /// Difficulty is fixed at 1_000_000
// #[derive(Clone)]
// pub struct MinimalSha3Algorithm;
//
// // Here we implement the general PowAlgorithm trait for our concrete Sha3Algorithm
// impl<B: BlockT<Hash = H256>> PowAlgorithm<B> for MinimalSha3Algorithm {
// 	type Difficulty = U256;
//
// 	fn difficulty(&self, _parent: B::Hash) -> Result<Self::Difficulty, Error<B>> {
// 		// Fixed difficulty hardcoded here
// 		Ok(U256::from(1_000_000))
// 	}
//
// 	fn verify(
// 		&self,
// 		_parent: &BlockId<B>,
// 		pre_hash: &H256,
// 		_pre_digest: Option<&[u8]>,
// 		seal: &RawSeal,
// 		difficulty: Self::Difficulty,
// 		_obj_hashes: &[H256],
// 		_obj: &[u8],
// 	) -> Result<bool, Error<B>> {
// 		// Try to construct a seal object by decoding the raw seal given
// 		let seal = match Seal::decode(&mut &seal[..]) {
// 			Ok(seal) => seal,
// 			Err(_) => return Ok(false),
// 		};
//
// 		// See whether the hash meets the difficulty requirement. If not, fail fast.
// 		if !hash_meets_difficulty(&seal.work, difficulty) {
// 			return Ok(false);
// 		}
//
// 		// Make sure the provided work actually comes from the correct pre_hash
// 		let compute = Compute {
// 			difficulty,
// 			pre_hash: *pre_hash,
// 			nonce: seal.nonce,
// 		};
//
// 		// let _obj = <Module<>::get(&seal.nonce);
//
// 		if compute.compute() != seal {
// 			return Ok(false);
// 		}
//
// 		Ok(true)
// 	}
// }
//
// /// A complete PoW Algorithm that uses Sha3 hashing.
// /// Needs a reference to the client so it can grab the difficulty from the runtime.
// pub struct Sha3Algorithm<C> {
// 	client: Arc<C>,
// }
//
// impl<C> Sha3Algorithm<C> {
// 	pub fn new(client: Arc<C>) -> Self {
// 		Self { client }
// 	}
// }
//
// // Manually implement clone. Deriving doesn't work because
// // it'll derive impl<C: Clone> Clone for Sha3Algorithm<C>. But C in practice isn't Clone.
// impl<C> Clone for Sha3Algorithm<C> {
// 	fn clone(&self) -> Self {
// 		Self::new(self.client.clone())
// 	}
// }
//
// // Here we implement the general PowAlgorithm trait for our concrete Sha3Algorithm
// impl<B: BlockT<Hash = H256>, C> PowAlgorithm<B> for Sha3Algorithm<C>
// where
// 	C: ProvideRuntimeApi<B>,
// 	C::Api: DifficultyApi<B, U256>,
// {
// 	type Difficulty = U256;
//
// 	fn difficulty(&self, parent: B::Hash) -> Result<Self::Difficulty, Error<B>> {
// 		let parent_id = BlockId::<B>::hash(parent);
// 		self.client
// 			.runtime_api()
// 			.difficulty(&parent_id)
// 			.map_err(|err| {
// 				sc_consensus_poscan::Error::Environment(format!(
// 					"Fetching difficulty from runtime failed: {:?}",
// 					err
// 				))
// 			})
// 	}
//
// 	fn verify(
// 		&self,
// 		_parent: &BlockId<B>,
// 		pre_hash: &H256,
// 		_pre_digest: Option<&[u8]>,
// 		seal: &RawSeal,
// 		difficulty: Self::Difficulty,
// 		_obj_hashes: &[H256],
// 		_obj: &[u8],
// 	) -> Result<bool, Error<B>> {
// 		// Try to construct a seal object by decoding the raw seal given
// 		let seal = match Seal::decode(&mut &seal[..]) {
// 			Ok(seal) => seal,
// 			Err(_) => return Ok(false),
// 		};
//
// 		// See whether the hash meets the difficulty requirement. If not, fail fast.
// 		if !hash_meets_difficulty(&seal.work, difficulty) {
// 			return Ok(false);
// 		}
//
// 		// Make sure the provided work actually comes from the correct pre_hash
// 		let compute = Compute {
// 			difficulty,
// 			pre_hash: *pre_hash,
// 			nonce: seal.nonce,
// 		};
//
// 		//let _obj = <Module<T>>::get_obj(&seal.nonce);
//
// 		if compute.compute() != seal {
// 			return Ok(false);
// 		}
//
// 		Ok(true)
// 	}
// }

// /// A complete PoW Algorithm that uses Sha3 hashing.
// /// Needs a reference to the client so it can grab the difficulty from the runtime.
// pub struct PoscanAlgorithm<C> {
// 	client: Arc<C>,
// }
//
// impl<C> PoscanAlgorithm<C> {
// 	pub fn new(client: Arc<C>) -> Self {
// 		Self { client }
// 	}
// }
//
// // Manually implement clone. Deriving doesn't work because
// // it'll derive impl<C: Clone> Clone for Sha3Algorithm<C>. But C in practice isn't Clone.
// impl<C> Clone for PoscanAlgorithm<C> {
// 	fn clone(&self) -> Self {
// 		Self::new(self.client.clone())
// 	}
// }

// Here we implement the general PowAlgorithm trait for our concrete Sha3Algorithm
// impl<B: BlockT<Hash = H256>, C> PowAlgorithm<B> for PoscanAlgorithm<C>
// 	where
// 		C: ProvideRuntimeApi<B>,
// 		C::Api: DifficultyApi<B, U256>,
// {
// 	type Difficulty = U256;
//
// 	fn difficulty(&self, parent: B::Hash) -> Result<Self::Difficulty, Error<B>> {
// 		let parent_id = BlockId::<B>::hash(parent);
// 		self.client
// 			.runtime_api()
// 			.difficulty(&parent_id)
// 			.map_err(|err| {
// 				sc_consensus_poscan::Error::Environment(format!(
// 					"Fetching difficulty from runtime failed: {:?}",
// 					err
// 				))
// 			})
// 	}
//
// 	fn verify(
// 		&self,
// 		_parent: &BlockId<B>,
// 		pre_hash: &H256,
// 		_pre_digest: Option<&[u8]>,
// 		seal: &RawSeal,
// 		difficulty: Self::Difficulty,
// 		poscan_data: &PoscanData,
// 	) -> Result<bool, Error<B>> {
// 		// Try to construct a seal object by decoding the raw seal given
// 		let seal = match Seal::decode(&mut &seal[..]) {
// 			Ok(seal) => seal,
// 			Err(_) => return Ok(false),
// 		};
//
// 		// See whether the hash meets the difficulty requirement. If not, fail fast.
// 		if !hash_meets_difficulty(&seal.work, difficulty) {
// 			return Ok(false);
// 		}
//
// 		// Make sure the provided work actually comes from the correct pre_hash
// 		let compute = Compute {
// 			difficulty,
// 			pre_hash: *pre_hash,
// 			poscan_hash: seal.poscan_hash,
// 		};
//
// 		if compute.compute() != seal {
// 			return Ok(false);
// 		}
//
// 		// verify poscan
// 		let hashes = get_obj_hashes(&poscan_data.obj);
// 		if hashes != poscan_data.hashes {
// 			return Ok(false)
// 		}
//
// 		Ok(true)
// 	}
// }

#[derive(Clone)]
pub struct PoscanAlgorithm;

impl<B: BlockT<Hash = H256>> PowAlgorithm<B> for PoscanAlgorithm
{
	type Difficulty = U256;

	fn difficulty(&self, _parent: B::Hash) -> Result<Self::Difficulty, Error<B>> {
		// Fixed difficulty hardcoded here
		Ok(U256::from(10))
	}

	fn verify(
		&self,
		_parent: &BlockId<B>,
		pre_hash: &H256,
		_pre_digest: Option<&[u8]>,
		seal: &RawSeal,
		difficulty: Self::Difficulty,
		poscan_data: &PoscanData,
	) -> Result<bool, Error<B>> {
		// Try to construct a seal object by decoding the raw seal given
		let seal = match Seal::decode(&mut &seal[..]) {
			Ok(seal) => seal,
			Err(_) => return Ok(false),
		};

		// See whether the hash meets the difficulty requirement. If not, fail fast.
		if !hash_meets_difficulty(&seal.work, difficulty) {
			return Ok(false);
		}

		// Make sure the provided work actually comes from the correct pre_hash
		let compute = Compute {
			difficulty,
			pre_hash: *pre_hash,
			poscan_hash: seal.poscan_hash,
		};

		if compute.compute() != seal {
			return Ok(false);
		}

		// verify poscan
		let hashes = get_obj_hashes(&poscan_data.obj);
		if hashes != poscan_data.hashes {
			return Ok(false)
		}

		Ok(true)
	}
}


use p3d;
use log::*;
use std::str::FromStr;

pub fn get_obj_hashes(data: &Vec<u8>) -> Vec<H256> {
	let mut buf: Vec<H256> = Vec::new();
	// TODO: pass params as args
	let res = p3d::p3d_process(data, p3d::AlgoType::Grid2d, 6i16, 2i16 );

	match res {
		Ok(v) => {
			for i in 0..v.len() {
				// prn(v[i].as_str());
				let h = H256::from_str(v[i].as_str()).unwrap();
				buf.push(h);
			}
			// buf.extend(v.concat().as_bytes().to_vec());
		},
		Err(_) => {
			// runtime_print!(">>> Error");
			// TODO: handle error
			return Vec::new()
		},
	}

	buf
}

