// SPDX-License-Identifier: Apache-2.0
// This file is part of Frontier.
//
// Copyright (c) 2021-2022 Parity Technologies (UK) Ltd.
//
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

use frame_support::{
	assert_ok,
	pallet_prelude::GenesisBuild,
	parameter_types,
	traits::{ConstU32, OnFinalize},
	weights::DispatchClass,
};
use sp_core::{H256, U256};
use sp_io::TestExternalities;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	Permill,
};

use super::*;
use crate as pallet_base_fee;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::simple_max(1024);
}
impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type Origin = Origin;
	type Call = Call;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type DbWeight = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
}

parameter_types! {
	pub DefaultBaseFeePerGas: U256 = U256::from(100_000_000_000 as u128);
	pub DefaultElasticity: Permill = Permill::from_parts(125_000);
}

pub struct BaseFeeThreshold;
impl pallet_base_fee::BaseFeeThreshold for BaseFeeThreshold {
	fn lower() -> Permill {
		Permill::zero()
	}
	fn ideal() -> Permill {
		Permill::from_parts(500_000)
	}
	fn upper() -> Permill {
		Permill::from_parts(1_000_000)
	}
}

impl Config for Test {
	type Event = Event;
	type Threshold = BaseFeeThreshold;
	type DefaultBaseFeePerGas = DefaultBaseFeePerGas;
	type DefaultElasticity = DefaultElasticity;
}

frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		BaseFee: pallet_base_fee::{Pallet, Call, Storage, Event},
	}
);

pub fn new_test_ext(base_fee: Option<U256>, elasticity: Option<Permill>) -> TestExternalities {
	let mut t = frame_system::GenesisConfig::default()
		.build_storage::<Test>()
		.unwrap();

	match (base_fee, elasticity) {
		(Some(base_fee), Some(elasticity)) => {
			pallet_base_fee::GenesisConfig::<Test>::new(base_fee, elasticity)
		}
		(None, Some(elasticity)) => {
			let mut config = pallet_base_fee::GenesisConfig::<Test>::default();
			config.elasticity = elasticity;
			config
		}
		(Some(base_fee), None) => {
			let mut config = pallet_base_fee::GenesisConfig::<Test>::default();
			config.base_fee_per_gas = base_fee;
			config
		}
		(None, None) => pallet_base_fee::GenesisConfig::<Test>::default(),
	}
	.assimilate_storage(&mut t)
	.unwrap();

	TestExternalities::new(t)
}

#[test]
fn should_default() {
	new_test_ext(None, None).execute_with(|| {
		assert_eq!(
			BaseFee::base_fee_per_gas(),
			U256::from(100_000_000_000 as u128)
		);
		assert_eq!(BaseFee::elasticity(), Permill::from_parts(125_000));
	});
}

#[test]
fn should_not_overflow_u256() {
	let base_fee = U256::max_value();
	new_test_ext(Some(base_fee), None).execute_with(|| {
		let init = BaseFee::base_fee_per_gas();
		System::register_extra_weight_unchecked(1000000000000, DispatchClass::Normal);
		BaseFee::on_finalize(System::block_number());
		assert_eq!(BaseFee::base_fee_per_gas(), init);
	});
}

#[test]
fn should_handle_zero() {
	let base_fee = U256::zero();
	new_test_ext(Some(base_fee), None).execute_with(|| {
		let init = BaseFee::base_fee_per_gas();
		BaseFee::on_finalize(System::block_number());
		assert_eq!(BaseFee::base_fee_per_gas(), init);
	});
}

#[test]
fn should_handle_consecutive_empty_blocks() {
	let base_fee = U256::from(1_000_000_000);
	new_test_ext(Some(base_fee), None).execute_with(|| {
		for _ in 0..10000 {
			BaseFee::on_finalize(System::block_number());
			System::set_block_number(System::block_number() + 1);
		}
		assert_eq!(
			BaseFee::base_fee_per_gas(),
			// 8 is the lowest number which's 12.5% is >= 1.
			U256::from(7)
		);
	});
	let zero_elasticity = Permill::zero();
	new_test_ext(Some(base_fee), Some(zero_elasticity)).execute_with(|| {
		for _ in 0..10000 {
			BaseFee::on_finalize(System::block_number());
			System::set_block_number(System::block_number() + 1);
		}
		assert_eq!(
			BaseFee::base_fee_per_gas(),
			// base fee won't change
			base_fee
		);
	});
}

#[test]
fn should_handle_consecutive_full_blocks() {
	let base_fee = U256::from(1_000_000_000);
	new_test_ext(Some(base_fee), None).execute_with(|| {
		for _ in 0..10000 {
			// Register max weight in block.
			System::register_extra_weight_unchecked(1000000000000, DispatchClass::Normal);
			BaseFee::on_finalize(System::block_number());
			System::set_block_number(System::block_number() + 1);
		}
		assert_eq!(
			BaseFee::base_fee_per_gas(),
			// Max value allowed in the algorithm before overflowing U256.
			U256::from_dec_str(
				"930583037201699994746877284806656508753618758732556029383742480470471799"
			)
			.unwrap()
		);
	});
	let zero_elasticity = Permill::zero();
	new_test_ext(Some(base_fee), Some(zero_elasticity)).execute_with(|| {
		for _ in 0..10000 {
			// Register max weight in block.
			System::register_extra_weight_unchecked(1000000000000, DispatchClass::Normal);
			BaseFee::on_finalize(System::block_number());
			System::set_block_number(System::block_number() + 1);
		}
		assert_eq!(
			BaseFee::base_fee_per_gas(),
			// base fee won't change
			base_fee
		);
	});
}

#[test]
fn should_increase_total_base_fee() {
	let base_fee = U256::from(1_000_000_000);
	new_test_ext(Some(base_fee), None).execute_with(|| {
		assert_eq!(BaseFee::base_fee_per_gas(), U256::from(1000000000));
		// Register max weight in block.
		System::register_extra_weight_unchecked(1000000000000, DispatchClass::Normal);
		BaseFee::on_finalize(System::block_number());
		// Expect the base fee to increase by 12.5%.
		assert_eq!(BaseFee::base_fee_per_gas(), U256::from(1125000000));
	});
}

#[test]
fn should_increase_delta_of_base_fee() {
	let base_fee = U256::from(1_000_000_000);
	new_test_ext(Some(base_fee), None).execute_with(|| {
		assert_eq!(BaseFee::base_fee_per_gas(), U256::from(1000000000));
		// Register 75% capacity in block weight.
		System::register_extra_weight_unchecked(750000000000, DispatchClass::Normal);
		BaseFee::on_finalize(System::block_number());
		// Expect a 6.25% increase in base fee for a target capacity of 50% ((75/50)-1 = 0.5 * 0.125 = 0.0625).
		assert_eq!(BaseFee::base_fee_per_gas(), U256::from(1062500000));
	});
}

#[test]
fn should_idle_base_fee() {
	let base_fee = U256::from(1_000_000_000);
	new_test_ext(Some(base_fee), None).execute_with(|| {
		assert_eq!(BaseFee::base_fee_per_gas(), U256::from(1000000000));
		// Register half capacity in block weight.
		System::register_extra_weight_unchecked(500000000000, DispatchClass::Normal);
		BaseFee::on_finalize(System::block_number());
		// Expect the base fee to remain unchanged
		assert_eq!(BaseFee::base_fee_per_gas(), U256::from(1000000000));
	});
}

#[test]
fn set_base_fee_per_gas_dispatchable() {
	let base_fee = U256::from(1_000_000_000);
	new_test_ext(Some(base_fee), None).execute_with(|| {
		assert_eq!(BaseFee::base_fee_per_gas(), U256::from(1000000000));
		assert_ok!(BaseFee::set_base_fee_per_gas(Origin::root(), U256::from(1)));
		assert_eq!(BaseFee::base_fee_per_gas(), U256::from(1));
	});
}

#[test]
fn set_elasticity_dispatchable() {
	let base_fee = U256::from(1_000_000_000);
	new_test_ext(Some(base_fee), None).execute_with(|| {
		assert_eq!(BaseFee::elasticity(), Permill::from_parts(125_000));
		assert_ok!(BaseFee::set_elasticity(
			Origin::root(),
			Permill::from_parts(1_000)
		));
		assert_eq!(BaseFee::elasticity(), Permill::from_parts(1_000));
	});
}
