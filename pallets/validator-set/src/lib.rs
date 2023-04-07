//! # Validator Set Pallet
//!
//! This file is part of 3DPass.
//! Copyright (c) 2022 3DPass.
//!
//! The Validator Set Pallet allows for addition and removal of
//! authorities/validators via extrinsics (transaction calls), in
//! Substrate-based PoA networks. It also integrates with the im-online pallet
//! to automatically remove offline validators.
//!
//! The pallet uses the Session pallet and implements related traits for session
//! management. Currently, it uses periodic session rotation provided by the
//! session pallet to automatically rotate sessions. For this reason, the
//! validator addition and removal becomes effective only after 2 sessions
//! (queuing + applying).

#![cfg_attr(not(feature = "std"), no_std)]

mod mock;
mod tests;

use frame_support::{
	ensure,
	pallet_prelude::*,
	traits::{
		Currency, LockableCurrency, EstimateNextSessionRotation,
		Get, ValidatorSet, ValidatorSetWithIdentification,
		OnUnbalanced, ExistenceRequirement, LockIdentifier, WithdrawReasons,
	},
	sp_runtime::SaturatedConversion,
};
use log;
pub use pallet::*;
use sp_runtime::traits::{Convert, Zero};
use sp_staking::offence::{Offence, OffenceError, ReportOffence};
use sp_version::RuntimeVersion;
use sp_std::{collections::btree_set::BTreeSet, prelude::*};
use core::convert::TryInto;
use sp_consensus_poscan::HOURS;

use rewards_api::RewardLocksApi;
use validator_set_api::ValidatorSetApi;

const CUR_SPEC_VERSION: u32 = 101;
const UPGRADE_SLASH_DELAY: u32 = 5 * 24 * HOURS;
const LOCK_ID: LockIdentifier = *b"validatr";
pub const LOG_TARGET: &'static str = "runtime::validator-set";

pub type BalanceOf<T> =
<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

type NegativeImbalanceOf<T> = <<T as Config>::Currency as Currency<
	<T as frame_system::Config>::AccountId,
>>::NegativeImbalance;


#[derive(Encode, Decode, Debug, Clone, PartialEq, TypeInfo)]
pub enum RemoveReason {
	Normal,
	DepositBelowLimit,
	ImOnlineSlash,
}

impl Default for RemoveReason {
	fn default() -> Self {
		RemoveReason::Normal
	}
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_system::pallet_prelude::*;

	/// Configure the pallet by specifying the parameters and types on which it
	/// depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_session::Config + pallet_treasury::Config + pallet_balances::Config {
		/// The Event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// Origin for adding or removing a validator.
		type AddRemoveOrigin: EnsureOrigin<Self::Origin>;

		/// Minimum number of validators to leave in the validator set during
		/// auto removal.
		type MinAuthorities: Get<u32>;

		type Currency: LockableCurrency<Self::AccountId>;

		#[pallet::constant]
		type PoscanEngineId: Get<[u8; 4]>;

		#[pallet::constant]
		type FilterLevels: Get<[(u128, u32); 4]>;

		#[pallet::constant]
		type MaxMinerDepth: Get<u32>;

		type RewardLocksApi: RewardLocksApi<Self::AccountId, BalanceOf<Self>>;

		#[pallet::constant]
		type PenaltyOffline: Get<u128>;

		#[pallet::constant]
		type MinLockAmount: Get<u128>;

		#[pallet::constant]
		type MinLockPeriod: Get<u32>;

		#[pallet::constant]
		type SlashValidatorFor: Get<u32>;

		#[pallet::constant]
		type AddAfterSlashPeriod: Get<u32>;

		type Slash: OnUnbalanced<NegativeImbalanceOf<Self>>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn validators)]
	pub type Validators<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn approved_validators)]
	pub type ApprovedValidators<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn validators_to_remove)]
	pub type OfflineValidators<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn authors)]
	pub type Authors<T: Config> = StorageMap<_, Twox64Concat, T::BlockNumber, Option<T::AccountId>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn locks)]
	pub type ValidatorLock<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, Option<(T::BlockNumber, BalanceOf<T>, Option<u32>)>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn enter_deposit)]
	pub type EnterDeposit<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, Option<(T::BlockNumber, BalanceOf<T>)>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn removed)]
	pub type AccountRemoveReason<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, Option<(T::BlockNumber, RemoveReason)>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn upgrades)]
	pub type LastUpgrade<T: Config> = StorageValue<_, T::BlockNumber, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// New validator addition initiated. Effective in ~2 sessions.
		ValidatorAdditionInitiated(T::AccountId),

		/// Validator removal initiated. Effective in ~2 sessions.
		ValidatorRemovalInitiated(T::AccountId),

		ValidatorSlash(T::AccountId, BalanceOf<T>),

		ValidatorLockBalance(T::AccountId, T::BlockNumber, BalanceOf<T>, Option<u32>),

		ValidatorUnlockBalance(T::AccountId, BalanceOf<T>),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Target (post-removal) validator count is below the minimum.
		TooLowValidatorCount,
		/// Validator is already in the validator set.
		Duplicate,
		/// Validator is not approved for re-addition.
		ValidatorNotApproved,
		/// Only the validator can add itself back after coming online.
		BadOrigin,
		/// Has not mined.
		ValidatorHasNotMined,
		/// Locked amount too low.
		AmountLockedBelowLimit,
		/// decrease lock amount not allowed .
		DecreaseLockAmountNotAllowed,
		/// Decrease lcck prolongation not allowed.
		DecreaseLockPeriodNotAllowed,
		/// Lcck prolongation period too little.
		PeriodLockBelowLimit, // {pub limit: u32},
		/// No lock.
		NotLocked,
		/// Unsufficient Balance,
		UnsufficientBalance,
		/// lock ia active
		LockIsActive, // {pub upto_block: u32},
		/// temporary disalowed
		TmpDisalowed,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(n: T::BlockNumber) {
			let author = frame_system::Pallet::<T>::digest()
				.logs
				.iter()
				.filter_map(|s| s.as_pre_runtime())
				.filter_map(|(id, mut data)| {
					if id == T::PoscanEngineId::get() {
						T::AccountId::decode(&mut data).ok()
					} else {
						None
					}
				}
				)
				.next();

			if let Some(author) = author {
				let deposit = T::RewardLocksApi::locks(&author);
				let d = u128::from_le_bytes(deposit.encode().try_into().unwrap());

				log::debug!(target: LOG_TARGET, "Account: {:?}", &author);
				log::debug!(target: LOG_TARGET, "Deposit: {}", d);
				Authors::<T>::insert(n, Some(author));
			}
			else {
				log::debug!(target: LOG_TARGET, "No authon");
			}
		}

		fn on_runtime_upgrade() -> frame_support::weights::Weight {
			let current_block = frame_system::Pallet::<T>::block_number();
			<LastUpgrade<T>>::put(current_block);
			0
		}
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub initial_validators: Vec<T::AccountId>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { initial_validators: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			Pallet::<T>::initialize_validators(&self.initial_validators);
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Add a new validator.
		///
		/// New validator's session keys should be set in Session pallet before
		/// calling this.
		///
		/// The origin can be configured using the `AddRemoveOrigin` type in the
		/// host runtime. Can also be set to sudo/root.
		#[pallet::weight(0)]
		pub fn add_validator(origin: OriginFor<T>, validator_id: T::AccountId) -> DispatchResult {
			T::AddRemoveOrigin::ensure_origin(origin)?;

			Self::do_add_validator(validator_id.clone(), false)?;
			Self::approve_validator(validator_id)?;

			Ok(())
		}

		/// Remove a validator.
		///
		/// The origin can be configured using the `AddRemoveOrigin` type in the
		/// host runtime. Can also be set to sudo/root.
		#[pallet::weight(0)]
		pub fn remove_validator(
			origin: OriginFor<T>,
			validator_id: T::AccountId,
		) -> DispatchResult {
			T::AddRemoveOrigin::ensure_origin(origin)?;
			let current_block = frame_system::Pallet::<T>::block_number();

			Self::do_remove_validator(validator_id.clone())?;
			Self::unapprove_validator(validator_id.clone())?;
			AccountRemoveReason::<T>::insert(&validator_id, Some((current_block, RemoveReason::Normal)));

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn add_validator_self(origin: OriginFor<T>) -> DispatchResult {
			let validator_id = ensure_signed(origin)?;

			Self::do_add_validator(validator_id.clone(), true)?;
			Self::approve_validator(validator_id)?;

			Ok(())
		}

		/// Add an approved validator again when it comes back online.
		///
		/// For this call, the dispatch origin must be the validator itself.
		#[pallet::weight(0)]
		pub fn rejoin_validator(
			origin: OriginFor<T>,
			validator_id: T::AccountId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(who == validator_id, Error::<T>::BadOrigin);

			let approved_set: BTreeSet<_> = <ApprovedValidators<T>>::get().into_iter().collect();
			ensure!(approved_set.contains(&validator_id), Error::<T>::ValidatorNotApproved);
			let current_number = frame_system::Pallet::<T>::block_number();

			let suspend_period = T::SlashValidatorFor::get();
			let allow_period = T::AddAfterSlashPeriod::get();
			let mut check_block_num = true;
			let maybe_removed = AccountRemoveReason::<T>::get(&validator_id);

			if let Some(remove_data) = maybe_removed {
				check_block_num = false;
				match remove_data.1 {
					RemoveReason::Normal => { },
					RemoveReason::DepositBelowLimit |
					RemoveReason::ImOnlineSlash => {
						let t1 = remove_data.0 + suspend_period.into();
						let t2 = t1 + allow_period.into();

						if current_number < t1 {
							return Err(Error::<T>::TmpDisalowed.into());

						}
						else if current_number >= t2 {
							check_block_num = true;
						}
					}
				}
			}

			Self::do_add_validator(validator_id, check_block_num)?;

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn lock(
			origin: OriginFor<T>,
			amount: BalanceOf<T>,
			until: T::BlockNumber,
			period: Option<u32>,
		) -> DispatchResult {
			let validator_id = ensure_signed(origin)?;
			let min_period = T::MinLockPeriod::get();
			let free = <T as pallet::Config>::Currency::free_balance(&validator_id);
			let current_number = frame_system::Pallet::<T>::block_number();

			if free < amount {
				return Err(Error::<T>::UnsufficientBalance.into());
			}

			if until - current_number < min_period.into() {
				return Err(Error::<T>::PeriodLockBelowLimit.into());
			}

			if let Some(per) = period {
				if per < min_period {
					return Err(Error::<T>::PeriodLockBelowLimit.into());
				}
			}

			if let Some((to_block, val, _)) = ValidatorLock::<T>::get(&validator_id) {
				if amount < val {
					return Err(Error::<T>::DecreaseLockAmountNotAllowed.into());
				}
				if until < to_block {
					return Err(Error::<T>::DecreaseLockPeriodNotAllowed.into());
				}
			}

			Self::set_lock(validator_id.clone(), until, amount, period);

			Self::deposit_event(Event::ValidatorLockBalance(validator_id.clone(), until, amount, period));
			log::debug!(target: LOG_TARGET, "Locked {:?} for validator_id: {:?} up to block {:?}.", amount, validator_id, until);

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn unlock(
			origin: OriginFor<T>,
			amount: Option<BalanceOf<T>>,
		) -> DispatchResult {
			let validator_id = ensure_signed(origin)?;
			let lock_item = ValidatorLock::<T>::get(&validator_id).ok_or(Error::<T>::NotLocked)?;
			let current_number = frame_system::Pallet::<T>::block_number();

			if lock_item.0 > current_number {
				return Err(Error::<T>::LockIsActive.into())
			}

			let remove_all;
			let unlock_amount;

			if let Some(amount) = amount {
				unlock_amount = amount;
				remove_all = false;
			}
			else {
				unlock_amount = lock_item.1;
				remove_all = true;
			}

			if remove_all {
				<T as pallet::Config>::Currency::remove_lock(
					LOCK_ID,
					&validator_id,
				);
				ValidatorLock::<T>::remove(&validator_id);
			}
			else {
				let new_lock_amount = lock_item.1 - unlock_amount;
				<T as pallet::Config>::Currency::set_lock(
					LOCK_ID,
					&validator_id,
					new_lock_amount,
					WithdrawReasons::all(),
				);
				ValidatorLock::<T>::insert(&validator_id, Some((lock_item.0, new_lock_amount, lock_item.2)));
			}
			Self::deposit_event(Event::ValidatorUnlockBalance(validator_id.clone(), unlock_amount));
			log::debug!(target: LOG_TARGET, "Unlocked {:?} for validator_id: {:?}.", unlock_amount, validator_id);

			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	fn initialize_validators(validators: &[T::AccountId]) {
		assert!(validators.len() as u32 >= T::MinAuthorities::get(), "Initial set of validators must be at least T::MinAuthorities");
		assert!(<Validators<T>>::get().is_empty(), "Validators are already initialized!");

		<Validators<T>>::put(validators);
		<ApprovedValidators<T>>::put(validators);
	}

	fn do_add_validator(validator_id: T::AccountId, check_block_num: bool) -> DispatchResult {
		let cur_block_number = <frame_system::Pallet<T>>::block_number();

		let item_lock = ValidatorLock::<T>::get(&validator_id).ok_or(Error::<T>::AmountLockedBelowLimit)?;
		let deposit =
			if item_lock.0 < cur_block_number  {
				BalanceOf::<T>::zero()
			}
			else {
				item_lock.1
			};
		{
			let d = u128::from_le_bytes(deposit.encode().try_into().unwrap());
			log::debug!(target: LOG_TARGET, "Deposit: {}", d);
		}
		if check_block_num {
			let levels = T::FilterLevels::get();
			let mut depth: u32 = T::MaxMinerDepth::get();

			if deposit < levels[0].0.saturated_into() {
				log::debug!(target: LOG_TARGET, "Too low deposit to be validator");
				return Err(Error::<T>::AmountLockedBelowLimit.into());
			}

			for i in (0..levels.len()).rev() {
				if deposit >= levels[i].0.saturated_into() {
					depth = levels[i].1;
					break
				}
			}

			let mut found = false;
			let mut n = 0u32;
			loop {
				n += 1;
				let block_num = cur_block_number - n.into();
				if block_num < 1u32.into() || n > depth {
					break;
				}
				if let Some(author_id) = Authors::<T>::get(block_num) {
					if validator_id == author_id {
						log::debug!(target: LOG_TARGET, "Validator found as miner in block {:?}", block_num);
						found = true;
						break;
					}
				}
			}
			if !found {
				log::debug!(target: LOG_TARGET, "Validator NOT found as miner within {} blocks", depth);
				return Err(Error::<T>::ValidatorHasNotMined.into());
			}
		}
		else {
			if !Self::check_lock(&validator_id) {
				return Err(Error::<T>::AmountLockedBelowLimit.into());
			}
		}

		let validator_set: BTreeSet<_> = <Validators<T>>::get().into_iter().collect();
		ensure!(!validator_set.contains(&validator_id), Error::<T>::Duplicate);
		<Validators<T>>::mutate(|v| v.push(validator_id.clone()));

		EnterDeposit::<T>::insert(&validator_id, Some((cur_block_number, deposit)));

		Self::deposit_event(Event::ValidatorAdditionInitiated(validator_id.clone()));
		log::debug!(target: LOG_TARGET, "Validator addition initiated.");

		Ok(())
	}

	fn do_remove_validator(validator_id: T::AccountId) -> DispatchResult {
		let mut validators = <Validators<T>>::get();

		// Ensuring that the post removal, target validator count doesn't go
		// below the minimum.
		ensure!(
			validators.len() as u32 > T::MinAuthorities::get(),
			Error::<T>::TooLowValidatorCount
		);

		validators.retain(|v| *v != validator_id);

		<Validators<T>>::put(validators);

		Self::deposit_event(Event::ValidatorRemovalInitiated(validator_id.clone()));
		log::debug!(target: LOG_TARGET, "Validator removal initiated.");

		Ok(())
	}

	fn approve_validator(validator_id: T::AccountId) -> DispatchResult {
		let approved_set: BTreeSet<_> = <ApprovedValidators<T>>::get().into_iter().collect();
		ensure!(!approved_set.contains(&validator_id), Error::<T>::Duplicate);
		<ApprovedValidators<T>>::mutate(|v| v.push(validator_id.clone()));
		Ok(())
	}

	fn unapprove_validator(validator_id: T::AccountId) -> DispatchResult {
		let mut approved_set = <ApprovedValidators<T>>::get();
		approved_set.retain(|v| *v != validator_id);
		Ok(())
	}

	// Adds offline validators to a local cache for removal at new session.
	fn mark_for_removal(validator_id: T::AccountId, reason: RemoveReason) {
		let current_block = <frame_system::Pallet<T>>::block_number();
		AccountRemoveReason::<T>::insert(&validator_id, Some((current_block, reason)));

		<OfflineValidators<T>>::mutate(|v| v.push(validator_id.clone()));
		log::debug!(target: LOG_TARGET, "Offline validator marked for auto removal: {:#?}", validator_id);
	}

	fn slash(validator_id: &T::AccountId, amount: BalanceOf<T>) {
		let pot_id = pallet_treasury::Pallet::<T>::account_id();
		let min_bal = <T as pallet::Config>::Currency::minimum_balance();

		let zero = BalanceOf::<T>::zero();
		let maybe_lock = ValidatorLock::<T>::get(&validator_id);
		let mut usable: u128 = pallet_balances::Pallet::<T>::usable_balance(validator_id).saturated_into();

		let unlock_amount = amount - usable.saturated_into();
		if unlock_amount > zero {
			if let Some(lock_item) = maybe_lock {
				if unlock_amount <= lock_item.1 {
					Self::set_lock(
						validator_id.clone(),
						lock_item.0,
						lock_item.1 - unlock_amount,
						lock_item.2,
					);
					usable = pallet_balances::Pallet::<T>::usable_balance(validator_id).saturated_into();
				}
			}
			let unlock_amount = amount - usable.saturated_into();
			if unlock_amount > zero {
				log::debug!(target: LOG_TARGET, "Try to unlock rewards locks for {:#?}, usable: {}", validator_id, usable);
				let total_reward_locked = T::RewardLocksApi::locks(&validator_id);
				T::RewardLocksApi::unlock_upto(&validator_id, total_reward_locked - unlock_amount);
				usable = pallet_balances::Pallet::<T>::usable_balance(validator_id).saturated_into();
				log::debug!(target: LOG_TARGET, "After unlock rewards locks for {:#?} usable: {}", validator_id, usable);
			}
		}

		let amount = core::cmp::min(amount, usable.saturated_into()) - min_bal;
		let res = <T as pallet::Config>::Currency::transfer(
			&validator_id, &pot_id, amount, ExistenceRequirement::KeepAlive,
		);

		if let Err(e) = res {
			log::error!(target: LOG_TARGET, "Error slash validator {:#?} by {:?}: {:?}.", validator_id, &amount, &e);
			return
		}

		log::debug!(target: LOG_TARGET, "Slash validator {:?} by {:?}.", validator_id, &amount);
		Self::deposit_event(Event::ValidatorSlash(validator_id.clone(), amount));
	}


	// Removes offline validators from the validator set and clears the offline
	// cache. It is called in the session change hook and removes the validators
	// who were reported offline during the session that is ending. We do not
	// check for `MinAuthorities` here, because the offline validators will not
	// produce blocks and will have the same overall effect on the runtime.
	fn remove_offline_validators() {
		let validators_to_remove: BTreeSet<_> = <OfflineValidators<T>>::get().into_iter().collect();

		let mut validators = <Validators<T>>::get();
		let mut to_remove = 0;
		for r in validators_to_remove.iter() {
			if validators.len() as u32 <= T::MinAuthorities::get() {
				break
			}
			validators.retain(|v| *v != *r);
			to_remove += 1;
		}
		if to_remove > 0 {
			<Validators<T>>::put(validators);
			log::debug!(
				target: LOG_TARGET,
				"Initiated removal of {:?} offline validators.",
				to_remove,
			);
		}
		// Clear the offline validator list to avoid repeated deletion.
		<OfflineValidators<T>>::put(Vec::<T::AccountId>::new());
	}

	fn mark_if_no_locks() {
		let current_block = <frame_system::Pallet<T>>::block_number();
		if current_block < 100u32.into() {
			return
		}

		for v in Self::validators().into_iter() {
			if !Self::check_lock(&v) {
				Self::mark_for_removal(v, RemoveReason::DepositBelowLimit)
			}
		}
	}

	fn check_lock(validator_id: &T::AccountId) -> bool {
		let levels = T::FilterLevels::get();
		let zero = BalanceOf::<T>::zero();

		let maybe_enter_depo = EnterDeposit::<T>::get(&validator_id);
		let maybe_lock = ValidatorLock::<T>::get(&validator_id);
		let mut true_locked: BalanceOf<T> = zero;

		if let Some(lock) = maybe_lock {
			let current_block = frame_system::Pallet::<T>::block_number();
			true_locked = if lock.0 < current_block { zero } else { lock.1.into() };
		}

		true_locked >= maybe_enter_depo.map_or_else(|| levels[0].0.saturated_into(), |d| d.1)
	}

	fn set_lock(
		validator_id: T::AccountId,
		when: T::BlockNumber,
		amount: BalanceOf<T>,
		period: Option<u32>,
	) {
		<T as pallet::Config>::Currency::extend_lock(
			LOCK_ID,
			&validator_id,
			amount,
			WithdrawReasons::all(),
		);

		ValidatorLock::<T>::insert(&validator_id, Some((when, amount, period)));
	}

	fn renew_locks() {
		let cur_block_number = <frame_system::Pallet<T>>::block_number();

		for v in Self::validators().into_iter() {
			if let Some((when, amount, Some(period))) = ValidatorLock::<T>::get(&v) {
				if cur_block_number >= when {
					let when = ((cur_block_number - when) / period.into() + 1u32.into()) * period.into();
					Self::set_lock(v.clone(), when, amount, Some(period));
				}
			}
		}
	}

	fn is_slash_delay() -> bool {
		let s: RuntimeVersion = <T as frame_system::Config>::Version::get();
		let sv = s.spec_version;

		if sv == CUR_SPEC_VERSION {
			let current_block = frame_system::Pallet::<T>::block_number();
			let upgrade_block = <LastUpgrade<T>>::get();

			if current_block - upgrade_block <= UPGRADE_SLASH_DELAY.into() {
				return true
			}
		}
		false
	}
}

// Provides the new set of validators to the session module when session is
// being rotated.
impl<T: Config> pallet_session::SessionManager<T::AccountId> for Pallet<T> {
	// Plan a new session and provide new validator set.
	fn new_session(_new_index: u32) -> Option<Vec<T::AccountId>> {
		Self::renew_locks();

		if Self::is_slash_delay() {
			log::debug!(target: LOG_TARGET, "New session called; within slash delay.");
			<OfflineValidators<T>>::put(Vec::<T::AccountId>::new());
			return Some(Self::validators())
		}

		Self::mark_if_no_locks();
		// Remove any offline and slashed validators.
		Self::remove_offline_validators();
		log::debug!(target: LOG_TARGET, "New session called; updated validator set provided.");

		Some(Self::validators())
	}

	fn end_session(_end_index: u32) {}

	fn start_session(_start_index: u32) {}
}

impl<T: Config> EstimateNextSessionRotation<T::BlockNumber> for Pallet<T> {
	fn average_session_length() -> T::BlockNumber {
		Zero::zero()
	}

	fn estimate_current_session_progress(
		_now: T::BlockNumber,
	) -> (Option<sp_runtime::Permill>, frame_support::dispatch::Weight) {
		(None, Zero::zero())
	}

	fn estimate_next_session_rotation(
		_now: T::BlockNumber,
	) -> (Option<T::BlockNumber>, frame_support::dispatch::Weight) {
		(None, Zero::zero())
	}
}

// Implementation of Convert trait for mapping ValidatorId with AccountId.
pub struct ValidatorOf<T>(sp_std::marker::PhantomData<T>);

impl<T: Config> Convert<T::ValidatorId, Option<T::ValidatorId>> for ValidatorOf<T> {
	fn convert(account: T::ValidatorId) -> Option<T::ValidatorId> {
		Some(account)
	}
}

impl<T: Config> ValidatorSet<T::AccountId> for Pallet<T> {
	type ValidatorId = T::ValidatorId;
	type ValidatorIdOf = T::ValidatorIdOf;

	fn session_index() -> sp_staking::SessionIndex {
		pallet_session::Pallet::<T>::current_index()
	}

	fn validators() -> Vec<Self::ValidatorId> {
		pallet_session::Pallet::<T>::validators()
	}
}

impl<T: Config> ValidatorSetWithIdentification<T::AccountId> for Pallet<T> {
	type Identification = T::ValidatorId;
	type IdentificationOf = ValidatorOf<T>;
}

// Offence reporting and unresponsiveness management.
impl<T: Config, O: Offence<(T::AccountId, T::AccountId)>>
	ReportOffence<T::AccountId, (T::AccountId, T::AccountId), O> for Pallet<T>
{
	fn report_offence(_reporters: Vec<T::AccountId>, offence: O) -> Result<(), OffenceError> {
		if Self::is_slash_delay() {
			return Ok(())
		}

		let offenders = offence.offenders();
		let penalty: u128 = T::PenaltyOffline::get();
		let val: BalanceOf<T> = penalty.saturated_into();

		for (v, _) in offenders.into_iter() {
			log::debug!(target: LOG_TARGET, "offender reported: {:?}", &v);

			if !Self::validators().contains(&v) {
				continue
			}

			Self::slash(&v, val);
			Self::mark_for_removal(v, RemoveReason::ImOnlineSlash);
		}

		Ok(())
	}

	fn is_known_offence(
		_offenders: &[(T::AccountId, T::AccountId)],
		_time_slot: &O::TimeSlot,
	) -> bool {
		false
	}
}

impl<T: Config> ValidatorSetApi<T::AccountId> for Pallet<T> {
	fn validators() -> Vec<T::AccountId> {
		Self::validators()
	}
}
