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

#[cfg(test)]
mod tests;

#[cfg(test)]
mod mock;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;
pub mod weights;
pub use weights::WeightInfo;

pub use pallet_evm::AddressMapping;
use sp_runtime::traits::Zero;
use sp_std::vec::Vec;

pub use self::pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{dispatch::PostDispatchInfo, pallet_prelude::*};
	use frame_system::pallet_prelude::*;
	use sp_core::H160;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Mapping from address to account id.
		type AddressMapping: AddressMapping<Self::AccountId>;
		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Maximum address count exceeded
		MaxAddressCountExceeded,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Increment `sufficients` for existing accounts having a nonzero `nonce` but zero `sufficients`, `consumers` and `providers` value.
		/// This state was caused by a previous bug in EVM create account dispatchable.
		///
		/// Any accounts in the input list not satisfying the above condition will remain unaffected.
		#[pallet::weight(
			<T as pallet::Config>::WeightInfo::hotfix_inc_account_sufficients(addresses.len().try_into().unwrap_or(u32::MAX))
		)]
		pub fn hotfix_inc_account_sufficients(
			origin: OriginFor<T>,
			addresses: Vec<H160>,
		) -> DispatchResultWithPostInfo {
			const MAX_ADDRESS_COUNT: usize = 1000;

			frame_system::ensure_signed(origin)?;
			ensure!(
				addresses.len() <= MAX_ADDRESS_COUNT,
				Error::<T>::MaxAddressCountExceeded
			);

			for address in addresses {
				let account_id = T::AddressMapping::into_account_id(address);
				let nonce = frame_system::Pallet::<T>::account_nonce(&account_id);
				let refs = frame_system::Pallet::<T>::consumers(&account_id)
					.saturating_add(frame_system::Pallet::<T>::providers(&account_id))
					.saturating_add(frame_system::Pallet::<T>::sufficients(&account_id));

				if !nonce.is_zero() && refs.is_zero() {
					frame_system::Pallet::<T>::inc_sufficients(&account_id);
				}
			}

			Ok(PostDispatchInfo {
				actual_weight: None,
				pays_fee: Pays::Yes,
			})
		}
	}
}
