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
		ReservableCurrency,
		Currency,
		ExistenceRequirement,
	},
	pallet_prelude::*,
};

// Import the same permission structure as used in rewards pallet
use frame_support::traits::EitherOfDiverse;
use frame_system::EnsureRoot;
use pallet_collective::EnsureProportionAtLeast;
use frame_support::traits::BalanceStatus;

use sp_runtime::SaturatedConversion;
pub use sp_runtime::Percent;
use sp_runtime::traits::{Saturating, Zero, Hash};
use num_traits::float::Float;
use serial_numbers_api::SerialNumbersApi;

// use sp_core::crypto::AccountId32;
use sp_core::H256;
use sp_core::offchain::Duration;
use sp_inherents::IsFatalError;
use sp_runtime::traits::BlakeTwo256;

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
	ObjIdx,
	POSCAN_ALGO_GRID2D_V3A,
	MAX_OBJECT_SIZE,
	MAX_PROPERTIES,
	DEFAULT_OBJECT_HASHES,
	MAX_OBJECT_HASHES,
	DEFAULT_MAX_ALGO_TIME,
	MAX_ESTIMATORS,
	FEE_PER_BYTE,
	PropIdx,
	Property,
	PropClass,
	PropValue,
	CopyPermission,
	DOLLARS,
};

use sp_runtime::traits::StaticLookup;

// Define the same permission structure as used in rewards pallet
// Note: This type is defined for potential future use but currently unused
#[allow(dead_code)]
type EnsureRootOrHalfCouncil<T> = EitherOfDiverse<
	EnsureRoot<<T as frame_system::Config>::AccountId>,
	EnsureProportionAtLeast<<T as frame_system::Config>::AccountId, pallet_collective::Instance1, 1, 2>,
>;

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

// Add this constant near the top of the file, after the existing constants
// Note: We use 2/3 consensus threshold from the old version instead of fixed minimum

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

		#[pallet::constant]
		type DynamicRewardsGrowthRate: Get<u32>;

		type Currency: LockableCurrency<Self::AccountId> + ReservableCurrency<Self::AccountId>;

		type ValidatorSet: ValidatorSetApi<Self::AccountId, Self::BlockNumber, BalanceOf::<Self>>;

		// #[pallet::constant]MAX_OBJECT_SIZE Get<u32>;
        /// Serial numbers API provider
        type SerialNumbers: serial_numbers_api::SerialNumbersApi<Self::AccountId>;
        		/// poscanAssets API provider
        type PoscanAssets: poscan_api::PoscanApi<Self::AccountId, Self::BlockNumber>;
        
        /// Origin for council control functions
        type CouncilOrigin: EnsureOrigin<Self::Origin>;
	}

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(4);

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn owners)]
	pub type Owners<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, BoundedVec<ObjIdx, ConstU32<100>>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn objects_of_inspector)]
	pub type ObjectsOfInspector<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, BoundedVec<ObjIdx, ConstU32<100>>, ValueQuery>;

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

	#[pallet::storage]
	#[pallet::getter(fn dynamic_rewards_growth_rate)]
	pub type DynamicRewardsGrowthRate<T> = StorageValue<_, u32, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn pending_storage_fees)]
	pub type PendingStorageFees<T> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn private_object_permissions)]
	pub type PrivateObjectPermissions<T: Config> = StorageMap<_, Twox64Concat, ObjIdx, BoundedVec<CopyPermission<T::AccountId, T::BlockNumber>, ConstU32<32>>, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event emitted when an object has been created.
		ObjCreated(T::AccountId),
		/// Event emitted when an object has been approved.
		ObjApproved(ObjIdx),
		/// Event emitted when an object has not been approved (expired).
		ObjNotApproved(ObjIdx, NotApprovedReason),
        /// Event emitted when an object enters QC inspection
        QCInspecting(ObjIdx, T::AccountId),
        /// Event emitted when an object passes QC
        QCPassed(ObjIdx, T::AccountId),
        /// Event emitted when an object is rejected by QC
        QCRejected(ObjIdx, T::AccountId),
        /// Event emitted when object ownership is transferred
        ObjectOwnershipTransferred(ObjIdx, T::AccountId, T::AccountId),
        /// Event emitted when object ownership is abdicated
        ObjectOwnershipAbdicated(ObjIdx, T::AccountId),
        /// Event emitted when verification fee is charged from fee payer
        VerificationFeeCharged(ObjIdx, T::AccountId, BalanceOf<T>),
        /// Event emitted when dynamic rewards growth rate is set
        DynamicRewardsGrowthRateSet(u32),
        /// Event emitted when inspector fee is paid
        InspectorFeePaid(ObjIdx, T::AccountId, BalanceOf<T>),
        /// Event emitted when object is estimated
        ObjEstimated(ObjIdx),
        /// Event emitted when unspent rewards are unlocked
        UnspentRewardsUnlocked(ObjIdx, T::AccountId, BalanceOf<T>),
        /// Event emitted when storage fees are distributed to validators
        StorageFeesDistributed(BalanceOf<T>),
        /// Event emitted when author part is set
        AuthorPartSet(Percent),
        /// Event emitted when rewards are set
        RewardsSet(BalanceOf<T>),
        /// Event emitted when QC inspection times out and inspector fee is unreserved
        QCInspectionTimeout(ObjIdx, T::AccountId, BalanceOf<T>),
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
		/// Not permitted to create replica
		NotPermitted,
		/// License period expired
		LicenseExpired,
		/// Replica must match at least one hash
		NotAReplica,
		/// Original object not found
		OriginalNotFound,
		/// Too many permissions
		TooManyPermissions,
		/// Cannot create replica of not approved object
		OriginalNotApproved,
        /// Serial number does not exist
        SerialNumberNotFound,
        /// Serial number already used
        SerialNumberAlreadyUsed,
        /// Serial number expired
        SerialNumberExpired,
        /// Not the owner of the serial number
        SerialNumberNotOwner,
        /// Only inspector can perform this action
        NotInspector,
        /// Object not in QCInspecting state
        NotQCInspecting,
        /// Object has associated asset and cannot be transferred
        ObjectHasAsset,
        /// Object ownership has been abdicated and cannot be transferred
        OwnershipAbdicated,
        /// Fee payer has insufficient balance to pay verification fee
        FeePayerInsufficientBalance,
        /// Object is not in NotApproved state
        ObjectNotNotApproved,
        /// No unspent rewards to unlock
        NoUnspentRewards,
        /// Caller is not the fee payer
        NotFeePayer,
        /// QC timeout is too short (minimum 5 blocks)
        QCTimeoutTooShort,
        /// QC timeout is too long (maximum 100,000 blocks)
        QCTimeoutTooLong,
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
			poscan_algo::hashable_object::try_call_130();

			for obj_idx in Objects::<T>::iter_keys() {
				Objects::<T>::mutate(obj_idx, |obj_data| {
					match obj_data {
						Some(ref mut obj_data) => {
							match obj_data.state {
								ObjectState::Created(_when) => {
									obj_data.state = ObjectState::Estimating(now);
									log::debug!(target: LOG_TARGET, "on_initialize: Created -> Estimating for obj_idx={}", &obj_idx);
								},
								ObjectState::Estimating(when) => {
									// Calculate outliers first
										obj_data.est_outliers = Self::outliers(&obj_data.estimators);
									let n_est = obj_data.estimators.len() - obj_data.est_outliers.len();
									let n_val = T::ValidatorSet::validators().len();
									
									// Check if consensus is reached (2/3 of total validators as per old version)
									if n_est * 2 >= n_val {
										// Use the old version's average calculation (as intended)
										let t = obj_data.estimators
											.iter()
											.filter(|a| !obj_data.est_outliers.contains(&a.0))
											.fold(0u64, |acc, (_, time)| acc + time);
										
										let average_time = if n_est > 0 {
											t / (n_est as u64)
										} else {
											0u64
										};
										
										obj_data.state = ObjectState::Estimated(now, average_time);
										
										// Distribute estimator rewards
										Self::reward_estimators(obj_idx);
										
										Self::deposit_event(Event::ObjEstimated(obj_idx));
										log::debug!(target: LOG_TARGET, "on_initialize: Estimating -> Estimated for obj_idx={} with {} valid estimators (consensus reached)", &obj_idx, n_est);
									} else if now.saturating_sub(when) >= T::EstimatePeriod::get().into() {
										// Check if estimation period has expired without reaching consensus
										obj_data.state = ObjectState::NotApproved(now);
										
										// Even if expired, distribute rewards to estimators who participated
										if obj_data.estimators.len() > 0 {
											Self::reward_estimators(obj_idx);
										}
										
										Self::deposit_event(Event::ObjNotApproved(obj_idx, NotApprovedReason::Expired));
										log::debug!(target: LOG_TARGET, "on_initialize: Estimating -> NotApproved (expired) for obj_idx={} with {} valid estimators (consensus not reached)", &obj_idx, n_est);
									}
									// Otherwise, keep it in Estimating state
								},
								ObjectState::Estimated(when, _) => {
									// Check if approval timeout has expired
									if now.saturating_sub(when) >= T::ApproveTimeout::get().into() {
										obj_data.state = ObjectState::NotApproved(now);
										obj_data.est_outliers = Self::outliers(&obj_data.estimators);
										Self::deposit_event(Event::ObjNotApproved(obj_idx, NotApprovedReason::Expired));
										log::debug!(target: LOG_TARGET, "on_initialize: Estimated -> NotApproved (expired) for obj_idx={}", &obj_idx);
									}
									// Otherwise, keep it in Estimated state
								},
								ObjectState::QCInspecting(_, when) => {
									// Check if QC inspection timeout has expired
									// Use the custom QC timeout for this object
									let qc_timeout = obj_data.qc_timeout;
									if now.saturating_sub(when) >= qc_timeout.into() {
										// Unreserve inspector fee back to the fee payer
										let inspector_fee = obj_data.inspector_fee.saturated_into();
										let _ = T::Currency::unreserve(&obj_data.fee_payer, inspector_fee);
										
										obj_data.state = ObjectState::NotApproved(now);
										Self::deposit_event(Event::ObjNotApproved(obj_idx, NotApprovedReason::Expired));
										Self::deposit_event(Event::QCInspectionTimeout(obj_idx, obj_data.fee_payer.clone(), inspector_fee));
										log::debug!(target: LOG_TARGET, "on_initialize: QCInspecting -> NotApproved (timeout) for obj_idx={} with custom timeout={}, unreserved {:?} fee", &obj_idx, qc_timeout, inspector_fee);
									}
									// Otherwise, keep it in QCInspecting state
								},
								ObjectState::QCPassed(_, _when) => {
									// Transition QCPassed to Created to start the normal estimation process
									obj_data.state = ObjectState::Created(now);
									log::debug!(target: LOG_TARGET, "on_initialize: QCPassed -> Created for obj_idx={}", &obj_idx);
								},
								ObjectState::SelfProved(_when) => {
									// Keep in SelfProved state - no automatic transition
									// Self-proved objects need manual verification via verify_self_proved_object
								},
								ObjectState::QCRejected(_, _when) => {
									obj_data.state = ObjectState::NotApproved(now);
									obj_data.est_outliers = Self::outliers(&obj_data.estimators);
									Self::deposit_event(Event::ObjNotApproved(obj_idx, NotApprovedReason::Expired));
									log::debug!(target: LOG_TARGET, "on_initialize: QCRejected -> NotApproved for obj_idx={}", &obj_idx);
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
			
			// Distribute pending storage fees to validators
			Self::distribute_storage_fees_to_validators();
			
			// TODO:
			0
		}

		fn on_runtime_upgrade() -> frame_support::weights::Weight {
			let onchain_version = Pallet::<T>::on_chain_storage_version();
			log::info!(target: LOG_TARGET, "on_runtime_upgrade: onchain_version={:?}", &onchain_version);
			
			let mut weight = Weight::zero();
			
			// Handle migration from old mainnet version (v2) to new version (v4)
			if onchain_version < 4 {
				log::info!(target: LOG_TARGET, "on_runtime_upgrade: starting migration from v{:?} to v4", onchain_version);
				
				// First, handle the old v2 migration if needed
			if onchain_version < 3 {
					log::info!(target: LOG_TARGET, "on_runtime_upgrade: applying v3 migration first");
				
				// Migrate all objects to add replica, self-proved, fee_payer, inspector_fee, and qc_timeout fields
				let _migrated_count = Objects::<T>::translate::<ObjData<T::AccountId, T::BlockNumber>, _>(|_key, mut obj| {
					// Add replica fields
					obj.is_replica = false;
					obj.original_obj = None;
                    obj.sn_hash = None;
                    obj.inspector = None;
                    obj.is_ownership_abdicated = false;
					
					// Add self-proved fields
					obj.is_self_proved = false;
					obj.proof_of_existence = None;
                    obj.ipfs_link = None;
					
					// Add fee_payer field (initially same as owner)
					obj.fee_payer = obj.owner.clone();
					
					// Add inspector_fee field (initially zero)
					obj.inspector_fee = 0u128;
					
					// Add qc_timeout field (use default timeout)
					obj.qc_timeout = T::ApproveTimeout::get();
					
					Some(obj)
				});
				
					log::info!(target: LOG_TARGET, "on_runtime_upgrade: successfully migrated objects to v3");
					weight = weight.saturating_add(1000u64);
				}
				
				// Now apply v4 migration - add new storage fields and update reward system
				log::info!(target: LOG_TARGET, "on_runtime_upgrade: applying v4 migration");
				
				// Initialize new storage fields with default values
				// EstimatorsShare defaults to 70% (from old AuthorPart logic)
				let author_part = AuthorPart::<T>::get().unwrap_or(T::AuthorPartDefault::get());
				AuthorPart::<T>::put(author_part);
				log::info!(target: LOG_TARGET, "on_runtime_upgrade: initialized AuthorPart to {:?}", author_part);
				
				// DynamicRewardsGrowthRate defaults to 100
				if DynamicRewardsGrowthRate::<T>::get().is_none() {
					DynamicRewardsGrowthRate::<T>::put(100u32);
					log::info!(target: LOG_TARGET, "on_runtime_upgrade: initialized DynamicRewardsGrowthRate to 100");
				}
				
				// PendingStorageFees starts at zero
				PendingStorageFees::<T>::put(BalanceOf::<T>::zero());
				log::info!(target: LOG_TARGET, "on_runtime_upgrade: initialized PendingStorageFees to zero");
				
				// Update storage version to v4
				StorageVersion::new(4).put::<Pallet::<T>>();
				
				log::info!(target: LOG_TARGET, "on_runtime_upgrade: successfully migrated to v4 with new storage fields and reward system");
				
				// Add weight for migration operations
				weight = weight.saturating_add(2000u64);
			}
			
			weight
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
			is_private: bool,
			obj: BoundedVec<u8, ConstU32<MAX_OBJECT_SIZE>>,
			num_approvals: u8,
			hashes: Option<BoundedVec<H256, ConstU32<MAX_OBJECT_HASHES>>>,
			properties: BoundedVec<PropValue, ConstU32<MAX_PROPERTIES>>,
			is_replica: bool,
			original_obj: Option<ObjIdx>,
            sn_hash: Option<u64>,
            is_self_proved: bool,
            proof_of_existence: Option<H256>,
            ipfs_link: Option<BoundedVec<u8, ConstU32<512>>>,
		) -> DispatchResultWithPostInfo {
			let acc = ensure_signed(origin)?;
			let obj_idx = <Pallet::<T>>::obj_count();

			if num_approvals == 0 {
				return Err(Error::<T>::ZeroApprovalsRequested.into());
			}

			let compress_with = CompressWith::Lzss;
			let compressed_obj: BoundedVec<u8, ConstU32<MAX_OBJECT_SIZE>> =
				compress_with.compress(obj.to_vec()).try_into().unwrap();

			for (_idx, obj_data)
				in Objects::<T>::iter().filter(
					|obj| !matches!(obj.1.state, ObjectState::<T::BlockNumber, T::AccountId>::NotApproved(_))
			) {
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

			if is_replica {
				let orig_idx = original_obj.ok_or(Error::<T>::OriginalNotFound)?;
				let orig = Objects::<T>::get(orig_idx).ok_or(Error::<T>::OriginalNotFound)?;
				// Only allow if original is Approved
				match orig.state {
					ObjectState::<T::BlockNumber, T::AccountId>::Approved(_) => {},
					_ => return Err(Error::<T>::OriginalNotApproved.into()),
				}
				// At least one hash must match
				let match_found = hashes.iter().any(|h| orig.hashes.contains(h));
				if !match_found {
					return Err(Error::<T>::NotAReplica.into());
				}
				// Permission logic for private originals
				if orig.is_private {
					let perms = PrivateObjectPermissions::<T>::get(orig_idx);
					let now = <frame_system::Pallet<T>>::block_number();
					let mut allowed = false;
					for perm in perms.iter() {
						if perm.who == acc && perm.until >= now && perm.max_copies > 0 {
							allowed = true;
							break;
						}
					}
					if !allowed {
						return Err(Error::<T>::NotPermitted.into());
					}
				}
                // Serial number checks for replicas
                if let Some(sn_idx) = sn_hash {
                    // a. Serial number exists
                    let _sn_vec = <T as frame_system::Config>::BlockNumber::from(0u32); // dummy block number for API
                    let sn_details = T::SerialNumbers::get_serial_numbers(Some(sn_idx));
                    let sn_details = sn_details.get(0).ok_or(Error::<T>::SerialNumberNotFound)?;
                    // b. Serial number is not used
                    if T::SerialNumbers::is_serial_number_used(sn_details.sn_hash) {
                        return Err(Error::<T>::SerialNumberAlreadyUsed.into());
                    }
                    // c. Serial number is not expired
                    if sn_details.is_expired {
                        return Err(Error::<T>::SerialNumberExpired.into());
                    }
                    // d. Serial number owner matches object owner
                    if sn_details.owner != acc {
                        return Err(Error::<T>::SerialNumberNotOwner.into());
                    }
                }
				// Do NOT check for duplicate hashes (replica is allowed)
			} else {
				// Normal: reject if duplicate
				if let Some(_) = Self::find_dup(None, &hashes) {
					return Err(Error::<T>::DuplicatedHashes.into());
				}
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

			// Validate self-proved object parameters
			if is_self_proved {
				// Self-proved objects must be replicas
				if !is_replica {
					return Err(Error::<T>::NotAReplica.into());
				}
				// Must have proof of existence
				if proof_of_existence.is_none() {
					return Err(Error::<T>::NoHashes.into());
				}
				// Must have IPFS link
				if ipfs_link.is_none() {
					return Err(Error::<T>::NoHashes.into());
				}
			}

			let block_num = <frame_system::Pallet<T>>::block_number();
			let state = if is_self_proved {
				ObjectState::<T::BlockNumber, T::AccountId>::SelfProved(block_num)
			} else {
				ObjectState::<T::BlockNumber, T::AccountId>::Created(block_num)
			};
			
			let obj_to_store = if is_self_proved {
				// For self-proved objects, don't store the actual object
				BoundedVec::default()
			} else {
				compressed_obj
			};
			
			let obj_data = ObjData::<T::AccountId, T::BlockNumber> {
				state,
				obj: obj_to_store,
				compressed_with: if is_self_proved { None } else { Some(CompressWith::Lzss) },
				category,
				is_private,
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
				is_replica,
				original_obj,
                sn_hash,
                inspector: None,
                is_ownership_abdicated: false,
                is_self_proved,
                proof_of_existence,
                ipfs_link,
                fee_payer: acc.clone(), // Initial submitter is the fee payer
                				inspector_fee: 0u128, // No inspector fee for initial submission
                qc_timeout: T::ApproveTimeout::get(), // Use default timeout for non-QC objects
			};
			<Objects<T>>::insert(obj_idx, obj_data);
			<ObjCount<T>>::put(obj_idx + 1);
			let _ = <Owners<T>>::mutate(acc.clone(), |v| v.try_push(obj_idx));

			// TODO:
			Self::deposit_event(Event::ObjCreated(acc));
			Ok(().into())
		}

		/// Inspect and put file to poscan storage (QC flow)
		#[pallet::weight(<Pallet::<T>>::fee_per_byte().unwrap_or(FEE_PER_BYTE) * obj.len() as u64)]
		pub fn inspect_put_object(
			origin: OriginFor<T>,
			category: ObjectCategory,
			is_private: bool,
			obj: BoundedVec<u8, ConstU32<MAX_OBJECT_SIZE>>,
			num_approvals: u8,
			hashes: Option<BoundedVec<H256, ConstU32<MAX_OBJECT_HASHES>>>,
			properties: BoundedVec<PropValue, ConstU32<MAX_PROPERTIES>>,
			is_replica: bool,
			original_obj: Option<ObjIdx>,
            sn_hash: Option<u64>,
            inspector: <T::Lookup as StaticLookup>::Source,
            inspector_fee: BalanceOf<T>,
            qc_timeout: u32,
            is_self_proved: bool,
            proof_of_existence: Option<H256>,
            ipfs_link: Option<BoundedVec<u8, ConstU32<512>>>,
		) -> DispatchResultWithPostInfo {
			let acc = ensure_signed(origin)?;
            let inspector = T::Lookup::lookup(inspector)?;
            let inspector_clone = inspector.clone();
			let obj_idx = <Pallet::<T>>::obj_count();

			if num_approvals == 0 {
				return Err(Error::<T>::ZeroApprovalsRequested.into());
			}

			// Validate QC timeout bounds (5-100,000 blocks)
			ensure!(qc_timeout >= 5, Error::<T>::QCTimeoutTooShort);
			ensure!(qc_timeout <= 100_000, Error::<T>::QCTimeoutTooLong);

			let compress_with = CompressWith::Lzss;
			let compressed_obj: BoundedVec<u8, ConstU32<MAX_OBJECT_SIZE>> =
				compress_with.compress(obj.to_vec()).try_into().unwrap();

			for (_idx, obj_data)
				in Objects::<T>::iter().filter(
					|obj| !matches!(obj.1.state, ObjectState::<T::BlockNumber, T::AccountId>::NotApproved(_))
			) {
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
			let total_required = lock_amount + inspector_fee;
			let free = <T as pallet::Config>::Currency::free_balance(&acc);
			if free < total_required {
				return Err(Error::<T>::UnsufficientBalance.into());
			}

			let tot_locked= AccountLock::<T>::get(&acc);
			let new_locked = tot_locked.saturating_add(lock_amount);
			Self::set_lock(&acc, new_locked);

			// Reserve inspector fee
			T::Currency::reserve(&acc, inspector_fee)?;

			let hashes = match hashes {
				Some(hashes) => hashes,
				None => Self::calc_hashes(&category, None, &obj, DEFAULT_OBJECT_HASHES)?,
			};

			if hashes.len() == 0 {
				return Err(Error::<T>::NoHashes.into());
			}

			log::debug!(target: LOG_TARGET, "inspect_put_object::hashes");
			for a in hashes.iter() {
				log::debug!(target: LOG_TARGET, "{:#?}", a);
			}

			if is_replica {
				let orig_idx = original_obj.ok_or(Error::<T>::OriginalNotFound)?;
				let orig = Objects::<T>::get(orig_idx).ok_or(Error::<T>::OriginalNotFound)?;
				// Only allow if original is Approved
				match orig.state {
					ObjectState::<T::BlockNumber, T::AccountId>::Approved(_) => {},
					_ => return Err(Error::<T>::OriginalNotApproved.into()),
				}
				// At least one hash must match
				let match_found = hashes.iter().any(|h| orig.hashes.contains(h));
				if !match_found {
					return Err(Error::<T>::NotAReplica.into());
				}
				// Permission logic for private originals
				if orig.is_private {
					let perms = PrivateObjectPermissions::<T>::get(orig_idx);
					let now = <frame_system::Pallet<T>>::block_number();
					let mut allowed = false;
					for perm in perms.iter() {
						if perm.who == acc && perm.until >= now && perm.max_copies > 0 {
							allowed = true;
							break;
						}
					}
					if !allowed {
						return Err(Error::<T>::NotPermitted.into());
					}
				}
                // Serial number checks for replicas
                if let Some(sn_idx) = sn_hash {
                    let sn_details = T::SerialNumbers::get_serial_numbers(Some(sn_idx));
                    let sn_details = sn_details.get(0).ok_or(Error::<T>::SerialNumberNotFound)?;
                    if T::SerialNumbers::is_serial_number_used(sn_details.sn_hash) {
                        return Err(Error::<T>::SerialNumberAlreadyUsed.into());
                    }
                    if sn_details.is_expired {
                        return Err(Error::<T>::SerialNumberExpired.into());
                    }
                    if sn_details.owner != acc {
                        return Err(Error::<T>::SerialNumberNotOwner.into());
                    }
                }
				// Do NOT check for duplicate hashes (replica is allowed)
			} else {
				// Normal: reject if duplicate
				if let Some(_) = Self::find_dup(None, &hashes) {
					return Err(Error::<T>::DuplicatedHashes.into());
				}
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

			// Validate self-proved object parameters
			if is_self_proved {
				// Self-proved objects must be replicas
				if !is_replica {
					return Err(Error::<T>::NotAReplica.into());
				}
				// Must have proof of existence
				if proof_of_existence.is_none() {
					return Err(Error::<T>::NoHashes.into());
				}
				// Must have IPFS link
				if ipfs_link.is_none() {
					return Err(Error::<T>::NoHashes.into());
				}
			}

			let block_num = <frame_system::Pallet<T>>::block_number();
            let state = ObjectState::<T::BlockNumber, T::AccountId>::QCInspecting(inspector.clone(), block_num);
			
			let obj_to_store = if is_self_proved {
				// For self-proved objects, don't store the actual object
				BoundedVec::default()
			} else {
				compressed_obj
			};
			
			let obj_data = ObjData::<T::AccountId, T::BlockNumber> {
				state,
				obj: obj_to_store,
				compressed_with: if is_self_proved { None } else { Some(CompressWith::Lzss) },
				category,
				is_private,
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
				is_replica,
				original_obj,
                sn_hash,
                inspector: Some(inspector),
                is_ownership_abdicated: false,
                is_self_proved,
                proof_of_existence,
                ipfs_link,
                fee_payer: acc.clone(), // Initial submitter is the fee payer
                				inspector_fee: inspector_fee.saturated_into(), // Inspector fee reserved for QC inspection
                qc_timeout, // Custom QC timeout for this object
			};
			<Objects<T>>::insert(obj_idx, obj_data);
			<ObjCount<T>>::put(obj_idx + 1);
			let _ = <Owners<T>>::mutate(acc.clone(), |v| v.try_push(obj_idx));
			let _ = <ObjectsOfInspector<T>>::mutate(inspector_clone.clone(), |v| v.try_push(obj_idx));

			Self::deposit_event(Event::ObjCreated(acc));
            Self::deposit_event(Event::QCInspecting(obj_idx, inspector_clone));
			Ok(().into())
		}

		/// Inspector approves the object (QC passed)
		#[pallet::weight(1_000_000)]
		pub fn qc_approve(
			origin: OriginFor<T>,
			obj_idx: ObjIdx,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			let mut obj_data = Objects::<T>::get(obj_idx).ok_or(Error::<T>::ObjectNotFound)?;
			
			// Verify the inspector and transfer fee
			match &obj_data.state {
				ObjectState::<T::BlockNumber, T::AccountId>::QCInspecting(ref inspector, _) if *inspector == who => {
					// Transfer inspector fee to the inspector
					let _ = T::Currency::repatriate_reserved(
						&obj_data.owner,
						&who,
						obj_data.inspector_fee.saturated_into(),
						BalanceStatus::Free,
					);
					
					let block_num = <frame_system::Pallet<T>>::block_number();
					let inspector_fee = obj_data.inspector_fee.saturated_into();
					obj_data.state = ObjectState::<T::BlockNumber, T::AccountId>::QCPassed(who.clone(), block_num);
					Objects::<T>::insert(obj_idx, obj_data);
					
                    Self::deposit_event(Event::QCPassed(obj_idx, who.clone()));
                    Self::deposit_event(Event::InspectorFeePaid(obj_idx, who, inspector_fee));
					Ok(().into())
				},
				ObjectState::<T::BlockNumber, T::AccountId>::QCInspecting(_, _) => Err(Error::<T>::NotInspector.into()),
				_ => Err(Error::<T>::NotQCInspecting.into()),
			}
		}

		/// Inspector rejects the object (QC rejected)
		#[pallet::weight(1_000_000)]
		pub fn qc_reject(
			origin: OriginFor<T>,
			obj_idx: ObjIdx,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			let mut obj_data = Objects::<T>::get(obj_idx).ok_or(Error::<T>::ObjectNotFound)?;
			
			// Verify the inspector and transfer fee
			match &obj_data.state {
				ObjectState::<T::BlockNumber, T::AccountId>::QCInspecting(ref inspector, _) if *inspector == who => {
					// Transfer inspector fee to the inspector (even for rejection)
					let _ = T::Currency::repatriate_reserved(
						&obj_data.owner,
						&who,
						obj_data.inspector_fee.saturated_into(),
						BalanceStatus::Free,
					);
					
					let block_num = <frame_system::Pallet<T>>::block_number();
					let inspector_fee = obj_data.inspector_fee.saturated_into();
					obj_data.state = ObjectState::<T::BlockNumber, T::AccountId>::QCRejected(who.clone(), block_num);
					Objects::<T>::insert(obj_idx, obj_data);
					
                    Self::deposit_event(Event::QCRejected(obj_idx, who.clone()));
                    Self::deposit_event(Event::InspectorFeePaid(obj_idx, who, inspector_fee));
					Ok(().into())
				},
				ObjectState::<T::BlockNumber, T::AccountId>::QCInspecting(_, _) => Err(Error::<T>::NotInspector.into()),
				_ => Err(Error::<T>::NotQCInspecting.into()),
			}
		}

		/// Set permissions for private object replicas
		#[pallet::weight(1_000_000)]
		pub fn set_private_object_permissions(
			origin: OriginFor<T>,
			obj_idx: ObjIdx,
			permissions: BoundedVec<CopyPermission<T::AccountId, T::BlockNumber>, ConstU32<32>>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			let obj = Objects::<T>::get(obj_idx).ok_or(Error::<T>::ObjectNotFound)?;
			ensure!(obj.owner == who, Error::<T>::NotOwner);
			use sp_runtime::DispatchError;
			PrivateObjectPermissions::<T>::try_mutate(obj_idx, |perms| -> Result<(), DispatchError> {
				*perms = permissions;
				Ok(())
			})?;
			Ok(().into())
		}

		#[pallet::weight(1_000_000)]
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
						if let ObjectState::<T::BlockNumber, T::AccountId>::Estimated(..) = obj_data.state {
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
									obj_data.state = ObjectState::<T::BlockNumber, T::AccountId>::NotApproved(current_block);
									Self::deposit_event(Event::ObjNotApproved(
										obj_idx,
										NotApprovedReason::DuplicateFound(obj_idx, dup_idx))
									);
									return;
								}

								obj_data.when_approved = Some(current_block);
								obj_data.state = ObjectState::<T::BlockNumber, T::AccountId>::Approved(current_block);
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

        /// Transfer object ownership to another account if allowed
        #[pallet::weight(1_000_000)]
        pub fn transfer_object_ownership(
            origin: OriginFor<T>,
            obj_idx: ObjIdx,
            new_owner: <T::Lookup as StaticLookup>::Source,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            let new_owner = T::Lookup::lookup(new_owner)?;
            Objects::<T>::try_mutate(obj_idx, |maybe_obj| {
                let obj = maybe_obj.as_mut().ok_or(Error::<T>::ObjectNotFound)?;
                ensure!(obj.owner == who, Error::<T>::NotOwner);
                ensure!(!obj.is_ownership_abdicated, Error::<T>::OwnershipAbdicated);
                // Asset association check (requires trait bound and runtime API)
                if T::PoscanAssets::object_has_asset(obj_idx) {
                    return Err::<(), DispatchError>(Error::<T>::ObjectHasAsset.into());
                }
                obj.owner = new_owner.clone();
                Ok::<(), DispatchError>(())
            })?;
            Owners::<T>::mutate(&who, |v| v.retain(|&idx| idx != obj_idx));
            Owners::<T>::mutate(&new_owner, |v| { let _ = v.try_push(obj_idx); });
            Self::deposit_event(Event::ObjectOwnershipTransferred(obj_idx, who, new_owner));
            Ok(().into())
        }

        /// Abdicate object ownership (irreversible)
        #[pallet::weight(1_000_000)]
        pub fn abdicate_the_obj_ownership(
            origin: OriginFor<T>,
            obj_idx: ObjIdx,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            let mut obj_data = Objects::<T>::get(obj_idx).ok_or(Error::<T>::ObjectNotFound)?;
            ensure!(obj_data.owner == who, Error::<T>::NotOwner);
            ensure!(!obj_data.is_ownership_abdicated, Error::<T>::OwnershipAbdicated);
            ensure!(!T::PoscanAssets::object_has_asset(obj_idx), Error::<T>::ObjectHasAsset);
            obj_data.is_ownership_abdicated = true;
            Objects::<T>::insert(obj_idx, obj_data);
            let _obj = Objects::<T>::get(obj_idx).ok_or(Error::<T>::ObjectNotFound)?;
            Self::deposit_event(Event::ObjectOwnershipAbdicated(obj_idx, who));
            Ok(().into())
        }

        /// Set dynamic rewards growth rate (Council only)
        #[pallet::weight(10_000)]
        pub fn set_dynamic_rewards_growth_rate(
            origin: OriginFor<T>,
            growth_rate: u32,
        ) -> DispatchResultWithPostInfo {
            T::CouncilOrigin::ensure_origin(origin)?;
            
            // Validate the growth rate (should be between 1 and 100)
            ensure!(growth_rate >= 1 && growth_rate <= 100, Error::<T>::InvalidSharePropValue);
            
            // Store the new growth rate
            DynamicRewardsGrowthRate::<T>::put(growth_rate);
            
            Self::deposit_event(Event::DynamicRewardsGrowthRateSet(growth_rate));
            Ok(().into())
        }

        /// Set rewards (Council only)
        #[pallet::weight(10_000)]
        pub fn set_rewards(
            origin: OriginFor<T>,
            rewards: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            T::CouncilOrigin::ensure_origin(origin)?;
            // Validate the rewards (should be at least 1 DOLLAR)
            ensure!(rewards >= (1 * DOLLARS).saturated_into(), Error::<T>::InvalidSharePropValue);
            // Store the new rewards
            Rewards::<T>::put(rewards);
            Self::deposit_event(Event::RewardsSet(rewards));
            Ok(().into())
        }

        /// Verify a self-proved object by providing the actual object data
        #[pallet::weight(<Pallet::<T>>::fee_per_byte().unwrap_or(FEE_PER_BYTE) * obj.len() as u64)]
        pub fn verify_self_proved_object(
            origin: OriginFor<T>,
            obj: BoundedVec<u8, ConstU32<MAX_OBJECT_SIZE>>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            
            // Calculate proof of existence hash from the provided object
            let proof_of_existence = BlakeTwo256::hash(&obj).into();
            
            // Find the self-proved object by proof of existence hash
            let mut found_obj_idx = None;
            for (obj_idx, obj_data) in Objects::<T>::iter() {
                if let Some(ref proof) = obj_data.proof_of_existence {
                    if *proof == proof_of_existence {
                        found_obj_idx = Some(obj_idx);
                        break;
                    }
                }
            }
            
            let obj_idx = found_obj_idx.ok_or(Error::<T>::ObjectNotFound)?;
            let mut obj_data = Objects::<T>::get(obj_idx).ok_or(Error::<T>::ObjectNotFound)?;
            
            // Verify it's a self-proved object
            ensure!(obj_data.is_self_proved, Error::<T>::ObjectNotFound);
            ensure!(matches!(obj_data.state, ObjectState::<T::BlockNumber, T::AccountId>::SelfProved(_)), Error::<T>::ObjectNotFound);
            
            // Verify the current owner is the one submitting
            ensure!(obj_data.owner == who, Error::<T>::NotOwner);
            
            // Charge verification fee from the fee payer (not the transaction sender)
            Self::charge_verification_fee_from_payer(obj_idx, &obj_data.fee_payer, obj.len())?;
            
            // Calculate hashes for the object
            let hashes = Self::calc_hashes(&obj_data.category, None, &obj.to_vec(), DEFAULT_OBJECT_HASHES)?;
            
            // Verify at least one hash matches (replica requirement)
            if obj_data.is_replica {
                if let Some(orig_idx) = obj_data.original_obj {
                    let orig = Objects::<T>::get(orig_idx).ok_or(Error::<T>::OriginalNotFound)?;
                    let match_found = hashes.iter().any(|h| orig.hashes.contains(h));
                    if !match_found {
                        return Err(Error::<T>::NotAReplica.into());
                    }
                }
            }
            
            // Update the object with the actual data and move to Created state
            let block_num = <frame_system::Pallet<T>>::block_number();
            obj_data.state = ObjectState::<T::BlockNumber, T::AccountId>::Created(block_num);
            obj_data.obj = obj;
            obj_data.compressed_with = Some(CompressWith::Lzss);
            obj_data.hashes = hashes;
            
            // Insert the updated object
            Objects::<T>::insert(obj_idx, obj_data);
            
            Self::deposit_event(Event::ObjCreated(who));
            Ok(().into())
        }

		/// Unlock unspent rewards for a NotApproved object
		#[pallet::weight(1_000_000)]
		pub fn unlock_unspent_rewards(
			origin: OriginFor<T>,
			obj_idx: ObjIdx,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			// Get the object data
			let obj_data = Objects::<T>::get(&obj_idx)
				.ok_or(Error::<T>::ObjectNotFound)?;

			// Ensure the object is in NotApproved state
			ensure!(
				matches!(obj_data.state, ObjectState::<T::BlockNumber, T::AccountId>::NotApproved(_)),
				Error::<T>::ObjectNotNotApproved
			);

			// Ensure the caller is the fee payer (who initially locked the funds)
			ensure!(obj_data.fee_payer == who, Error::<T>::NotFeePayer);

			// Calculate unspent rewards
			// If the object went through QC inspection and was rejected without estimation,
			// the user should get back both estimator and author rewards since no work was performed
			let unspent_rewards = if obj_data.estimators.is_empty() {
				// No estimators participated, so both est_rewards and author_rewards are unspent
				obj_data.est_rewards.saturated_into::<BalanceOf<T>>().saturating_add(obj_data.author_rewards.saturated_into())
			} else {
				// Estimators participated, so only author_rewards are unspent
				obj_data.author_rewards.saturated_into()
			};

			// Ensure there are unspent rewards to unlock
			ensure!(unspent_rewards > BalanceOf::<T>::zero(), Error::<T>::NoUnspentRewards);

			// Get current locked amount
			let current_locked = AccountLock::<T>::get(&who);

			// Calculate new locked amount (subtract unspent rewards)
			let new_locked = current_locked.saturating_sub(unspent_rewards);

			// Update the lock
			Self::set_lock(&who, new_locked);

			// Emit event
			Self::deposit_event(Event::UnspentRewardsUnlocked(obj_idx, who.clone(), unspent_rewards));

			log::debug!(target: LOG_TARGET, "unlock_unspent_rewards: unlocked {:?} for obj_idx={} owner={:?} (estimators_participated={})", 
				unspent_rewards, obj_idx, who, !obj_data.estimators.is_empty());

			Ok(().into())
		}

		#[pallet::weight(1_000_000)]
		pub fn set_author_part(
			origin: OriginFor<T>,
			part: Percent,
		) -> DispatchResultWithPostInfo {
			T::CouncilOrigin::ensure_origin(origin)?;
			AuthorPart::<T>::put(part);
			// Optionally emit an event (add to Event enum if needed)
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
			if let ObjectState::<T::BlockNumber, T::AccountId>::Created(_) = obj_data.state {
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
		let current_block = <frame_system::Pallet<T>>::block_number();
		
		for (idx, obj_data) in Objects::<T>::iter() {
			if let ObjectState::<T::BlockNumber, T::AccountId>::Estimated(when, _) = obj_data.state {
				// Check if approval timeout has expired
				if current_block.saturating_sub(when) < T::ApproveTimeout::get().into() {
					v.push((idx, obj_data));
				} else {
					log::info!(target: LOG_TARGET, "Estimation of object {} has expired (timeout)", &idx);
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
					if let ObjectState::<T::BlockNumber, T::AccountId>::Estimating(..) = obj_data.state {
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
			let base_rewards = Rewards::<T>::get().unwrap_or(T::RewardsDefault::get().saturated_into());
			let dynamic_rewards = Self::calculate_dynamic_rewards(base_rewards);
			let author_rewards = author_part * dynamic_rewards;
			let est_rewards = dynamic_rewards.saturating_sub(author_rewards);
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
			if matches!(obj.state, ObjectState::<T::BlockNumber, T::AccountId>::Approved(_)) {
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

	/// Returns a list of object indexes that are replicas of the given object index
	pub fn replicas_of(original_idx: ObjIdx) -> Vec<ObjIdx> {
		Objects::<T>::iter()
			.filter_map(|(idx, obj)| {
				if obj.is_replica {
					if let Some(orig) = obj.original_obj {
						if orig == original_idx {
							return Some(idx);
						}
					}
				}
				None
			})
			.collect()
	}

	/// Count objects in the queue (waiting for processing)
	fn count_objects_in_queue() -> u32 {
		let mut count = 0u32;
		for (_, obj_data) in Objects::<T>::iter() {
			match obj_data.state {
				ObjectState::<T::BlockNumber, T::AccountId>::Created(_) |
				ObjectState::<T::BlockNumber, T::AccountId>::Estimating(_) |
				ObjectState::<T::BlockNumber, T::AccountId>::QCInspecting(_, _) => {
					count += 1;
				},
				_ => {}
			}
		}
		count
	}

	/// Calculate dynamic rewards based on queue size using exponential formula
	fn calculate_dynamic_rewards(base_rewards: BalanceOf<T>) -> BalanceOf<T> {
		let queue_size = Self::count_objects_in_queue();
		
		// If no objects in queue, return base rewards
		if queue_size == 0 {
			return base_rewards;
		}
		
		// Get the growth rate from storage or use default from configuration
		let growth_rate = DynamicRewardsGrowthRate::<T>::get().unwrap_or(T::DynamicRewardsGrowthRate::get()) as f64;
		
		// Exponential formula: base_rewards * (1 + queue_size^0.5 / growth_rate)
		// Higher growth_rate = slower growth, lower growth_rate = faster growth
		let queue_factor = (queue_size as f64).sqrt() / growth_rate as f64;
		let multiplier = 1.0 + queue_factor;
		
		// Convert to fixed-point arithmetic for precision
		let base_u128: u128 = base_rewards.saturated_into();
		let multiplier_u128 = (multiplier * 1_000_000.0) as u128; // 6 decimal places precision
		let result = (base_u128 * multiplier_u128) / 1_000_000;
		
		log::debug!(target: LOG_TARGET, "Dynamic rewards: queue_size={}, base_rewards={}, growth_rate={}, multiplier={}, result={}", 
			queue_size, base_u128, growth_rate, multiplier, result);
		
		result.saturated_into()
	}

	/// Charge verification fee from the fee payer account
	fn charge_verification_fee_from_payer(
		obj_idx: ObjIdx,
		fee_payer: &T::AccountId,
		obj_len: usize,
	) -> Result<(), Error<T>> {
		let fee_per_byte = Self::fee_per_byte().unwrap_or(FEE_PER_BYTE);
		let total_fee = fee_per_byte * obj_len as u64;
		
		// Check if fee payer has sufficient balance
		let fee_payer_balance = T::Currency::free_balance(fee_payer);
		ensure!(fee_payer_balance >= total_fee.saturated_into(), Error::<T>::FeePayerInsufficientBalance);
		
		// Charge the fee from the fee payer using RESERVE instead of FEE to avoid burning
		T::Currency::withdraw(fee_payer, total_fee.saturated_into(), WithdrawReasons::RESERVE, ExistenceRequirement::KeepAlive)
			.map_err(|_| Error::<T>::FeePayerInsufficientBalance)?;
		
		// Add the fee to pending storage fees for distribution to validators
		PendingStorageFees::<T>::mutate(|pending_fees| {
			*pending_fees = pending_fees.saturating_add(total_fee.saturated_into());
		});
		
		// Emit event for fee charging
		Self::deposit_event(Event::VerificationFeeCharged(obj_idx, fee_payer.clone(), total_fee.saturated_into()));
		
		log::info!(target: LOG_TARGET, "Charged verification fee {} from fee payer {:?} for object size {}, added to pending storage fees", total_fee, fee_payer, obj_len);
		Ok(())
	}

	/// Distribute pending storage fees to validators
	fn distribute_storage_fees_to_validators() {
		let pending_fees = PendingStorageFees::<T>::get();
		if pending_fees.is_zero() {
			return;
		}

		let validators = T::ValidatorSet::validators();
		if validators.is_empty() {
			log::warn!(target: LOG_TARGET, "No validators available for storage fee distribution");
			return;
		}

		let fee_per_validator = pending_fees / validators.len().saturated_into();
		let mut total_distributed = BalanceOf::<T>::zero();

		for validator in validators.iter() {
			// Transfer fee to validator
			let _res = T::Currency::deposit_creating(validator, fee_per_validator);
			total_distributed = total_distributed.saturating_add(fee_per_validator);
		}

		// Clear pending fees
		PendingStorageFees::<T>::put(BalanceOf::<T>::zero());

		// Emit event for fee distribution
		Self::deposit_event(Event::StorageFeesDistributed(total_distributed));

		log::info!(target: LOG_TARGET, "Distributed {:?} storage fees to {} validators ({:?} per validator)", 
			total_distributed, validators.len(), fee_per_validator);
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
    fn replicas_of(original_idx: u32) -> Vec<u32> {
        Self::replicas_of(original_idx)
    }
    fn object_has_asset(_obj_idx: u32) -> bool {
        false // stub for trait compliance
	}
	fn get_dynamic_rewards_growth_rate() -> Option<u32> {
		DynamicRewardsGrowthRate::<T>::get()
	}
	fn get_author_part() -> Option<u8> {
		AuthorPart::<T>::get().map(|part| part.deconstruct())
	}
	fn get_unspent_rewards(obj_idx: u32) -> Option<u128> {
		let obj_data = Objects::<T>::get(&obj_idx)?;
		
		// Only return unspent rewards for NotApproved objects
		match obj_data.state {
			ObjectState::<T::BlockNumber, T::AccountId>::NotApproved(_) => {
				// If no estimators participated, return both est_rewards and author_rewards
				// If estimators participated, return only author_rewards
				if obj_data.estimators.is_empty() {
					Some(obj_data.est_rewards.saturated_into::<u128>().saturating_add(obj_data.author_rewards.saturated_into()))
				} else {
					Some(obj_data.author_rewards)
				}
			},
			_ => None,
		}
	}
	fn get_fee_payer(obj_idx: u32) -> Option<T::AccountId> {
		let obj_data = Objects::<T>::get(&obj_idx)?;
		Some(obj_data.fee_payer)
	}
	fn get_pending_storage_fees() -> Option<u128> {
		Some(PendingStorageFees::<T>::get().saturated_into())
	}
	fn get_rewards() -> Option<u128> {
		Some(PendingStorageFees::<T>::get().saturated_into())
	}
	fn get_qc_timeout(obj_idx: u32) -> Option<u32> {
		let obj_data = Objects::<T>::get(&obj_idx)?;
		Some(obj_data.qc_timeout)
	}
}
