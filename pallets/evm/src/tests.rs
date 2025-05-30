// SPDX-License-Identifier: Apache-2.0
// This file is part of Frontier.
//
// Copyright (c) 2020-2022 Parity Technologies (UK) Ltd.
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

#![cfg(test)]

use super::*;
use crate::mock::*;

use frame_support::{
	assert_ok,
	traits::{GenesisBuild, LockIdentifier, LockableCurrency, WithdrawReasons},
};
use std::{collections::BTreeMap, str::FromStr};

type Balances = pallet_balances::Pallet<Test>;
type EVM = Pallet<Test>;

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default()
		.build_storage::<Test>()
		.unwrap();

	let mut accounts = BTreeMap::new();
	accounts.insert(
		H160::from_str("1000000000000000000000000000000000000001").unwrap(),
		GenesisAccount {
			nonce: U256::from(1),
			balance: U256::from(1000000),
			storage: Default::default(),
			code: vec![
				0x00, // STOP
			],
		},
	);
	accounts.insert(
		H160::from_str("1000000000000000000000000000000000000002").unwrap(),
		GenesisAccount {
			nonce: U256::from(1),
			balance: U256::from(1000000),
			storage: Default::default(),
			code: vec![
				0xff, // INVALID
			],
		},
	);
	accounts.insert(
		H160::default(), // root
		GenesisAccount {
			nonce: U256::from(1),
			balance: U256::max_value(),
			storage: Default::default(),
			code: vec![],
		},
	);

	pallet_balances::GenesisConfig::<Test> {
		// Create the block author account with some balance.
		balances: vec![(
			H160::from_str("0x1234500000000000000000000000000000000000").unwrap(),
			12345,
		)],
	}
	.assimilate_storage(&mut t)
	.expect("Pallet balances storage can be assimilated");
	GenesisBuild::<Test>::assimilate_storage(&crate::GenesisConfig { accounts }, &mut t).unwrap();
	t.into()
}

#[test]
fn fail_call_return_ok() {
	new_test_ext().execute_with(|| {
		assert_ok!(EVM::call(
			Origin::root(),
			H160::default(),
			H160::from_str("1000000000000000000000000000000000000001").unwrap(),
			Vec::new(),
			U256::default(),
			1000000,
			U256::from(1_000_000_000),
			None,
			None,
			Vec::new(),
		));

		assert_ok!(EVM::call(
			Origin::root(),
			H160::default(),
			H160::from_str("1000000000000000000000000000000000000002").unwrap(),
			Vec::new(),
			U256::default(),
			1000000,
			U256::from(1_000_000_000),
			None,
			None,
			Vec::new(),
		));
	});
}

#[test]
fn fee_deduction() {
	new_test_ext().execute_with(|| {
		// Create an EVM address and the corresponding Substrate address that will be charged fees and refunded
		let evm_addr = H160::from_str("1000000000000000000000000000000000000003").unwrap();
		let substrate_addr = <Test as Config>::AddressMapping::into_account_id(evm_addr);

		// Seed account
		let _ = <Test as Config>::Currency::deposit_creating(&substrate_addr, 100);
		assert_eq!(Balances::free_balance(&substrate_addr), 100);

		// Deduct fees as 10 units
		let imbalance = <<Test as Config>::OnChargeTransaction as OnChargeEVMTransaction<Test>>::withdraw_fee(&evm_addr, U256::from(10)).unwrap();
		assert_eq!(Balances::free_balance(&substrate_addr), 90);

		// Refund fees as 5 units
		<<Test as Config>::OnChargeTransaction as OnChargeEVMTransaction<Test>>::correct_and_deposit_fee(&evm_addr, U256::from(5), U256::from(5), imbalance);
		assert_eq!(Balances::free_balance(&substrate_addr), 95);
	});
}

#[test]
fn ed_0_refund_patch_works() {
	new_test_ext().execute_with(|| {
		// Verifies that the OnChargeEVMTransaction patch is applied and fixes a known bug in Substrate for evm transactions.
		// https://github.com/paritytech/substrate/issues/10117
		let evm_addr = H160::from_str("1000000000000000000000000000000000000003").unwrap();
		let substrate_addr = <Test as Config>::AddressMapping::into_account_id(evm_addr);

		let _ = <Test as Config>::Currency::deposit_creating(&substrate_addr, 21_777_000_000_000);
		assert_eq!(Balances::free_balance(&substrate_addr), 21_777_000_000_000);

		let _ = EVM::call(
			Origin::root(),
			evm_addr,
			H160::from_str("1000000000000000000000000000000000000001").unwrap(),
			Vec::new(),
			U256::from(1_000_000_000),
			21776,
			U256::from(1_000_000_000),
			None,
			Some(U256::from(0)),
			Vec::new(),
		);
		// All that was due, was refunded.
		assert_eq!(Balances::free_balance(&substrate_addr), 776_000_000_000);
	});
}

#[test]
fn ed_0_refund_patch_is_required() {
	new_test_ext().execute_with(|| {
		// This test proves that the patch is required, verifying that the current Substrate behaviour is incorrect
		// for ED 0 configured chains.
		let evm_addr = H160::from_str("1000000000000000000000000000000000000003").unwrap();
		let substrate_addr = <Test as Config>::AddressMapping::into_account_id(evm_addr);

		let _ = <Test as Config>::Currency::deposit_creating(&substrate_addr, 100);
		assert_eq!(Balances::free_balance(&substrate_addr), 100);

		// Drain funds
		let _ =
			<<Test as Config>::OnChargeTransaction as OnChargeEVMTransaction<Test>>::withdraw_fee(
				&evm_addr,
				U256::from(100),
			)
			.unwrap();
		assert_eq!(Balances::free_balance(&substrate_addr), 0);

		// Try to refund. With ED 0, although the balance is now 0, the account still exists.
		// So its expected that calling `deposit_into_existing` results in the AccountData to increase the Balance.
		//
		// Is not the case, and this proves that the refund logic needs to be handled taking this into account.
		assert_eq!(
			<Test as Config>::Currency::deposit_into_existing(&substrate_addr, 5u32.into())
				.is_err(),
			true
		);
		// Balance didn't change, and should be 5.
		assert_eq!(Balances::free_balance(&substrate_addr), 0);
	});
}

#[test]
fn find_author() {
	new_test_ext().execute_with(|| {
		let author = EVM::find_author();
		assert_eq!(
			author,
			H160::from_str("1234500000000000000000000000000000000000").unwrap()
		);
	});
}

#[test]
fn reducible_balance() {
	new_test_ext().execute_with(|| {
		let evm_addr = H160::from_str("1000000000000000000000000000000000000001").unwrap();
		let account_id = <Test as Config>::AddressMapping::into_account_id(evm_addr);
		let existential = ExistentialDeposit::get();

		// Genesis Balance.
		let genesis_balance = EVM::account_basic(&evm_addr).0.balance;

		// Lock identifier.
		let lock_id: LockIdentifier = *b"te/stlok";
		// Reserve some funds.
		let to_lock = 1000;
		Balances::set_lock(lock_id, &account_id, to_lock, WithdrawReasons::RESERVE);
		// Reducible is, as currently configured in `account_basic`, (balance - lock - existential).
		let reducible_balance = EVM::account_basic(&evm_addr).0.balance;
		assert_eq!(reducible_balance, (genesis_balance - to_lock - existential));
	});
}

#[test]
fn author_should_get_tip() {
	new_test_ext().execute_with(|| {
		let author = EVM::find_author();
		let before_tip = EVM::account_basic(&author).0.balance;
		let result = EVM::call(
			Origin::root(),
			H160::default(),
			H160::from_str("1000000000000000000000000000000000000001").unwrap(),
			Vec::new(),
			U256::from(1),
			1000000,
			U256::from(2_000_000_000),
			Some(U256::from(1)),
			None,
			Vec::new(),
		);
		result.expect("EVM can be called");
		let after_tip = EVM::account_basic(&author).0.balance;
		assert_eq!(after_tip, (before_tip + 21000));
	});
}

#[test]
fn issuance_after_tip() {
	new_test_ext().execute_with(|| {
		let before_tip = <Test as Config>::Currency::total_issuance();
		let result = EVM::call(
			Origin::root(),
			H160::default(),
			H160::from_str("1000000000000000000000000000000000000001").unwrap(),
			Vec::new(),
			U256::from(1),
			1000000,
			U256::from(2_000_000_000),
			Some(U256::from(1)),
			None,
			Vec::new(),
		);
		result.expect("EVM can be called");
		let after_tip = <Test as Config>::Currency::total_issuance();
		// Only base fee is burned
		let base_fee: u64 = <Test as Config>::FeeCalculator::min_gas_price()
			.0
			.unique_saturated_into();
		assert_eq!(after_tip, (before_tip - (base_fee * 21_000)));
	});
}

#[test]
fn author_same_balance_without_tip() {
	new_test_ext().execute_with(|| {
		let author = EVM::find_author();
		let before_tip = EVM::account_basic(&author).0.balance;
		let _ = EVM::call(
			Origin::root(),
			H160::default(),
			H160::from_str("1000000000000000000000000000000000000001").unwrap(),
			Vec::new(),
			U256::default(),
			1000000,
			U256::default(),
			None,
			None,
			Vec::new(),
		);
		let after_tip = EVM::account_basic(&author).0.balance;
		assert_eq!(after_tip, before_tip);
	});
}

#[test]
fn refunds_should_work() {
	new_test_ext().execute_with(|| {
		let before_call = EVM::account_basic(&H160::default()).0.balance;
		// Gas price is not part of the actual fee calculations anymore, only the base fee.
		//
		// Because we first deduct max_fee_per_gas * gas_limit (2_000_000_000 * 1000000) we need
		// to ensure that the difference (max fee VS base fee) is refunded.
		let _ = EVM::call(
			Origin::root(),
			H160::default(),
			H160::from_str("1000000000000000000000000000000000000001").unwrap(),
			Vec::new(),
			U256::from(1),
			1000000,
			U256::from(2_000_000_000),
			None,
			None,
			Vec::new(),
		);
		let (base_fee, _) = <Test as Config>::FeeCalculator::min_gas_price();
		let total_cost = (U256::from(21_000) * base_fee) + U256::from(1);
		let after_call = EVM::account_basic(&H160::default()).0.balance;
		assert_eq!(after_call, before_call - total_cost);
	});
}

#[test]
fn refunds_and_priority_should_work() {
	new_test_ext().execute_with(|| {
		let author = EVM::find_author();
		let before_tip = EVM::account_basic(&author).0.balance;
		let before_call = EVM::account_basic(&H160::default()).0.balance;
		// We deliberately set a base fee + max tip > max fee.
		// The effective priority tip will be 1GWEI instead 1.5GWEI:
		// 		(max_fee_per_gas - base_fee).min(max_priority_fee)
		//		(2 - 1).min(1.5)
		let tip = U256::from(1_500_000_000);
		let max_fee_per_gas = U256::from(2_000_000_000);
		let used_gas = U256::from(21_000);
		let _ = EVM::call(
			Origin::root(),
			H160::default(),
			H160::from_str("1000000000000000000000000000000000000001").unwrap(),
			Vec::new(),
			U256::from(1),
			1000000,
			max_fee_per_gas,
			Some(tip),
			None,
			Vec::new(),
		);
		let (base_fee, _) = <Test as Config>::FeeCalculator::min_gas_price();
		let actual_tip = (max_fee_per_gas - base_fee).min(tip) * used_gas;
		let total_cost = (used_gas * base_fee) + U256::from(actual_tip) + U256::from(1);
		let after_call = EVM::account_basic(&H160::default()).0.balance;
		// The tip is deducted but never refunded to the caller.
		assert_eq!(after_call, before_call - total_cost);

		let after_tip = EVM::account_basic(&author).0.balance;
		assert_eq!(after_tip, (before_tip + actual_tip));
	});
}

#[test]
fn call_should_fail_with_priority_greater_than_max_fee() {
	new_test_ext().execute_with(|| {
		// Max priority greater than max fee should fail.
		let tip: u128 = 1_100_000_000;
		let result = EVM::call(
			Origin::root(),
			H160::default(),
			H160::from_str("1000000000000000000000000000000000000001").unwrap(),
			Vec::new(),
			U256::from(1),
			1000000,
			U256::from(1_000_000_000),
			Some(U256::from(tip)),
			None,
			Vec::new(),
		);
		assert!(result.is_err());
		// Some used weight is returned as part of the error.
		assert_eq!(result.unwrap_err().post_info.actual_weight, Some(7));
	});
}

#[test]
fn call_should_succeed_with_priority_equal_to_max_fee() {
	new_test_ext().execute_with(|| {
		let tip: u128 = 1_000_000_000;
		// Mimics the input for pre-eip-1559 transaction types where `gas_price`
		// is used for both `max_fee_per_gas` and `max_priority_fee_per_gas`.
		let result = EVM::call(
			Origin::root(),
			H160::default(),
			H160::from_str("1000000000000000000000000000000000000001").unwrap(),
			Vec::new(),
			U256::from(1),
			1000000,
			U256::from(1_000_000_000),
			Some(U256::from(tip)),
			None,
			Vec::new(),
		);
		assert!(result.is_ok());
	});
}

#[test]
fn handle_sufficient_reference() {
	new_test_ext().execute_with(|| {
		let addr = H160::from_str("1230000000000000000000000000000000000001").unwrap();
		let addr_2 = H160::from_str("1234000000000000000000000000000000000001").unwrap();
		let substrate_addr = <Test as Config>::AddressMapping::into_account_id(addr);
		let substrate_addr_2 = <Test as Config>::AddressMapping::into_account_id(addr_2);

		// Sufficients should increase when creating EVM accounts.
		let _ = <crate::AccountCodes<Test>>::insert(addr, &vec![0]);
		let account = frame_system::Account::<Test>::get(substrate_addr);
		// Using storage is not correct as it leads to a sufficient reference mismatch.
		assert_eq!(account.sufficients, 0);

		// Using the create / remove account functions is the correct way to handle it.
		EVM::create_account(addr_2, vec![1, 2, 3]);
		let account_2 = frame_system::Account::<Test>::get(substrate_addr_2);
		// We increased the sufficient reference by 1.
		assert_eq!(account_2.sufficients, 1);
		EVM::remove_account(&addr_2);
		let account_2 = frame_system::Account::<Test>::get(substrate_addr_2);
		// We decreased the sufficient reference by 1 on removing the account.
		assert_eq!(account_2.sufficients, 0);
	});
}

#[test]
fn runner_non_transactional_calls_with_non_balance_accounts_is_ok_without_gas_price() {
	// Expect to skip checks for gas price and account balance when both:
	//	- The call is non transactional (`is_transactional == false`).
	//	- The `max_fee_per_gas` is None.
	new_test_ext().execute_with(|| {
		let non_balance_account =
			H160::from_str("7700000000000000000000000000000000000001").unwrap();
		assert_eq!(
			EVM::account_basic(&non_balance_account).0.balance,
			U256::zero()
		);
		let _ = <Test as Config>::Runner::call(
			non_balance_account,
			H160::from_str("1000000000000000000000000000000000000001").unwrap(),
			Vec::new(),
			U256::from(1u32),
			1000000,
			None,
			None,
			None,
			Vec::new(),
			false, // non-transactional
			true,  // must be validated
			&<Test as Config>::config().clone(),
		)
		.expect("Non transactional call succeeds");
		assert_eq!(
			EVM::account_basic(&non_balance_account).0.balance,
			U256::zero()
		);
	});
}

#[test]
fn runner_non_transactional_calls_with_non_balance_accounts_is_err_with_gas_price() {
	// In non transactional calls where `Some(gas_price)` is defined, expect it to be
	// checked against the `BaseFee`, and expect the account to have enough balance
	// to pay for the call.
	new_test_ext().execute_with(|| {
		let non_balance_account =
			H160::from_str("7700000000000000000000000000000000000001").unwrap();
		assert_eq!(
			EVM::account_basic(&non_balance_account).0.balance,
			U256::zero()
		);
		let res = <Test as Config>::Runner::call(
			non_balance_account,
			H160::from_str("1000000000000000000000000000000000000001").unwrap(),
			Vec::new(),
			U256::from(1u32),
			1000000,
			Some(U256::from(1_000_000_000)),
			None,
			None,
			Vec::new(),
			false, // non-transactional
			true,  // must be validated
			&<Test as Config>::config().clone(),
		);
		assert!(res.is_err());
	});
}

#[test]
fn runner_transactional_call_with_zero_gas_price_fails() {
	// Transactional calls are rejected when `max_fee_per_gas == None`.
	new_test_ext().execute_with(|| {
		let res = <Test as Config>::Runner::call(
			H160::default(),
			H160::from_str("1000000000000000000000000000000000000001").unwrap(),
			Vec::new(),
			U256::from(1u32),
			1000000,
			None,
			None,
			None,
			Vec::new(),
			true, // transactional
			true, // must be validated
			&<Test as Config>::config().clone(),
		);
		assert!(res.is_err());
	});
}

#[test]
fn runner_max_fee_per_gas_gte_max_priority_fee_per_gas() {
	// Transactional and non transactional calls enforce `max_fee_per_gas >= max_priority_fee_per_gas`.
	new_test_ext().execute_with(|| {
		let res = <Test as Config>::Runner::call(
			H160::default(),
			H160::from_str("1000000000000000000000000000000000000001").unwrap(),
			Vec::new(),
			U256::from(1u32),
			1000000,
			Some(U256::from(1_000_000_000)),
			Some(U256::from(2_000_000_000)),
			None,
			Vec::new(),
			true, // transactional
			true, // must be validated
			&<Test as Config>::config().clone(),
		);
		assert!(res.is_err());
		let res = <Test as Config>::Runner::call(
			H160::default(),
			H160::from_str("1000000000000000000000000000000000000001").unwrap(),
			Vec::new(),
			U256::from(1u32),
			1000000,
			Some(U256::from(1_000_000_000)),
			Some(U256::from(2_000_000_000)),
			None,
			Vec::new(),
			false, // non-transactional
			true,  // must be validated
			&<Test as Config>::config().clone(),
		);
		assert!(res.is_err());
	});
}
