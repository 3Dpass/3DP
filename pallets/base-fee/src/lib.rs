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

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::comparison_chain)]

#[cfg(test)]
mod tests;

use frame_support::{traits::Get, weights::Weight};
use sp_core::U256;
use sp_runtime::Permill;

pub trait BaseFeeThreshold {
	fn lower() -> Permill;
	fn ideal() -> Permill;
	fn upper() -> Permill;
}

pub use self::pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event> + IsType<<Self as frame_system::Config>::Event>;
		/// Lower and upper bounds for increasing / decreasing `BaseFeePerGas`.
		type Threshold: BaseFeeThreshold;
		type DefaultBaseFeePerGas: Get<U256>;
		type DefaultElasticity: Get<Permill>;
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub base_fee_per_gas: U256,
		pub elasticity: Permill,
		_marker: PhantomData<T>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> GenesisConfig<T> {
		pub fn new(base_fee_per_gas: U256, elasticity: Permill) -> Self {
			Self {
				base_fee_per_gas,
				elasticity,
				_marker: PhantomData,
			}
		}
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self {
				base_fee_per_gas: T::DefaultBaseFeePerGas::get(),
				elasticity: T::DefaultElasticity::get(),
				_marker: PhantomData,
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			<BaseFeePerGas<T>>::put(self.base_fee_per_gas);
			<Elasticity<T>>::put(self.elasticity);
		}
	}

	#[pallet::type_value]
	pub fn DefaultBaseFeePerGas<T: Config>() -> U256 {
		T::DefaultBaseFeePerGas::get()
	}

	#[pallet::storage]
	#[pallet::getter(fn base_fee_per_gas)]
	pub type BaseFeePerGas<T> = StorageValue<_, U256, ValueQuery, DefaultBaseFeePerGas<T>>;

	#[pallet::type_value]
	pub fn DefaultElasticity<T: Config>() -> Permill {
		T::DefaultElasticity::get()
	}

	#[pallet::storage]
	#[pallet::getter(fn elasticity)]
	pub type Elasticity<T> = StorageValue<_, Permill, ValueQuery, DefaultElasticity<T>>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event {
		NewBaseFeePerGas { fee: U256 },
		BaseFeeOverflow,
		NewElasticity { elasticity: Permill },
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: T::BlockNumber) -> Weight {
			// Register the Weight used on_finalize.
			// 	- One storage read to get the block_weight.
			// 	- One storage read to get the Elasticity.
			// 	- One write to BaseFeePerGas.
			let db_weight = <T as frame_system::Config>::DbWeight::get();
			db_weight.reads(2).saturating_add(db_weight.write)
		}

		fn on_finalize(_n: <T as frame_system::Config>::BlockNumber) {
			if <Elasticity<T>>::get().is_zero() {
				// Zero elasticity means constant BaseFeePerGas.
				return;
			}

			let lower = T::Threshold::lower();
			let upper = T::Threshold::upper();
			// `target` is the ideal congestion of the network where the base fee should remain unchanged.
			// Under normal circumstances the `target` should be 50%.
			// If we go below the `target`, the base fee is linearly decreased by the Elasticity delta of lower~target.
			// If we go above the `target`, the base fee is linearly increased by the Elasticity delta of upper~target.
			// The base fee is fully increased (default 12.5%) if the block is upper full (default 100%).
			// The base fee is fully decreased (default 12.5%) if the block is lower empty (default 0%).
			let weight = <frame_system::Pallet<T>>::block_weight();
			let max_weight = <<T as frame_system::Config>::BlockWeights>::get().max_block;

			// We convert `weight` into block fullness and ensure we are within the lower and upper bound.
			let weight_used =
				Permill::from_rational(weight.total(), max_weight).clamp(lower, upper);
			// After clamp `weighted_used` is always between `lower` and `upper`.
			// We scale the block fullness range to the lower/upper range, and the usage represents the
			// actual percentage within this new scale.
			let usage = (weight_used - lower) / (upper - lower);

			// Target is our ideal block fullness.
			let target = T::Threshold::ideal();
			if usage > target {
				// Above target, increase.
				let coef = Permill::from_parts((usage.deconstruct() - target.deconstruct()) * 2u32);
				// How much of the Elasticity is used to mutate base fee.
				let coef = <Elasticity<T>>::get() * coef;
				<BaseFeePerGas<T>>::mutate(|bf| {
					if let Some(scaled_basefee) = bf.checked_mul(U256::from(coef.deconstruct())) {
						// Normalize to GWEI.
						let increase = scaled_basefee
							.checked_div(U256::from(1_000_000))
							.unwrap_or_else(U256::zero);
						*bf = bf.saturating_add(increase);
					} else {
						Self::deposit_event(Event::BaseFeeOverflow);
					}
				});
			} else if usage < target {
				// Below target, decrease.
				let coef = Permill::from_parts((target.deconstruct() - usage.deconstruct()) * 2u32);
				// How much of the Elasticity is used to mutate base fee.
				let coef = <Elasticity<T>>::get() * coef;
				<BaseFeePerGas<T>>::mutate(|bf| {
					if let Some(scaled_basefee) = bf.checked_mul(U256::from(coef.deconstruct())) {
						// Normalize to GWEI.
						let decrease = scaled_basefee
							.checked_div(U256::from(1_000_000))
							.unwrap_or_else(U256::zero);
						*bf = bf.saturating_sub(decrease);
					} else {
						Self::deposit_event(Event::BaseFeeOverflow);
					}
				});
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn set_base_fee_per_gas(origin: OriginFor<T>, fee: U256) -> DispatchResult {
			ensure_root(origin)?;
			let _ = Self::set_base_fee_per_gas_inner(fee);
			Self::deposit_event(Event::NewBaseFeePerGas { fee });
			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn set_elasticity(origin: OriginFor<T>, elasticity: Permill) -> DispatchResult {
			ensure_root(origin)?;
			let _ = Self::set_elasticity_inner(elasticity);
			Self::deposit_event(Event::NewElasticity { elasticity });
			Ok(())
		}
	}
}

impl<T: Config> fp_evm::FeeCalculator for Pallet<T> {
	fn min_gas_price() -> (U256, Weight) {
		(<BaseFeePerGas<T>>::get(), T::DbWeight::get().reads(1))
	}
}

impl<T: Config> Pallet<T> {
	pub fn set_base_fee_per_gas_inner(value: U256) -> Weight {
		<BaseFeePerGas<T>>::put(value);
		T::DbWeight::get().write
	}
	pub fn set_elasticity_inner(value: Permill) -> Weight {
		<Elasticity<T>>::put(value);
		T::DbWeight::get().write
	}
}
