//! # PoScan Pallet
//!
//! This file is a part of 3DPass (3dpass.org).
//! Copyright (c) 2023 3DPass.
//!
//! The PoScan Pallet is a part of 3DPRC-2 the object tokenization standard implementation
//! (https://github.com/3Dpass/whitepaper/blob/main/3DPRC-2.md),
//! which allows for the user objects authentication and its protection
//! from being copied within "The Ledger of Things" platform.
//!
//! Every object tokenized acquires its unique on-chain identity called HASH ID. By means of utilization of 
//! recognition algorithms implemented (p3d recognition toolkit is being used: https://github.com/3Dpass/p3d),
//! all the assets approved by the network, will be protected from being copied to the extent of the error 
//! of the algorithm precision.
//! 
//! 
//! 

#![cfg_attr(not(feature = "std"), no_std)]

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

use poscan_api::PoscanApi;
use validator_set_api::ValidatorSetApi;

use sp_consensus_poscan::{
	Algo3D,
	ObjectCategory,
	ObjectState,
	NotApprovedReason,
	CompressWith,
	Approval,
	ObjData,
	OldObjData,
	ObjIdx,
	POSCAN_ALGO_GRID2D_V3A,
	MAX_OBJECT_SIZE,
	MAX_PROPERTIES,
	PROP_NAME_LEN,
	DEFAULT_OBJECT_HASHES,
	MAX_OBJECT_HASHES,
	DEFAULT_MAX_ALGO_TIME,
	MAX_ESTIMATORS,
	FEE_PER_BYTE,
	PropIdx,
	Property,
	PropClass,
	PropValue,
};

pub mod inherents;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub type BalanceOf<T> =
<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

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

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
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
	#[pallet::getter(fn prop_count)]
	pub type PropCount<T> = StorageValue<_, PropIdx, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn properties)]
	pub type Properties<T: Config> = StorageMap<_, Twox64Concat, PropIdx, Property, OptionQuery>;

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

	#[pallet::storage]
	#[pallet::getter(fn fee_per_byte)]
	pub type FeePerByte<T> = StorageValue<_, u64, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event emitted when an object has been created.
		ObjCreated(T::AccountId),
		/// Event emitted when an object has been approved.
		ObjApproved(ObjIdx),
		/// Event emitted when an object has not been approved (expired).
		ObjNotApproved(ObjIdx, NotApprovedReason),
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
		/// No hashes provided.
		NoHashes,
		/// Object not found
		ObjectNotFound,
		/// Origin is not owner
		NotOwner,
		/// Unsufficent funds
		UnsufficentFunds,
		/// Too many properties
		TooManyProperties,
		/// Unknown property
		UnknownProperty,
		/// Property value too big
		InvalidPropValue,
		/// Share property must be multiple 10
		InvalidSharePropValue,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(now: T::BlockNumber) -> Weight {
			log::debug!(target: LOG_TARGET, "on_initialize");

			use sp_consensus_poscan::REJECT_OLD_ALGO_SINCE;
			if now >= (REJECT_OLD_ALGO_SINCE + 1).into() {
				poscan_algo::hashable_object::fake_test();
			}
			poscan_algo::hashable_object::try_call();
			poscan_algo::hashable_object::try_call_128();

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
									obj_data.est_outliers = Self::outliers(&obj_data.estimators);
									Self::deposit_event(Event::ObjNotApproved(obj_idx, NotApprovedReason::Expired));

								},
								ObjectState::Created(_) => {
									obj_data.state = ObjectState::Estimating(now);
									log::debug!(target: LOG_TARGET, "on_initialize ok for obj_idx={}", &obj_idx);
								},
								ObjectState::Estimating(block_num) => {
									log::debug!(target: LOG_TARGET, "on_initialize: Estimating");
									if now > block_num + T::EstimatePeriod::get().into() {
										obj_data.est_outliers = Self::outliers(&obj_data.estimators);
										let n_est = obj_data.estimators.len() - obj_data.est_outliers.len();
										let n_val = T::ValidatorSet::validators().len();

										if n_est * 2 >= n_val {
											let t = obj_data.estimators
												.iter()
												.filter(|a| !obj_data.est_outliers.contains(&a.0))
												.fold(0, |_, t| t.1
												);
											obj_data.state = ObjectState::Estimated(now, t / (n_est as u64));
											log::debug!(target: LOG_TARGET, "on_initialize mark as estimated obj_idx={}", &obj_idx);
											Self::reward_estimators(obj_idx);
										}
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

		fn on_runtime_upgrade() -> frame_support::weights::Weight {
			let onchain_version =  Pallet::<T>::on_chain_storage_version();
			log::info!(target: LOG_TARGET, "on_runtime_upgrade: onchain_version={:?}", &onchain_version);
			if onchain_version < 2 {
			let prop_idx = <Pallet::<T>>::prop_count();
				log::info!(target: LOG_TARGET, "on_runtime_upgrade: prop_idx={}", &prop_idx);
			if prop_idx == 0 {
				let name: BoundedVec<u8, ConstU32<PROP_NAME_LEN>> = "Non-Fungible".as_bytes()
					.to_vec().try_into().unwrap();
					Properties::<T>::insert(0, Property { name, class: PropClass::Relative, max_value: 1 });

				let name: BoundedVec<u8, ConstU32<PROP_NAME_LEN>> = "Share".as_bytes()
					.to_vec().try_into().unwrap();
					Properties::<T>::insert(1, Property { name, class: PropClass::Relative, max_value: 100_000_000 });
				<PropCount<T>>::put(2);
			}

				Objects::<T>::translate::<OldObjData<T::AccountId, T::BlockNumber>, _>(|key, old_data| {
					log::info!(target: LOG_TARGET, "on_runtime_upgrade: migrate object: {}", &key);

				let mut properties = BoundedVec::default();
					properties.try_push(PropValue { prop_idx: 0, max_value: 1 }).unwrap();

				Some(ObjData::<T::AccountId, T::BlockNumber> {
					state: old_data.state,
					obj: old_data.obj,
					compressed_with: old_data.compressed_with,
					category: old_data.category,
					hashes: old_data.hashes,
					when_created: old_data.when_created,
					when_approved: old_data.when_approved,
					owner: old_data.owner,
					estimators: old_data.estimators,
					est_outliers: old_data.est_outliers,
					approvers: old_data.approvers,
					num_approvals: old_data.num_approvals,
					est_rewards: old_data.est_rewards,
					author_rewards: old_data.author_rewards,
					prop: properties,
				})
			});
				StorageVersion::new(2).put::<Pallet::<T>>();
			}
			else {
				log::info!(target: LOG_TARGET, "on_runtime_upgrade: unused migration");
			}
			0
		}
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Put file to poscan storage
		#[pallet::weight(<Pallet::<T>>::fee_per_byte().unwrap_or(FEE_PER_BYTE) * obj.len() as u64)]
		pub fn put_object(
			origin: OriginFor<T>,
			category: ObjectCategory,
			obj: BoundedVec<u8, ConstU32<MAX_OBJECT_SIZE>>,
			num_approvals: u8,
			hashes: Option<BoundedVec<H256, ConstU32<MAX_OBJECT_HASHES>>>,
			properties: BoundedVec<PropValue, ConstU32<MAX_PROPERTIES>>
		) -> DispatchResultWithPostInfo {
			let acc = ensure_signed(origin)?;
			let obj_idx = <Pallet::<T>>::obj_count();

			if num_approvals == 0 {
				return Err(Error::<T>::ZeroApprovalsRequested.into());
			}

			let compress_with = CompressWith::Lzss;
			let compressed_obj: BoundedVec<u8, ConstU32<MAX_OBJECT_SIZE>> =
				compress_with.compress(obj.to_vec()).try_into().unwrap();

			for (_idx, obj_data) in Objects::<T>::iter() {
				match obj_data.compressed_with {
					None if obj_data.obj == obj => {
						return Err(Error::<T>::ObjectExists.into());
					},
					Some(CompressWith::Lzss) if obj_data.obj == compressed_obj => {
						return Err(Error::<T>::ObjectExists.into());
					},
					_ => {},
				}
			}

			let actual_rewords = Self::rewards(None).unwrap();
			let lock_amount = actual_rewords.0 + actual_rewords.1;
			let free = <T as pallet::Config>::Currency::free_balance(&acc);
			if free < lock_amount {
				return Err(Error::<T>::UnsufficientBalance.into());
			}

			let tot_locked= AccountLock::<T>::get(&acc);
			let new_locked = tot_locked.saturating_add(lock_amount);
			Self::set_lock(&acc, new_locked);

			let hashes = match hashes {
				Some(hashes) => hashes,
				None => Self::calc_hashes(&category, None, &obj, DEFAULT_OBJECT_HASHES)?,
			};

			if hashes.len() == 0 {
				return Err(Error::<T>::NoHashes.into());
			}

			log::debug!(target: LOG_TARGET, "put_object::hashes");
			for a in hashes.iter() {
				log::debug!(target: LOG_TARGET, "{:#?}", a);
			}

			if let Some(_) = Self::find_dup(None, &hashes) {
				return Err(Error::<T>::DuplicatedHashes.into());
			}

			let mut properties = properties.clone();
			let share_prop = properties.iter()
				.find(|PropValue {prop_idx, ..}| *prop_idx == 0);
			if share_prop.is_none() {
				properties.try_insert(0, PropValue {prop_idx: 0, max_value: 1} ).map_err(|_| Error::<T>::TooManyProperties)?;
			}

			for prop in properties.iter_mut() {
				match Properties::<T>::get(&prop.prop_idx) {
					Some(ref p) => {
						if prop.max_value > p.max_value {
							return Err(Error::<T>::InvalidPropValue.into())
						}
						if p.class == PropClass::Relative {
							let mut r = prop.max_value.clone();
							while r >= 10 { r /= 10; }
							if r > 1 {
								return Err(Error::<T>::InvalidSharePropValue.into())
							}
						}
					}
					None =>
						return Err(Error::<T>::UnknownProperty.into()),
				}
			}

			let block_num = <frame_system::Pallet<T>>::block_number();
			let state =  ObjectState::Created(block_num);
			let obj_data = ObjData::<T::AccountId, T::BlockNumber> {
				state,
				obj: compressed_obj,
				compressed_with: Some(CompressWith::Lzss),
				category,
				hashes,
				when_created: block_num,
				when_approved: None,
				owner: acc.clone(),
				estimators: BoundedVec::default(),
				est_outliers: BoundedVec::default(),
				approvers: BoundedVec::default(),
				num_approvals,
				est_rewards: actual_rewords.0.saturated_into(),
				author_rewards: actual_rewords.1.saturated_into(),
				prop: properties,
			};
			<Objects<T>>::insert(obj_idx, obj_data);
			<ObjCount<T>>::put(obj_idx + 1);
			let _ = <Owners<T>>::mutate(acc.clone(), |v| v.try_push(obj_idx));

			// TODO:
			Self::deposit_event(Event::ObjCreated(acc));
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
								if let Some(dup_idx) = Self::find_dup(Some(obj_idx), &obj_data.hashes) {
									obj_data.state = ObjectState::NotApproved(current_block);
									Self::deposit_event(Event::ObjNotApproved(
										obj_idx,
										NotApprovedReason::DuplicateFound(obj_idx, dup_idx))
									);
									return;
								}

								obj_data.when_approved = Some(current_block);
								obj_data.state = ObjectState::Approved(current_block);
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

		#[pallet::weight(1_000_000)]
		pub fn set_fee_per_byte(
			origin: OriginFor<T>,
			fee: u64,
		) -> DispatchResultWithPostInfo {
			T::AdminOrigin::ensure_origin(origin)?;

			FeePerByte::<T>::put(fee);
			Ok(().into())
		}

		#[pallet::weight(1_000_000)]
		pub fn add_property_type(
			origin: OriginFor<T>,
			name: BoundedVec<u8, ConstU32<64>>,
			class: PropClass,
			max_value: u128,
		) -> DispatchResultWithPostInfo {
			T::AdminOrigin::ensure_origin(origin)?;

			let prop_idx = <Pallet::<T>>::prop_count();
			Properties::<T>::insert(prop_idx, Property {name, class, max_value} );
			<PropCount<T>>::put(prop_idx + 1);
			Ok(().into())
		}
	}

	#[pallet::inherent]
	impl<T: Config> ProvideInherent for Pallet<T> {
		type Call = Call<T>;
		type Error = super::InherentError;
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

							if let Some(dup_idx) = Self::find_dup(Some(*obj_idx), &hashes) {
								log::error!(target: LOG_TARGET, "check_inherent: for obj_idx={} duplicated hashes found in obj_idx={}", obj_idx, &dup_idx);
								return Err(Self::Error::DuplicatedObjectHashes(*obj_idx, dup_idx))
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
			let rewards_by_est: u128 = est_rewards.saturated_into::<u128>() / ((obj_data.estimators.len() - obj_data.est_outliers.len()) as u128);

			let mut payed_rewards: BalanceOf<T> = BalanceOf::<T>::default();
			log::debug!(target: LOG_TARGET, "estimator rewards: {:?}", rewards_by_est);

			for (acc, _est) in obj_data.estimators.iter() {
				if !obj_data.est_outliers.contains(&acc) {
					let _res = <T as pallet::Config>::Currency::transfer(
						&owner, &acc, rewards_by_est.saturated_into(), ExistenceRequirement::KeepAlive,
					);
					payed_rewards = payed_rewards.saturating_add(rewards_by_est.saturated_into());
				}
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

	fn calc_hashes(
		cat: &ObjectCategory,
		compressed: Option<CompressWith>,
		obj: &Vec<u8>,
		num_hashes: u32,
	)-> Result<BoundedVec<H256, ConstU32<MAX_OBJECT_HASHES>>, Error::<T>> {
		match cat {
			ObjectCategory::Objects3D(Algo3D::Grid2dLow) => {
				let raw_obj = if let Some(compress_mode) = compressed {
					compress_mode.decompress(obj)
				}
				else {
					obj.clone()
				};
				Ok(poscan_algo::hashable_object::calc_obj_hashes_n(
					&POSCAN_ALGO_GRID2D_V3A, &raw_obj, &H256::default(), num_hashes,
				).try_into().unwrap())
			},
			_ => {
				Err(Error::UnsupportedCategory)
			}
		}
	}

	fn find_dup(maybe_obj_idx: Option<ObjIdx>, hashes: &Vec<H256>) -> Option<ObjIdx> {
		for (idx, obj) in Objects::<T>::iter() {
			if let Some(obj_idx) = maybe_obj_idx {
				if idx == obj_idx {
					continue
				}
			}
			if matches!(obj.state, ObjectState::Approved(_)) {
				let min_len = core::cmp::min(hashes.len(), obj.hashes.len());
				if hashes.iter().eq(obj.hashes[0..min_len].iter()) {
					return Some(idx)
				}
			}
		}
		None
	}

	fn check_hashes(obj_data: &ObjData<T::AccountId, T::BlockNumber>, proof_num: u32) -> (bool, Option<H256>) {
		let res_hashes =
			Self::calc_hashes(&obj_data.category, obj_data.compressed_with, &obj_data.obj, proof_num.clone());

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

	fn outliers(
		estimators: &BoundedVec<(T::AccountId, u64), ConstU32<MAX_ESTIMATORS>>
	) -> BoundedVec<T::AccountId, ConstU32<MAX_ESTIMATORS>> {
		let est_zero = estimators.iter()
			.filter_map(
				|a|
					if a.1 == 0 { Some(a) } else { None }
			)
			.collect::<Vec<_>>();

		let mut sorted: Vec<(T::AccountId, u64)> = estimators.clone().into();
		sorted.sort_by_key(|a| a.1);

		sorted.retain(|a| !est_zero.iter().any(|b| b.0 == a.0));
		let n = sorted.len();
		if n < 4 {
			return est_zero.iter()
				.map(|a| a.0.clone())
				.collect::<Vec<_>>()
				.try_into()
				.unwrap();
		}

		let q1 = (&n - 1) / 4;
		let q3 = (3 * (&n - 1)) / 4;
		let d = ((&n - 1) % 4) as u64;

		let val1 = (&sorted[q1].1 * (4 - &d) + &sorted[q1 + 1].1 * &d) as f64 / 4.0;
		let val2 = (&sorted[q3].1 * (4 - &d) + &sorted[q3 + 1].1 * &d) as f64 / 4.0;
		let iqr = val2 - val1;
		let r = 1.5f64 * iqr as f64;
		let rng = (val1 - r)..=(val2 + r);

		sorted.iter()
			.filter_map(
				|a|
					if rng.contains(&(a.1 as f64)) { None } else { Some(a.0.clone()) }
			)
			.chain(
				est_zero.iter().map(|a| a.0.clone()).into_iter()
			)
			.collect::<Vec<_>>()
			.try_into()
			.unwrap()
	}
}


impl<T: Config> PoscanApi<T::AccountId, T::BlockNumber> for Pallet<T> {
	fn get_poscan_object(i: u32) -> Option<sp_consensus_poscan::ObjData<T::AccountId, T::BlockNumber>> {
		match Objects::<T>::get(i) {
			Some(mut obj_data) => {
				if let Some(compressor) = obj_data.compressed_with {
					let resp_obj = obj_data.clone();
					obj_data.obj = compressor.decompress(&resp_obj.obj).try_into().unwrap();
					obj_data.compressed_with = None;
					Some(obj_data)
				}
				else {
					Some(obj_data)
				}
			},
			None => None,
		}
	}
	fn is_owner_of(account_id: &T::AccountId, obj_idx: u32) -> bool {
		let own_objects = Owners::<T>::get(&account_id);
		own_objects.contains(&obj_idx)
	}
	fn property(obj_idx: u32, prop_idx: u32) -> Option<PropValue> {
		Objects::<T>::get(&obj_idx)
			.map(
					|obj_data|
						obj_data.prop.iter()
							.find(|p| p.prop_idx == prop_idx)
							.cloned()
			)
			.flatten()
	}
}
