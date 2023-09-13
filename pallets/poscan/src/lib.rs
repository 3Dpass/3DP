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
use sp_core::offchain::Duration;
use sp_inherents::IsFatalError;

use sp_consensus_poscan::POSCAN_ALGO_GRID2D_V3A;
use poscan_api::PoscanApi;
use validator_set_api::ValidatorSetApi;

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
const DEFAULT_OBJECT_HASHES: u32 = 10;
const MAX_OBJECT_HASHES: u32 = 256 + DEFAULT_OBJECT_HASHES;
const DEFAULT_MAX_ALGO_TIME: u32 = 10;  // 10 sec
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
	use frame_support::traits::Len;
	use frame_system::pallet_prelude::*;
	use super::*;

	#[derive(Clone, PartialEq, Encode, Decode, TypeInfo, MaxEncodedLen, RuntimeDebug)]
	pub enum Algo3D {
		Grid2dLow,
		Grid2dHigh,
	}

	#[derive(Clone, PartialEq, Encode, Decode, TypeInfo, MaxEncodedLen, RuntimeDebug)]
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

	#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, RuntimeDebug)]
	pub enum ObjectState<Block>
	where
		Block: Encode + Decode + TypeInfo + Member,
	{
		Created(Block),
		Estimating(Block),
		Estimated(Block, u64),
		NotApproved(Block),
		Approved,
	}

	#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen)]
	pub struct Approval<Account, Block>
		where
			Account: Encode + Decode + TypeInfo + Member,
			Block: Encode + Decode + TypeInfo + Member,
	{
		pub account_id: Account,
		pub when: Block,
		pub proof: Option<H256>,
	}

	#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen)]
	pub struct ObjData<Account, Block>
		where
			Account: Encode + Decode + TypeInfo + Member,
			Block: Encode + Decode + TypeInfo + Member,
	{
		pub state: ObjectState<Block>,
		pub obj: BoundedVec<u8, ConstU32<MAX_OBJECT_SIZE>>,
		pub category: ObjectCategory,
		pub hashes: BoundedVec<H256, ConstU32<MAX_OBJECT_HASHES>>,
		pub when_created: Block,
		pub when_approved: Option<Block>,
		pub owner: Account,
		pub estimators: BoundedVec<(Account, u64), ConstU32<MAX_ESTIMATORS>>,
		pub approvers: BoundedVec<Approval<Account, Block>, ConstU32<256>>,
		pub num_approvals: u8,
		pub est_rewards: u128,
		pub author_rewards: u128,
	}

	pub type ObjIdx = u32;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + CreateSignedTransaction<Call<Self>> {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type Call: From<Call<Self>>;

		#[pallet::constant]
		type PoscanEngineId: Get<[u8; 4]>;

		/// Origin for pallet admin.
		type AdminOrigin: EnsureOrigin<Self::Origin>;

		#[pallet::constant]
		type EstimatePeriod: Get<u32>;

		#[pallet::constant]
		type ApproveTimeout: Get<u32>;

		#[pallet::constant]
		type RewardsDefault: Get<u128>;

		#[pallet::constant]
		type AuthorPartDefault: Get<Percent>;

		type Currency: LockableCurrency<Self::AccountId>;

		type ValidatorSet: ValidatorSetApi<Self::AccountId, Self::BlockNumber, BalanceOf::<Self>>;

		// #[pallet::constant]MAX_OBJECT_SIZE Get<u32>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn owners)]
	pub type Owners<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, BoundedVec<ObjIdx, ConstU32<100>>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn obj_count)]
	pub type ObjCount<T> = StorageValue<_, ObjIdx, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn objects)]
	pub type Objects<T: Config> = StorageMap<_, Twox64Concat, ObjIdx, ObjData<T::AccountId, T::BlockNumber>, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn locks)]
	pub type AccountLock<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn obj_rewards)]
	pub type Rewards<T> = StorageValue<_, BalanceOf<T>, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn author_part)]
	pub type AuthorPart<T> = StorageValue<_, Percent, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn algo_time)]
	pub type MaxAlgoTime<T> = StorageValue<_, u32, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event emitted when an object has been created.
		ObjCreated(T::AccountId),
		/// Event emitted when an object has been approved.
		ObjApproved(ObjIdx),
		/// Event emitted when an object has not been approved (expired).
		ObjNotApproved(ObjIdx),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// The object exists.
		ObjectExists,
		/// Object hashes are duplicated.
		DuplicatedHashes,
		/// Zero approvals requested.
		ZeroApprovalsRequested,
		/// Unsufficient balance to process object.
		UnsufficientBalance,
		/// Unsupported category.
		UnsupportedCategory,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(now: T::BlockNumber) -> Weight {
			log::debug!(target: LOG_TARGET, "on_initialize");

			for obj_idx in Objects::<T>::iter_keys() {
				Objects::<T>::mutate(obj_idx, |obj_data| {
					match obj_data {
						Some(ref mut obj_data) => {
							match obj_data.state {
								ObjectState::Created(when) |
								ObjectState::Estimated(when, _) |
								ObjectState::Estimating(when)
								if {
									let when_last_approved = obj_data.approvers
										.last()
										.map(|a| a.when)
										.unwrap_or(when);
									now > when_last_approved + T::ApproveTimeout::get().into()
								} => {
									log::debug!(target: LOG_TARGET, "on_initialize mark as NotApproved obj_idx={}", &obj_idx);
									obj_data.state = ObjectState::NotApproved(now);
									Self::deposit_event(Event::ObjNotApproved(obj_idx));
								},
								ObjectState::Created(_) => {
									obj_data.state = ObjectState::Estimating(now);
									log::debug!(target: LOG_TARGET, "on_initialize ok for obj_idx={}", &obj_idx);
								},
								ObjectState::Estimating(block_num) => {
									log::debug!(target: LOG_TARGET, "on_initialize: Estimating");
									let est_cnt = obj_data.estimators.len();
									let majority = T::ValidatorSet::validators().len() / 2;
									if now > block_num + T::EstimatePeriod::get().into() && est_cnt >= majority {
										let t = obj_data.estimators.iter().fold(0, |_, t| t.1);
										obj_data.state = ObjectState::Estimated(now, t / (est_cnt as u64));
										log::debug!(target: LOG_TARGET, "on_initialize mark as estimated obj_idx={}", &obj_idx);
										Self::reward_estimators(obj_idx);
									}
								},
								_ => {},
							};
						}
						None => {
							log::debug!(target: LOG_TARGET, "on_initialize no object with obj_idx={}", &obj_idx);
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
		/// Put file to poscan storage
		#[pallet::weight(10_000 * obj.len() as u64)]
		pub fn put_object(
			origin: OriginFor<T>,
			category: ObjectCategory,
			obj: BoundedVec<u8, ConstU32<MAX_OBJECT_SIZE>>,
			num_approvals: u8,
			hashes: Option<BoundedVec<H256, ConstU32<MAX_OBJECT_HASHES>>>,
		) -> DispatchResultWithPostInfo {
			let acc = ensure_signed(origin)?;
			let obj_idx = <Pallet::<T>>::obj_count();

			if num_approvals == 0 {
				return Err(Error::<T>::ZeroApprovalsRequested.into());
			}

			for (_idx, obj_data) in Objects::<T>::iter() {
				if obj_data.obj == obj {
					return Err(Error::<T>::ObjectExists.into());
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
				None => Self::calc_hashes(&category, &obj, DEFAULT_OBJECT_HASHES)?,
			};

			log::debug!(target: LOG_TARGET, "put_object::hashes");
			for a in hashes.iter() {
				log::debug!(target: LOG_TARGET, "{:#?}", a);
			}

			for (_idx, obj_data) in Objects::<T>::iter() {
				let min_len = core::cmp::min(obj_data.hashes.len(), hashes.len());
				if obj_data.hashes.iter().eq(hashes[0..min_len].iter()) {
					return Err(Error::<T>::DuplicatedHashes.into());
				}
			}

			log::debug!(target: LOG_TARGET, "hashes len={}", hashes.len());

			let block_num = <frame_system::Pallet<T>>::block_number();
			let state =  ObjectState::Created(block_num);
			let obj_data = ObjData::<T::AccountId, T::BlockNumber> {
				state,
				obj,
				category,
				hashes,
				when_created: block_num,
				when_approved: None,
				owner: acc.clone(),
				estimators: BoundedVec::default(),
				approvers: BoundedVec::default(),
				num_approvals,
				est_rewards: actual_rewords.0.saturated_into(),
				author_rewards: actual_rewords.1.saturated_into(),
			};
			<Objects<T>>::insert(obj_idx, obj_data);
			<ObjCount<T>>::put(obj_idx + 1);
			let _ = <Owners<T>>::mutate(acc.clone(), |v| v.try_push(1));

			// TODO:
			Self::deposit_event(Event::ObjCreated(acc));
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
			proof: Option<H256>, // additional hash if found
		)-> DispatchResultWithPostInfo {
			ensure_none(origin)?;

			Objects::<T>::mutate(&obj_idx, |obj| {
				match obj {
					Some(ref mut obj_data) => {
						if let ObjectState::Estimated(..) = obj_data.state {
							let current_block = <frame_system::Pallet<T>>::block_number();
							let approval = Approval { account_id: author.clone(), when: current_block, proof};
							if obj_data.approvers.try_push(approval).is_err() {
								log::debug!(target: LOG_TARGET, "approve addition err for obj_idx={}", &obj_idx);
								return
							}

							let author_rewards = &obj_data.author_rewards / obj_data.num_approvals as u128;
							let tot_locked: u128 = AccountLock::<T>::get(&obj_data.owner).saturated_into();
							let new_locked: u128 = tot_locked.saturating_sub((author_rewards).saturated_into()).saturated_into();
							Self::set_lock(&obj_data.owner, new_locked.saturated_into());
							let res = <T as pallet::Config>::Currency::transfer(
								&obj_data.owner, &author, author_rewards.saturated_into(), ExistenceRequirement::KeepAlive,
							);
							match res {
								Ok(_) => {
									log::debug!(target: LOG_TARGET, "author {:#?} rewards ok for obj_idx={}", &author, &obj_idx);
								},
								Err(_) => {
									log::debug!(target: LOG_TARGET, "approve rewards err for obj_idx={}", &obj_idx);
								}
							};
							if obj_data.approvers.len() >= obj_data.num_approvals as usize {
								obj_data.when_approved = Some(current_block);
								obj_data.state = ObjectState::Approved;
								Self::deposit_event(Event::ObjApproved(obj_idx));
								log::debug!(target: LOG_TARGET, "approve applyed for obj_idx={}", &obj_idx);

							}
						}
						else {
							log::debug!(target: LOG_TARGET, "approve invalid state ({:#?}) for obj_idx={}", &obj_data.state, &obj_idx);
						}
					},
					None => log::debug!(target: LOG_TARGET, "approve no object with obj_idx={}", &obj_idx),
				}
			});

			Ok(().into())
		}

		#[pallet::weight(1_000_000)]
		pub fn set_algo_time(
			origin: OriginFor<T>,
			algo_time: u32,
		) -> DispatchResultWithPostInfo {
			T::AdminOrigin::ensure_origin(origin)?;

			MaxAlgoTime::<T>::put(algo_time);
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
				let proof_num: u32 = (obj_data.hashes.len() + obj_data.approvers.len()) as u32 + 1;
				let (res, proof) = Self::check_hashes(&obj_data, proof_num);

				if res {
					log::debug!(target: LOG_TARGET, "create_inherent: hashes true");
					Some(Call::approve { author, obj_idx, proof })
				}
				else {
					log::debug!(target: LOG_TARGET, "create_inherent: hashes false");
					None
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
					if let Ok(is_light) = poscan_algo::hashable_object::is_light() {
						if is_light {
							return Ok(())
						}
					}

					let obj_data = &Objects::<T>::get(obj_idx);
					if let Some(obj_data) = obj_data {
						let hashes = &obj_data.hashes;

						let proof_num: u32 = (obj_data.hashes.len() + obj_data.approvers.len()) as u32;
						let (res, _) = Self::check_hashes(&obj_data, proof_num);

						if res {
							log::debug!(target: LOG_TARGET, "check_inherent: hashes true");

							for (idx, obj_data) in Objects::<T>::iter() {
								if idx != *obj_idx {
									let min_len = core::cmp::min(obj_data.hashes.len(), hashes.len());
									if obj_data.hashes.iter().eq(hashes[0..min_len].iter()) {
										log::error!(target: LOG_TARGET, "check_inherent: for obj_idx={} duplicated hashes found in obj_idx={}", obj_idx, &idx);
										return Err(Self::Error::DuplicatedObjectHashes(*obj_idx, idx))
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
	pub fn created_objects() -> Vec<(u32, ObjData<T::AccountId, T::BlockNumber>)> {
		log::debug!(target: LOG_TARGET, "Select estimated");

		let mut v: Vec<(u32, ObjData<T::AccountId, T::BlockNumber>)> = Vec::new();
		for (idx, obj_data) in Objects::<T>::iter() {
			if let ObjectState::Created(_) = obj_data.state {
				v.push((idx, obj_data));
			}
		}
		log::debug!(target: LOG_TARGET, "Select created {} object(s)", v.len());
		v
	}

	pub fn estimating_objects() -> Vec<(u32, Vec<u8>)> {
		Vec::new()
	}

	pub fn estimated_objects() -> Vec<(u32, ObjData<T::AccountId, T::BlockNumber>)> {
		log::debug!(target: LOG_TARGET, "Select estimated");

		let mut v: Vec<(u32, ObjData<T::AccountId, T::BlockNumber>)> = Vec::new();
		for (idx, obj_data) in Objects::<T>::iter() {
			if let ObjectState::Estimated(_, diff) = obj_data.state {
				if diff < Self::max_algo_time().millis() {
					v.push((idx, obj_data));
				}
				else {
					log::info!(target: LOG_TARGET, "Estimation of object {} is too big", &idx);
				}
			}
		}
		log::debug!(target: LOG_TARGET, "Estimated {} object(s)", v.len());
		v
	}

	pub fn add_obj_estimation(acc: &T::AccountId, obj_idx: u32, dt: u64) {
		log::debug!(target: LOG_TARGET, "add_obj_estimation");

		Objects::<T>::mutate(obj_idx, |obj_data| {
			match obj_data {
				Some(obj_data) => {
					if let ObjectState::Estimating(..) = obj_data.state {
						let mut a: Vec<_> = obj_data.estimators.to_vec();
						a.push((acc.clone(), dt));
						obj_data.estimators = a.try_into().unwrap();
						log::debug!(target: LOG_TARGET, "add_obj_estimation ok for obj_idx={}", &obj_idx);
					}
				},
				None => log::debug!(target: LOG_TARGET, "add_obj_estimation no object with obj_idx={}", &obj_idx),
			}
		});
	}

	fn reward_estimators(obj_idx: ObjIdx) {
		log::debug!(target: LOG_TARGET, "reward_estimators");

		let obj_data = Objects::<T>::get(&obj_idx).unwrap();
		let rewards = Self::rewards(Some(obj_idx));

		if let Some((est_rewards, _)) = rewards {
			let owner = obj_data.owner;
			// TODO:
			let rewards_by_est: u128 = est_rewards.saturated_into::<u128>() / (obj_data.estimators.len() as u128);
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

	fn rewards(obj_idx: Option<ObjIdx>) -> Option<(BalanceOf::<T>, BalanceOf::<T>)> {
		if let Some(obj_idx) = obj_idx {
			let obj_data = &Objects::<T>::get(obj_idx);
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

	fn calc_hashes(cat: &ObjectCategory, obj: &Vec<u8>, num_hashes: u32) -> Result<BoundedVec<H256, ConstU32<MAX_OBJECT_HASHES>>, Error::<T>> {
		match cat {
			ObjectCategory::Objects3D(Algo3D::Grid2dLow) => {
				Ok(poscan_algo::hashable_object::calc_obj_hashes_n(
					&POSCAN_ALGO_GRID2D_V3A, &obj, &H256::default(), num_hashes,
				).try_into().unwrap())
			},
			_ => {
				Err(Error::UnsupportedCategory)
			}
		}
	}

	fn check_hashes(obj_data: &ObjData<T::AccountId, T::BlockNumber>, proof_num: u32) -> (bool, Option<H256>) {
		let res_hashes =
			Self::calc_hashes(&obj_data.category, &obj_data.obj, proof_num.clone());

		if res_hashes.is_err() {
			return (false, None)
		}

		let mut hashes: Vec<_> = res_hashes.unwrap()
			.iter()
			.map(|a| Some(*a))
			.collect();

		let mut all_prev_hashes = // : BoundedVec<H256, ConstU32<MAX_OBJECT_HASHES>> =
			obj_data.hashes
				.iter()
				.map(|a| Some(*a))
				.collect::<Vec<_>>();
		all_prev_hashes.extend(obj_data.approvers.iter().map(|a| a.proof));

		hashes.extend(sp_std::iter::repeat(None).take(proof_num as usize - hashes.len()));
		let proof = *hashes.get(proof_num as usize - 1).unwrap_or(&None);
		log::debug!(target: LOG_TARGET, "all_prev_hashes len={}", all_prev_hashes.len());
		log::debug!(target: LOG_TARGET, "hashes len={}", hashes.len());

		log::debug!(target: LOG_TARGET, "all_prev_hashes");
		for a in all_prev_hashes.iter() {
			log::debug!(target: LOG_TARGET, "{:#?}", &a);
		}

		log::debug!(target: LOG_TARGET, "hashes");
		for a in hashes.iter() {
			log::debug!(target: LOG_TARGET, "{:#?}", &a);
		}

		if all_prev_hashes == hashes[0..all_prev_hashes.len()].to_vec() {
			log::debug!(target: LOG_TARGET, "create_inherent: hashes true");
			(true, proof)
		}
		else {
			log::debug!(target: LOG_TARGET, "create_inherent: hashes false");
			(false, proof)
		}
	}

	pub fn max_algo_time() -> Duration {
		let tout = MaxAlgoTime::<T>::get().unwrap_or(DEFAULT_MAX_ALGO_TIME);
		Duration::from_millis(tout as u64 * 1000)
	}
}


impl<T: Config> PoscanApi for Pallet<T> {
	fn uncompleted_objects() -> Option<Vec<(u32, Vec<u8>)>> {
		None
	}
}
