// This file is a part of 3DPass.

// Copyright (C) 2017-2021 Parity Technologies (UK) Ltd.
// Copyright (C) 2022 3DPass
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Primitives for Proof-of-Scan (PoScan) consensus.

#![cfg_attr(not(feature = "std"), no_std)]
// #![no_std]


use sp_std::vec::Vec;
use sp_std::fmt::Debug;
use sp_runtime::{ConsensusEngineId, Percent};
use codec::{Encode, Decode, MaxEncodedLen};
use lzss::{Lzss, SliceReader, VecWriter};

use sp_runtime::{
	traits::{Member, ConstU32,},
	BoundedVec,
};

use sp_core::{
	H256, RuntimeDebug,
};

use scale_info::TypeInfo;

#[cfg(feature = "std")]
use serde::{Serialize, Deserialize};

/// The `ConsensusEngineId` of PoScan.
pub const POSCAN_ENGINE_ID: ConsensusEngineId = [b'p', b'o', b's', b'c'];
pub const POSCAN_SEAL_V1_ID: ConsensusEngineId = POSCAN_ENGINE_ID;
pub const POSCAN_SEAL_V2_ID: ConsensusEngineId = [b'p', b's', b'c', b'2'];
pub const POSCAN_COIN_ID: u8 = 71;

pub const POSCAN_ALGO_GRID2D: [u8; 16] = *b"grid2d-1.1      ";
pub const POSCAN_ALGO_GRID2D_V2: [u8; 16] = *b"grid2d-1.2      ";
pub const POSCAN_ALGO_GRID2D_V3: [u8; 16] = *b"grid2d-1.3      ";
pub const POSCAN_ALGO_GRID2D_V3_1: [u8; 16] = *b"grid2d-1.3.1    ";
pub const POSCAN_ALGO_GRID2D_V3A: [u8; 16] = *b"grid2d-1.3a     ";

pub const SUPPORTED_ALGORITHMS: [[u8; 16];5] =
	[
		POSCAN_ALGO_GRID2D,
		POSCAN_ALGO_GRID2D_V2,
		POSCAN_ALGO_GRID2D_V3,
		POSCAN_ALGO_GRID2D_V3_1,
		POSCAN_ALGO_GRID2D_V3A,
	];

pub const REJECT_OLD_ALGO_SINCE: u32 = 740500;
pub const SCALE_DIFF_SINCE: u32 = 370_898;
pub const SCALE_DIFF_BY: u32 = 1_000_000;
pub const MAX_MINING_OBJ_LEN: usize = 100 * 1024;

pub const CONS_V2_SPEC_VER: u32 = 128;

/// Type of seal.
pub type Seal = Vec<u8>;
pub type Difficulty = sp_core::U256;

/// Block interval, in seconds, the network will tune its next_target for.
pub const BLOCK_TIME_SEC: u64 = 60;
/// Block time interval in milliseconds.
pub const BLOCK_TIME: u64 = BLOCK_TIME_SEC * 1000;

/// Nominal height for standard time intervals, hour is 60 blocks
pub const HOUR_HEIGHT: u64 = 3600 / BLOCK_TIME_SEC;
/// A day is 1440 blocks
pub const DAY_HEIGHT: u64 = 24 * HOUR_HEIGHT;
/// A week is 10_080 blocks
pub const WEEK_HEIGHT: u64 = 7 * DAY_HEIGHT;
/// A year is 524_160 blocks
pub const YEAR_HEIGHT: u64 = 52 * WEEK_HEIGHT;

/// Number of blocks used to calculate difficulty adjustments
pub const DIFFICULTY_ADJUST_WINDOW: u64 = HOUR_HEIGHT;
/// Clamp factor to use for difficulty adjustment
/// Limit value to within this factor of goal
pub const CLAMP_FACTOR: u128 = 2;
/// Dampening factor to use for difficulty adjustment
pub const DIFFICULTY_DAMP_FACTOR: u128 = 3;
/// Minimum difficulty, enforced in diff retargetting
/// avoids getting stuck when trying to increase difficulty subject to dampening
pub const MIN_DIFFICULTY: u128 = DIFFICULTY_DAMP_FACTOR;
/// Maximum difficulty.
pub const MAX_DIFFICULTY: u128 = u128::max_value();

/// Value of 1 P3D.
pub const DOLLARS: u128 = 1_000_000_000_000;
/// Value of cents relative to P3D.
pub const CENTS: u128 = DOLLARS / 100;
/// Value of millicents relative to P3D.
pub const MILLICENTS: u128 = CENTS / 1_000;
/// Value of microcents relative to P3D.
pub const MICROCENTS: u128 = MILLICENTS / 1_000;

pub const fn deposit(items: u32, bytes: u32) -> u128 {
	items as u128 * 2 * DOLLARS + (bytes as u128) * 10 * MILLICENTS
}

/// Block number of one minute.
pub const MINUTES: u32 = 1;
/// Block number of one hour.
pub const HOURS: u32 = 60;
/// Block number of one day.
pub const DAYS: u32 = 24 * HOURS;

pub const MAX_OBJECT_SIZE: u32 = 100_000;
pub const MAX_PROPERTIES: u32 = 100;
pub const PROP_NAME_LEN: u32 = 64;
pub const DEFAULT_OBJECT_HASHES: u32 = 10;
pub const MAX_OBJECT_HASHES: u32 = 256 + DEFAULT_OBJECT_HASHES;
pub const DEFAULT_MAX_ALGO_TIME: u32 = 10;  // 10 sec
pub const FEE_PER_BYTE: u64 = 10_000;
pub const MAX_ESTIMATORS: u32 = 1000;


#[derive(Eq, PartialEq, codec::Encode, codec::Decode)]
pub enum CheckMemberError {
	NoPool,
	NoMember,
	PoolSuspended,
}

/// Define methods that total difficulty should implement.
pub trait TotalDifficulty {
	fn increment(&mut self, other: Self);
}

impl TotalDifficulty for sp_core::U256 {
	fn increment(&mut self, other: Self) {
		let ret = self.saturating_add(other);
		*self = ret;
	}
}

impl TotalDifficulty for u128 {
	fn increment(&mut self, other: Self) {
		let ret = self.saturating_add(other);
		*self = ret;
	}
}

sp_api::decl_runtime_apis! {
	/// API necessary for timestamp-based difficulty adjustment algorithms.
	pub trait TimestampApi<Moment: Decode> {
		/// Return the timestamp in the current block.
		fn timestamp() -> Moment;
	}

	/// API for those chains that put their difficulty adjustment algorithm directly
	/// onto runtime. Note that while putting difficulty adjustment algorithm to
	/// runtime is safe, putting the PoW algorithm on runtime is not.
	pub trait DifficultyApi<Difficulty: Decode> {
		/// Return the target difficulty of the next block.
		fn difficulty() -> Difficulty;
		fn hist_steps() -> u32;
	}

	pub trait MiningPoolApi<AccountId>
	where
		AccountId: codec::Decode + codec::Encode,
	{
		fn difficulty(pool_id: &AccountId) -> Difficulty;
		fn member_status(pool_id: &AccountId, member_id: &AccountId) -> Result<(), CheckMemberError>;
		fn get_stat(pool_id: &AccountId) -> Option<(Percent, Percent, Vec<(AccountId, u32)>)>;
	}

	pub trait AlgorithmApi {
		fn identifier() -> [u8; 8];
	}

	pub trait PoscanApi<AccountId, BlockNum>
	where
		BlockNum: Clone + Eq + Debug + Sync + Send + codec::Decode + codec::Encode + TypeInfo + Member,
		AccountId: Clone + Eq + Debug + Sync + Send + codec::Decode + codec::Encode + TypeInfo + Member,
	{
		fn get_poscan_object(i: u32) -> Option<ObjData<AccountId, BlockNum>>;
		fn check_object(alg_id: &[u8;16], obj: &Vec<u8>, hashes: &Vec<H256>) -> bool;
		fn get_obj_hashes_wasm(ver: &[u8; 16], data: &Vec<u8>, pre: &H256, depth: u32, patch_rot: bool) -> Vec<H256>;
	}
}

pub fn compress_obj(obj: &[u8]) -> Vec<u8> {
	type MyLzss = Lzss<10, 4, 0x20, { 1 << 10 }, { 2 << 10 }>;
	let result = MyLzss::compress(
		SliceReader::new(obj),
		VecWriter::with_capacity(4096),
	);

	result.unwrap()
}

pub fn decompress_obj(obj: &[u8]) -> Vec<u8> {
	type MyLzss = Lzss<10, 4, 0x20, { 1 << 10 }, { 2 << 10 }>;
	let result = MyLzss::decompress(
		SliceReader::new(obj),
		VecWriter::with_capacity(4096),
	);

	result.unwrap()
}

pub type ObjIdx = u32;
pub type PropIdx = u32;

#[derive(Clone, PartialEq, Encode, Decode, TypeInfo, MaxEncodedLen, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum Algo3D {
	Grid2dLow,
	Grid2dHigh,
}

#[derive(Clone, PartialEq, Encode, Decode, TypeInfo, MaxEncodedLen, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum ObjectCategory
{
	/// 3D objects
	Objects3D(Algo3D),
	/// 2D drawings
	Drawings2D,
	/// Music
	Music,
	/// Biometrics
	Biometrics,
	/// Movements
	Movements,
	/// Texts
	Texts,
}

#[derive(Clone, PartialEq, Encode, Decode, TypeInfo, MaxEncodedLen, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum ObjectState<Block>
	where
		Block: Encode + Decode + TypeInfo + Member,
{
	Created(Block),
	Estimating(Block),
	Estimated(Block, u64),
	NotApproved(Block),
	Approved(Block),
}

#[derive(Clone, PartialEq, Encode, Decode, TypeInfo, MaxEncodedLen, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum NotApprovedReason
{
	Expired,
	DuplicateFound(ObjIdx, ObjIdx),
}

#[derive(Copy, Clone, PartialEq, Encode, Decode, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum CompressWith {
	Lzss,
}

impl CompressWith {
	pub fn compress(&self, data: Vec<u8>) -> Vec<u8> {
		match self {
			Self::Lzss => { compress_obj(&data) },
		}
	}

	pub fn decompress(&self, data: &Vec<u8>) -> Vec<u8> {
		match self {
			Self::Lzss => { decompress_obj(&data) },
		}
	}
}

#[derive(Clone, PartialEq, Encode, Decode, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Approval<Account, Block>
	where
		Account: Encode + Decode + TypeInfo + Member,
		Block: Encode + Decode + TypeInfo + Member,
{
	pub account_id: Account,
	pub when: Block,
	pub proof: Option<H256>,
}

#[derive(Clone, PartialEq, Debug, Encode, Decode, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct PropValue {
	pub prop_idx: u32,
	pub max_value: u128,
}

#[derive(Clone, PartialEq, Debug, Encode, Decode, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum PropClass {
	Relative,
	Absolute,
}

#[derive(Clone, PartialEq, Encode, Decode, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Property {
	pub name: BoundedVec<u8, ConstU32<PROP_NAME_LEN>>,
	pub class: PropClass,
	pub max_value: u128,
}

#[derive(Clone, PartialEq, Encode, Decode, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct ObjData<Account, Block>
	where
		Account: Encode + Decode + TypeInfo + Member,
		Block: Encode + Decode + TypeInfo + Member,
{
	pub state: ObjectState<Block>,
	pub obj: BoundedVec<u8, ConstU32<MAX_OBJECT_SIZE>>,
	pub compressed_with: Option<CompressWith>,
	pub category: ObjectCategory,
	pub hashes: BoundedVec<H256, ConstU32<MAX_OBJECT_HASHES>>,
	pub when_created: Block,
	pub when_approved: Option<Block>,
	pub owner: Account,
	pub estimators: BoundedVec<(Account, u64), ConstU32<MAX_ESTIMATORS>>,
	pub est_outliers: BoundedVec<Account, ConstU32<MAX_ESTIMATORS>>,
	pub approvers: BoundedVec<Approval<Account, Block>, ConstU32<256>>,
	pub num_approvals: u8,
	pub est_rewards: u128,
	pub author_rewards: u128,
	pub prop: BoundedVec<PropValue, ConstU32<MAX_PROPERTIES>>,
}

#[derive(Clone, PartialEq, Encode, Decode, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct OldObjData<Account, Block>
	where
		Account: Encode + Decode + TypeInfo + Member,
		Block: Encode + Decode + TypeInfo + Member,
{
	pub state: ObjectState<Block>,
	pub obj: BoundedVec<u8, ConstU32<MAX_OBJECT_SIZE>>,
	pub compressed_with: Option<CompressWith>,
	pub category: ObjectCategory,
	pub hashes: BoundedVec<H256, ConstU32<MAX_OBJECT_HASHES>>,
	pub when_created: Block,
	pub when_approved: Option<Block>,
	pub owner: Account,
	pub estimators: BoundedVec<(Account, u64), ConstU32<MAX_ESTIMATORS>>,
	pub est_outliers: BoundedVec<Account, ConstU32<MAX_ESTIMATORS>>,
	pub approvers: BoundedVec<Approval<Account, Block>, ConstU32<256>>,
	pub num_approvals: u8,
	pub est_rewards: u128,
	pub author_rewards: u128,
}
