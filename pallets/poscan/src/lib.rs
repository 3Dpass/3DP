#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/v3/runtime/frame>
pub use pallet::*;
extern crate alloc;

use sp_std::vec::Vec;
// use sp_timestamp::{InherentError, InherentType, INHERENT_IDENTIFIER};
// use sp_timestamp::InherentError;

use frame_system::offchain::{
	CreateSignedTransaction,
	//SubmitTransaction,
};
use frame_support::{
	dispatch::DispatchResultWithPostInfo,
	traits::{
		LockIdentifier,
		WithdrawReasons,
		LockableCurrency,
		Currency,
		ExistenceRequirement,
	},
	pallet_prelude::*,
};

use sp_runtime::SaturatedConversion;
pub use sp_runtime::Percent;
use sp_runtime::traits::{Saturating, Zero};

// use sp_core::crypto::AccountId32;
use sp_core::H256;
use sp_inherents::IsFatalError;

use sp_consensus_poscan::POSCAN_ALGO_GRID2D_V3_1;
use poscan_api::PoscanApi;

pub mod inherents;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub type BalanceOf<T> =
<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

const MAX_OBJECT_SIZE: u32 = 100_000;
const MAX_OBJECT_HASHES: u32 = 10;
const MAX_ESTIMATORS: u32 = 1000;
const LOCK_ID: LockIdentifier = *b"poscan  ";

pub const LOG_TARGET: &str = "runtime::poscan";

#[derive(Encode, sp_runtime::RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Decode, thiserror::Error))]
pub enum InherentError {
	/// Object not found.
	#[cfg_attr(feature = "std", error("Object not found. object_idx {0}."))]
	ObjectNotFound(u32),
	/// Object hashes are invalid.
	#[cfg_attr(feature = "std", error("Invalid hashes for object_idx {0}."))]
	InvalidObjectHashes(u32),
	/// Object hashes are duplicated.
	#[cfg_attr(feature = "std", error("Duplicated hashes for object_idx {0} and {1}."))]
	DuplicatedObjectHashes(u32, u32),
}

impl IsFatalError for InherentError {
	fn is_fatal_error(&self) -> bool {
		match self {
			InherentError::ObjectNotFound(_) => true,
			InherentError::InvalidObjectHashes(_) => true,
			InherentError::DuplicatedObjectHashes(..) => true,
		}
	}
}


#[frame_support::pallet]
pub mod pallet {
	// use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
	use super::*;
	// type MAX_OBJECT_SIZE = ConstU32<100_000>;

	#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, RuntimeDebug)]
	pub enum ObjectState<Account, Block>
	where
		Account: Encode + Decode + TypeInfo + Member,
		Block: Encode + Decode + TypeInfo + Member,
	{
		Created(Block),
		Estimating(Block, u128), // number of estimations, total time
		Estimated(u128),
		Approved(Account),
	}

	#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen)]
	pub struct ClaimData<Account, Block>
	where
		Account: Encode + Decode + TypeInfo + Member,
		Block: Encode + Decode + TypeInfo + Member,
	{
		pub state: ObjectState<Account, Block>,
		pub obj: BoundedVec<u8, ConstU32<MAX_OBJECT_SIZE>>,
		pub hashes: BoundedVec<H256, ConstU32<MAX_OBJECT_HASHES>>,
		// pub estimation: Option<u128>,
		pub owner: Account,
		pub estimators: BoundedVec<(Account, u128), ConstU32<MAX_ESTIMATORS>>,
		pub cnt: u32,
		pub est_rewards: u128,
		pub author_rewards: u128,
	}

	pub type ClaimIdx = u32;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + CreateSignedTransaction<Call<Self>> {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type Call: From<Call<Self>>;

		#[pallet::constant]
		type PoscanEngineId: Get<[u8; 4]>;

		#[pallet::constant]
		type EstimatePeriod: Get<u32>;

		#[pallet::constant]
		type RewardsDefault: Get<u128>;

		#[pallet::constant]
		type AuthorPartDefault: Get<Percent>;

		type Currency: LockableCurrency<Self::AccountId>;

		// #[pallet::constant]MAX_OBJECT_SIZE Get<u32>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn owners)]
	pub type Owners<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, BoundedVec<ClaimIdx, ConstU32<100>>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn claim_count)]
	pub type ClaimCount<T> = StorageValue<_, ClaimIdx, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn claims)]
	pub type Claims<T: Config> = StorageMap<_, Twox64Concat, ClaimIdx, ClaimData<T::AccountId, T::BlockNumber>, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn locks)]
	pub type AccountLock<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn obj_rewards)]
	pub type Rewards<T> = StorageValue<_, BalanceOf<T>, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn author_part)]
	pub type AuthorPart<T> = StorageValue<_, Percent, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event emitted when a proof has been claimed. [who, claim]
		ClaimCreated(T::AccountId),
		/// Event emitted when a claim is revoked by the owner. [who, claim]
		GetMiningObject(Vec<u8>),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// The proof has already been claimed.
		ProofAlreadyClaimed,
		/// Object hashes are duplicated.
		DuplicatedHashes,
		/// The proof does not exist, so it cannot be revoked.
		NoSuchProof,
		/// The proof is claimed by another account, so caller can't revoke it.
		NotProofOwner,
		/// Unsufficient balance for object claim.
		UnsufficientBalance,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(now: T::BlockNumber) -> Weight {
			log::debug!(target: LOG_TARGET, "on_initialize");

			for obj_idx in Claims::<T>::iter_keys() {
				Claims::<T>::mutate(obj_idx, |claim| {
					match claim {
						Some(ref mut claim) => {
							match claim.state {
								ObjectState::Created(_) => {
									claim.state = ObjectState::Estimating(now, 0);
									log::debug!(target: LOG_TARGET, "on_initialize ok for obj_idx={}", &obj_idx);
								},
								ObjectState::Estimating(block_num, t) => {
									log::debug!(target: LOG_TARGET, "on_initialize: Estimating");

									if now > block_num + T::EstimatePeriod::get().into() && claim.cnt > 0 {
										claim.state = ObjectState::Estimated(t / (claim.cnt as u128));
										log::debug!(target: LOG_TARGET, "on_initialize mark as estimated obj_idx={}", &obj_idx);
										Self::reward_estimators(obj_idx);
									}
								},
								_ => {},
							};
						},
						None => {
							log::debug!(target: LOG_TARGET, "on_initialize no claim with obj_idx={}", &obj_idx);
						},
					}
				});
			}
			// TODO:
			0
		}
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10_000 * obj.len() as u64)]
		pub fn put_object(
			origin: OriginFor<T>,
			obj: BoundedVec<u8, ConstU32<MAX_OBJECT_SIZE>>,
			hashes: Option<BoundedVec<H256, ConstU32<MAX_OBJECT_HASHES>>>,
		) -> DispatchResultWithPostInfo {
			let acc = ensure_signed(origin)?;
			let claim_idx = <Pallet::<T>>::claim_count();

			for (_idx, claim) in Claims::<T>::iter() {
				if claim.obj == obj {
					return Err(Error::<T>::ProofAlreadyClaimed.into());
				}
			}

			let actual_rewords = Self::rewards(None).unwrap();
			let lock_amount = actual_rewords.0 + actual_rewords.1;
			let free = <T as pallet::Config>::Currency::free_balance(&acc);
			if free < lock_amount {
				return Err(Error::<T>::UnsufficientBalance.into());
			}
			Self::set_lock(&acc, lock_amount);

			let hashes = match hashes {
				Some(hashes) => hashes,
				None => poscan_algo::hashable_object::calc_obj_hashes(
							&POSCAN_ALGO_GRID2D_V3_1, &obj, &H256::default()
					).try_into().unwrap(),
			};

			for (_idx, claim) in Claims::<T>::iter() {
				let min_len = core::cmp::min(claim.hashes.len(), hashes.len());
				if claim.hashes.iter().eq(hashes[0..min_len].iter()) {
					return Err(Error::<T>::DuplicatedHashes.into());
				}
			}

			log::debug!(target: LOG_TARGET, "hashes len={}", hashes.len());

			let block_num = <frame_system::Pallet<T>>::block_number();
			let state =  ObjectState::Created(block_num);
			let claim_data = ClaimData::<T::AccountId, T::BlockNumber> {
				state,
				obj,
				hashes,
				cnt: 0,
				owner: acc.clone(),
				estimators: BoundedVec::default(),
				est_rewards: actual_rewords.0.saturated_into(),
				author_rewards: actual_rewords.1.saturated_into(),
			};
			<Claims<T>>::insert(claim_idx, claim_data);
			<ClaimCount<T>>::put(claim_idx + 1);
			let _ = <Owners<T>>::mutate(acc.clone(), |v| v.try_push(1));

			// TODO:
			Self::deposit_event(Event::ClaimCreated(acc));
			Ok(().into())
		}

		#[pallet::weight(1_000_000_000)]
		pub fn get_object_ext(
			origin: OriginFor<T>,
		) -> DispatchResultWithPostInfo {
			ensure_signed(origin)?;
			// TODO:
			Ok(().into())
		}

		#[pallet::weight(1_000_000_000)]
		pub fn approve(
			origin: OriginFor<T>,
			author: T::AccountId,
			obj_idx: u32,
			_ok: bool,
		)-> DispatchResultWithPostInfo {
			ensure_none(origin)?;

			Claims::<T>::mutate(&obj_idx, |claim| {
				match claim {
					Some(ClaimData {ref mut state, ref owner, ref author_rewards, ..} ) => {
						// TODO:
						if let ObjectState::Estimated(_) = state {
							*state = ObjectState::Approved(author.clone());
							log::debug!(target: LOG_TARGET, "approve applyed for obj_idx={}", &obj_idx);

							let tot_locked: u128 = AccountLock::<T>::get(&owner).saturated_into();
							let new_locked: u128 = tot_locked.saturating_sub((*author_rewards).saturated_into()).saturated_into();
							Self::set_lock(&owner, new_locked.saturated_into());
							let res = <T as pallet::Config>::Currency::transfer(
								owner, &author, (*author_rewards).saturated_into(), ExistenceRequirement::KeepAlive,
							);
							match res {
								Ok(_) => {
									log::debug!(target: LOG_TARGET, "author rewards ok for obj_idx={}", &obj_idx);
								},
								Err(_) => {
									log::debug!(target: LOG_TARGET, "approve rewards err for obj_idx={}", &obj_idx);
								}
							};
						}
						else {
							log::debug!(target: LOG_TARGET, "approve invalid state ({:?}) for obj_idx={}", state, &obj_idx);
						}
					},
					None => log::debug!(target: LOG_TARGET, "approve no claim with obj_idx={}", &obj_idx),
				}
			});

			Ok(().into())
		}
	}

	#[pallet::inherent]
	impl<T: Config> ProvideInherent for Pallet<T> {
		type Call = Call<T>;
		type Error = super::InherentError;
		// const INHERENT_IDENTIFIER: InherentIdentifier = INHERENT_IDENTIFIER;
		const INHERENT_IDENTIFIER: InherentIdentifier = *b"p3d     ";

		fn create_inherent(_data: &InherentData) -> Option<Self::Call> {
			log::debug!(target: LOG_TARGET, "create_inherent");

			let objects = Self::estimated_objects();
			if objects.len() > 0 {
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
					.next().unwrap();

				let obj_idx = objects[0].0;
				let obj_data = &objects[0].1;

				let hashes: BoundedVec<H256, ConstU32<MAX_OBJECT_HASHES>> =
					poscan_algo::hashable_object::calc_obj_hashes(
						&POSCAN_ALGO_GRID2D_V3_1, &obj_data.obj, &H256::default()
					)
					.try_into().unwrap();

				if obj_data.hashes == hashes {
					log::debug!(target: LOG_TARGET, "create_inherent: hashes true");
					Some(Call::approve { author, obj_idx, ok: true })
				}
				else {
					log::debug!(target: LOG_TARGET, "create_inherent: hashes false");
					Some(Call::approve { author, obj_idx, ok: false })
				}
			}
			else {
				None
			}
		}

		fn check_inherent(
			call: &Self::Call,
			_data: &InherentData,
		) -> Result<(), Self::Error> {
			log::debug!(target: LOG_TARGET, "check_inherent");

			let t: Result<(), Self::Error> = match call {
				Call::approve { obj_idx, .. } => {
					let obj_data = &Claims::<T>::get(obj_idx);
					if let Some(obj_data) = obj_data {
						let hashes: BoundedVec<H256, ConstU32<MAX_OBJECT_HASHES>> =
							poscan_algo::hashable_object::calc_obj_hashes(
								&POSCAN_ALGO_GRID2D_V3_1, &obj_data.obj, &H256::default()
							).try_into().unwrap();

						if obj_data.hashes == hashes {
							log::debug!(target: LOG_TARGET, "check_inherent: hashes true");

							for (claim_idx, claim_data) in Claims::<T>::iter() {
								if claim_idx != *obj_idx {
									let min_len = core::cmp::min(claim_data.hashes.len(), hashes.len());
									if claim_data.hashes.iter().eq(hashes[0..min_len].iter()) {
										return Err(Self::Error::DuplicatedObjectHashes(*obj_idx, claim_idx))
									}
								}
							}

							Ok(())
						}
						else {
							log::debug!(target: LOG_TARGET, "check_inherent: hashes false");
							Err(Self::Error::InvalidObjectHashes(*obj_idx))
						}
					}
					else {
						Err(Self::Error::ObjectNotFound(*obj_idx))
					}
				}
				_ => Ok(()),
			};
			t
		}

		fn is_inherent(call: &Self::Call) -> bool {
			log::debug!(target: LOG_TARGET, "is_inherent");

			matches!(call, Call::approve { .. })
		}
	}
}

impl<T: Config> Pallet<T> {
	pub fn created_objects() -> Vec<(u32, ClaimData<T::AccountId, T::BlockNumber>)> {
		log::debug!(target: LOG_TARGET, "Select estimated");

		let mut v: Vec<(u32, ClaimData<T::AccountId, T::BlockNumber>)> = Vec::new();
		for (idx, claim) in Claims::<T>::iter() {
			if let ObjectState::Created(_) = claim.state {
				v.push((idx, claim));
			}
		}
		log::debug!(target: LOG_TARGET, "Select created {} claim(s)", v.len());
		v
	}

	pub fn estimating_objects() -> Vec<(u32, Vec<u8>)> {
		Vec::new()
	}

	pub fn estimated_objects() -> Vec<(u32, ClaimData<T::AccountId, T::BlockNumber>)> {
		log::debug!(target: LOG_TARGET, "Select estimated");

		let mut v: Vec<(u32, ClaimData<T::AccountId, T::BlockNumber>)> = Vec::new();
		for (idx, claim) in Claims::<T>::iter() {
			if let ObjectState::Estimated(diff) = claim.state {
				if diff < 10 {
					v.push((idx, claim));
				}
				else {
					log::info!(target: LOG_TARGET, "Estimation of claim {} is too big", &idx);
				}
			}
		}
		log::debug!(target: LOG_TARGET, "Estimated {} claim(s)", v.len());
		v
	}

	pub fn add_obj_estimation(acc: &T::AccountId, obj_idx: u32, dt: u128) {
		log::debug!(target: LOG_TARGET, "add_obj_estimation");

		Claims::<T>::mutate(obj_idx, |claim| {
			match claim {
				// Some(ClaimData {state: ObjectState::Estimating(_, ref mut cnt, ref mut t), ..} ) => {
				Some(obj_data) => { // {state: ObjectState::Estimating(_, ref mut cnt, ref mut t), ref mut estimators, ..} ) => {
					// *t += &dt;
					// let mut new_obj_data = obj_data.clone();
					if let ObjectState::Estimating(..) = obj_data.state {
						obj_data.cnt = &obj_data.cnt + 1;
						let mut a: Vec<_> = obj_data.estimators.to_vec();
						a.push((acc.clone(), dt));
						obj_data.estimators = a.try_into().unwrap();
						log::debug!(target: LOG_TARGET, "add_obj_estimation ok for obj_idx={}", &obj_idx);
					}
				},
				// Some(_st) => log::debug!(target: LOG_TARGET, "add_obj_estimation invalide state for obj_idx={}", &obj_idx),
				None => log::debug!(target: LOG_TARGET, "add_obj_estimation no claim with obj_idx={}", &obj_idx),
			}
		});
	}

	// fn lock(account_id: &T::AccountId, lock_amount: &BalanceOf<T>) -> DispatchResult {
	// 	<T as pallet::Config>::Currency::set_lock(
	// 		LOCK_ID,
	// 		&account_id,
	// 		*lock_amount,
	// 		WithdrawReasons::all(),
	// 	);
	// 	let _ = AccountLock::<T>::mutate(account_id, |_v| lock_amount);
	//
	// 	Ok(())
	// }

	fn reward_estimators(obj_idx: ClaimIdx) {
		log::debug!(target: LOG_TARGET, "reward_estimators");

		// TODO: remove lock and rewards estimators
		// let current_number = frame_system::Pallet::<T>::block_number();
		let obj_data = Claims::<T>::get(&obj_idx).unwrap();
		// let n_est = obj_data.estimators.len();
		let rewards = Self::rewards(Some(obj_idx));

		if let Some((est_rewards, _)) = rewards {
			let owner = obj_data.owner;
			// TODO:
			let rewards_by_est: u128 = est_rewards.saturated_into::<u128>() / (obj_data.cnt as u128);
			let mut payed_rewards: BalanceOf<T> = BalanceOf::<T>::default();
			log::debug!(target: LOG_TARGET, "estimator rewards: {:?}", rewards_by_est);

			for (acc, _est) in obj_data.estimators.iter() {
				let _res = <T as pallet::Config>::Currency::transfer(
					&owner, &acc, rewards_by_est.saturated_into(), ExistenceRequirement::KeepAlive,
				);
				payed_rewards = payed_rewards.saturating_add(rewards_by_est.saturated_into());
			}

			let tot_locked: u128 = AccountLock::<T>::get(&owner).saturated_into();
			let amount = est_rewards.saturating_sub(payed_rewards.saturated_into());
			let new_locked = tot_locked.saturating_sub(amount.saturated_into());
			Self::set_lock(&owner, new_locked.saturated_into());
		}
	}

	pub fn set_lock(account_id: &T::AccountId, amount: BalanceOf::<T>) {
		if amount == BalanceOf::<T>::zero() {
			<T as pallet::Config>::Currency::remove_lock(LOCK_ID, &account_id);
		}
		else {
			<T as pallet::Config>::Currency::set_lock(
				LOCK_ID,
				account_id,
				amount,
				WithdrawReasons::all(),
			);
		}
		let _ = AccountLock::<T>::mutate(account_id, |_v| amount);
	}

	pub fn get_inherent_provider(author: &Vec<u8>) -> self::inherents::InherentDataProvider {
		self::inherents::InherentDataProvider {
			author: Some(author.clone()),
			obj_idx: None,
			obj: None,
		}
	}

	fn rewards(obj_idx: Option<ClaimIdx>) -> Option<(BalanceOf::<T>, BalanceOf::<T>)> {
		if let Some(obj_idx) = obj_idx {
			let obj_data = &Claims::<T>::get(obj_idx);
			if let Some(obj_data) = obj_data {
				Some((obj_data.est_rewards.saturated_into(), obj_data.author_rewards.saturated_into()))
			}
			else {
				None
			}
		}
		else {
			let author_part = AuthorPart::<T>::get().unwrap_or(T::AuthorPartDefault::get());
			let rewards = Rewards::<T>::get().unwrap_or(T::RewardsDefault::get().saturated_into());
			let author_rewards = author_part * rewards;
			let est_rewards = rewards.saturating_sub(author_rewards);
			Some((est_rewards, author_rewards))
		}

	}
}


impl<T: Config> PoscanApi for Pallet<T> {
	fn uncompleted_objects() -> Option<Vec<(u32, Vec<u8>)>> {
		None
	}
}
