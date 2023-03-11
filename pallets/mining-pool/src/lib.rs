//! # Mining Pool Pallet
//!
//! The Mining Pool Pallet allows addition and removal of
//! pool's admins and members via extrinsics (transaction calls)
//! Substrate-based PoA networks. It also integrates with the Identity pallet
//!
#![allow(warnings)]
#![cfg_attr(not(feature = "std"), no_std)]

mod mock;
mod tests;

extern crate alloc;
use alloc::string::String;
use log;
use core::convert::TryInto;
use scale_info::prelude::format;

use frame_support::{
	ensure,
	pallet_prelude::*,
	traits::{
		Currency, LockableCurrency, EstimateNextSessionRotation,
		Get, ValidatorSet, ValidatorSetWithIdentification,
		OnUnbalanced, ExistenceRequirement, LockIdentifier, WithdrawReasons,
		OneSessionHandler,
	},
	sp_runtime::SaturatedConversion,
	BoundedSlice, WeakBoundedVec,
};
// use frame_system::offchain::{
// 	Signer, SigningTypes, SignedPayload, AppCrypto, CreateSignedTransaction,
// };
use frame_system::offchain::{
	AppCrypto, CreateSignedTransaction,
	SendTransactionTypes, SubmitTransaction, Signer,
};

pub use pallet::*;
use sp_runtime::traits::{Convert, Zero};
// use sp_runtime::offchain::storage_lock::{BlockAndTime, StorageLock};
use sp_runtime::{
	offchain::{
		storage::{
			MutateStorageError, StorageRetrievalError, StorageValueRef,
		},
		storage_lock::{StorageLock, Time},
	},
};
pub use sp_runtime::Percent;
use sp_application_crypto::RuntimeAppPublic;

use sp_runtime::traits::BlockNumberProvider;
use core::time::Duration;
use sp_staking::offence::{Offence, OffenceError, ReportOffence};
use sp_std::{collections::btree_set::BTreeSet, prelude::*};
use sp_core::U256;
// use sp_core::offchain::{storage::InMemOffchainStorage, OffchainStorage, OpaqueNetworkState};

use sp_core::offchain::OpaqueNetworkState;
use codec::{Decode, Encode, MaxEncodedLen, FullCodec};
use frame_system::Account;

use rewards_api::RewardLocksApi;
use mining_pool_stat_api::MiningPoolStatApi;
use pallet_identity::Judgement;

pub const LOG_TARGET: &'static str = "mining-pool";

pub type PoolId<T> = <T as frame_system::Config>::AccountId;

pub type BalanceOf<T> =
<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

const STAT_LOCK: &'static [u8] = b"mining-pool::lock";

use sp_application_crypto::KeyTypeId;
pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"pool");

pub mod sr25519 {
	mod app_sr25519 {
		use super::super::KEY_TYPE;
		use sp_application_crypto::{app_crypto, sr25519};
		app_crypto!(sr25519, KEY_TYPE);
	}

	sp_application_crypto::with_pair! {
		/// An i'm online keypair using sr25519 as its crypto.
		pub type AuthorityPair = app_sr25519::Pair;
	}

	/// An i'm online signature using sr25519 as its crypto.
	pub type AuthoritySignature = app_sr25519::Signature;

	/// An i'm online identifier using sr25519 as its crypto.
	pub type PoolAuthorityId = app_sr25519::Public;

}

// pub mod crypto {
// 	use super::KEY_TYPE;
// 	use sp_core::sr25519::Signature as Sr25519Signature;
// 	use sp_runtime::{
// 		app_crypto::{app_crypto, sr25519},
// 		traits::Verify,
// 		MultiSignature, MultiSigner,
// 	};
// 	app_crypto!(sr25519, KEY_TYPE);
//
// 	pub struct PoolAuthorityId;
//
// 	// impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for PoolAuthorityId {
// 	// 	type RuntimeAppPublic = Public;
// 	// 	type GenericSignature = sp_core::sr25519::Signature;
// 	// 	type GenericPublic = sp_core::sr25519::Public;
// 	// }
//
// 	// implemented for mock runtime in test
// 	impl frame_system::offchain::AppCrypto<<Sr25519Signature as Verify>::Signer, Sr25519Signature>
// 	for PoolAuthorityId
// 	{
// 		type RuntimeAppPublic = Public;
// 		type GenericSignature = sp_core::sr25519::Signature;
// 		type GenericPublic = sp_core::sr25519::Public;
// 	}
// }

pub type AuthIndex = u32;
// use crate::sr25519::PoolAuthorityId;

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct MiningStat<AccountId>
	where
	 	//BlockNumber: PartialEq + Eq + Decode + Encode,
	 	AccountId: PartialEq + Eq + Decode + Encode,
{
	/// Block number at the time heartbeat is created..
	// pub block_number: BlockNumber,
	/// A state of local network (peer id and external addresses)
	pub network_state: OpaqueNetworkState,
	/// Index of the current session.
	// pub session_index: SessionIndex,
	/// An index of the authority on the list of validators.
	pub authority_index: AuthIndex,
	/// The length of session validator set
	// pub validators_len: u32,
	pub pool_id: AccountId,
	pub pow_stat: Vec<(AccountId, u32)>,
	// public: Public,
}

// #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
// pub struct PricePayload<Public, BlockNumber> {
// 	block_number: BlockNumber,
// 	price: u32,
// 	public: Public,
// }

// impl<T: SigningTypes> SignedPayload<T> for MiningStat<T::Public, T::AccountId> {
// 	fn public(&self) -> T::Public {
// 		self.public.clone()
// 	}
// }

/// Error which may occur while executing the off-chain code.
#[derive(PartialEq)]
enum OffchainErr {
	FailedSigning,
	FailedToAcquireLock,
	NetworkState,
	SubmitTransaction,
}

impl sp_std::fmt::Debug for OffchainErr {
	fn fmt(&self, fmt: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		match *self {
			OffchainErr::FailedSigning => write!(fmt, "Failed to sign heartbeat"),
			OffchainErr::FailedToAcquireLock => write!(fmt, "Failed to acquire lock"),
			OffchainErr::NetworkState => write!(fmt, "Failed to fetch network state"),
			OffchainErr::SubmitTransaction => write!(fmt, "Failed to submit transaction"),
		}
	}
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_system::pallet_prelude::*;

	/// Configure the pallet by specifying the parameters and types on which it
	/// depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_validator_set::Config + pallet_session::Config + pallet_identity::Config + CreateSignedTransaction<Call<Self>>{
		/// The Event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type Call: From<Call<Self>>;

		/// Origin for adding or removing a validator.
		type AddRemoveOrigin: EnsureOrigin<Self::Origin>;

		type PoolAuthorityId: Member
			+ Parameter
			+ RuntimeAppPublic
			+ Ord
			+ MaybeSerializeDeserialize
			+ MaxEncodedLen;

		type MaxKeys: Get<u32>;

		type MaxMembers: Get<u32>;

		type Currency: LockableCurrency<Self::AccountId>;

		type RewardLocksApi: RewardLocksApi<Self::AccountId, BalanceOf<Self>>;

		type Difficulty: FullCodec + From<U256>;

		#[pallet::constant]
		type PoscanEngineId: Get<[u8; 4]>;

		#[pallet::constant]
		type UnsignedPriority: Get<TransactionPriority>;

		#[pallet::constant]
		type StatPeriod: Get<Self::BlockNumber>;

		#[pallet::constant]
		type MaxPoolPercent: Get<Percent>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// The current set of keys that may issue a heartbeat.
	#[pallet::storage]
	#[pallet::getter(fn keys)]
	pub(crate) type Keys<T: Config> =
		StorageValue<_, WeakBoundedVec<T::PoolAuthorityId, T::MaxKeys>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn pools)]
	pub type Pools<T: Config> = StorageMap<_, Twox64Concat, PoolId<T>, Vec<T::AccountId>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn pool_rewards)]
	pub type PoolRewards<T: Config> = StorageMap<_, Twox64Concat, PoolId<T>, Percent, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn mining_stat)]
	pub type PowStat<T: Config> = StorageMap<_, Twox64Concat, PoolId<T>, Vec<(T::AccountId, u32)>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn pow_difficulty)]
	pub type PowDifficulty<T: Config> = StorageMap<_, Twox64Concat, PoolId<T>, U256, OptionQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub keys: Vec<T::PoolAuthorityId>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig { keys: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			Pallet::<T>::initialize_keys(&self.keys);
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// New pool created.
		PoolCreated(T::AccountId),
		/// Pool removed.
		PoolRemoved(T::AccountId),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Minig pool not found
		PoolNotFound,
		/// Member is already in the pool.
		Duplicate,
		/// Pool rewards is higher the maximum.
		TooHighPoolRewards,
		/// Member count is higher the maximum.
		TooHighPoolCount,
		/// Member nas no registrar's label.
		NotRegistered,
		/// No pool
		PoolNotExists,
		/// Pool already exists.
		PoolAlreadyExists,
		/// Member is not approved
		MemberNotApproved,

		BadOrigin,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn offchain_worker(block_number: T::BlockNumber) {
			log::debug!(target: LOG_TARGET, "Hello World from offchain workers!");

			if Self::set_sent(block_number) {
                Self::send_mining_stat();
            }
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		pub fn set_pool_interest(origin: OriginFor<T>, percent: Percent) -> DispatchResult {
			let pool_id = ensure_signed(origin)?;

			if !<Pools<T>>::contains_key(&pool_id) {
				return Err(Error::<T>::PoolNotExists.into());
			}

			if percent > T::MaxPoolPercent::get() {
				return Err(Error::<T>::TooHighPoolRewards.into());
			}
			<PoolRewards<T>>::insert(pool_id, percent);
			Ok(())
		}

		#[pallet::weight(0)]
		pub fn create_pool(origin: OriginFor<T>) -> DispatchResult {
			let pool_id = ensure_signed(origin)?;

			if <Pools<T>>::contains_key(&pool_id) {
				return Err(Error::<T>::PoolAlreadyExists.into());
			}

			let reg = pallet_identity::Pallet::<T>::identity(&pool_id);
			if let Some(reg) = reg {
				for (rgstr_idx, judge) in reg.judgements {
					match judge {
						Judgement::Reasonable => {
							<Pools<T>>::insert(&pool_id, Vec::<T::AccountId>::new());
							Self::deposit_event(Event::PoolCreated(pool_id.clone()));
							return Ok(());
						},
						_ => {},
					}
				}
			}

			return Err(Error::<T>::NotRegistered.into());
		}

		#[pallet::weight(0)]
		pub fn set_pool_difficulty(origin: OriginFor<T>, difficulty: U256) -> DispatchResult {
			let pool_id = ensure_signed(origin)?;

			if !<Pools<T>>::contains_key(&pool_id) {
				return Err(Error::<T>::PoolNotExists.into());
			}

			<PowDifficulty<T>>::insert(pool_id, difficulty);
			Ok(())
		}

		#[pallet::weight(0)]
		pub fn add_member(origin: OriginFor<T>, member_id: T::AccountId) -> DispatchResult {
			let pool_id = ensure_signed(origin)?;

			if !<Pools<T>>::contains_key(&pool_id) {
				return Err(Error::<T>::PoolNotExists.into());
			}

			let reg = pallet_identity::Pallet::<T>::identity(&member_id);
			if let Some(reg) = reg {
				for (rgstr_idx, judge) in reg.judgements {
					match judge {
						Judgement::Reasonable => {
							<Pools<T>>::mutate(&pool_id, |v| v.push(member_id.clone()));
							log::debug!(target: LOG_TARGET, "member added");
							Self::deposit_event(Event::PoolCreated(pool_id.clone()));
							return Ok(());
						},
						_ => {},
					}
				}
			}

			return Err(Error::<T>::NotRegistered.into());
		}

		#[pallet::weight(0)]
		pub fn close_pool(
			origin: OriginFor<T>,
		) -> DispatchResult {
			let pool_id = ensure_signed(origin)?;
			if !<Pools<T>>::contains_key(&pool_id) {
				return Err(Error::<T>::PoolNotExists.into());
			}
			<Pools<T>>::remove(pool_id);
			Ok(())
		}

		#[pallet::weight(0)]
		pub fn remove_member(
			origin: OriginFor<T>,
			member_id: T::AccountId,
		) -> DispatchResult {
			let pool_id = ensure_signed(origin)?;

			if !<Pools<T>>::contains_key(&pool_id) {
				return Err(Error::<T>::PoolNotExists.into());
			}
			<Pools<T>>::mutate(pool_id, |v| v.retain(|m| *m != member_id.clone()));
			Ok(())
		}

		#[pallet::weight(0)]
		pub fn submit_mining_stat(
			origin: OriginFor<T>,
			mining_stat: MiningStat<T::AccountId>,
			_signature: <T::PoolAuthorityId as RuntimeAppPublic>::Signature,
		) -> DispatchResultWithPostInfo {
			log::debug!(target: LOG_TARGET, "submit_mining_stat");

			ensure_none(origin)?;
			let pool_id = mining_stat.pool_id;

			ensure!(<Pools<T>>::contains_key(&pool_id), Error::<T>::PoolNotFound);

			let pool = <Pools<T>>::get(&pool_id);
			let mut members: Vec<(T::AccountId, u32)> = mining_stat.pow_stat.into_iter().filter(|ms| pool.contains(&ms.0)).collect();
			log::debug!(target: LOG_TARGET, "submit_mining_stat stat: pool_id");
			PowStat::<T>::insert(&pool_id, &members);
			Ok(().into())
		}
	}

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			if let Call::submit_mining_stat { mining_stat, signature } = call {
				let pool_id = &mining_stat.pool_id;
				let authority_id = T::PoolAuthorityId::decode(&mut &pool_id.encode()[..]).unwrap();

				log::debug!(target: LOG_TARGET, "validate_unsigned for pool");

				// check signature (this is expensive so we do it last).
				let signature_valid = mining_stat.using_encoded(|encoded_stat| {
					authority_id.verify(&encoded_stat, signature)
				});

				if !signature_valid {
					log::debug!(target: LOG_TARGET, "validate_unsigned::InvalidTransaction::BadProof");
					return InvalidTransaction::BadProof.into()
				}

				ValidTransaction::with_tag_prefix("MiningPool")
					.priority(T::UnsignedPriority::get())
					// .and_provides((current_session, authority_id))
					.and_provides(authority_id)
					.longevity(
						5u64,
					)
					.propagate(true)
					.build()
			} else {
				log::debug!(target: LOG_TARGET, "validate_unsigned::InvalidTransaction::Call");
				InvalidTransaction::Call.into()
			}
		}
	}
}

impl<T: Config> Pallet<T> {
	fn initialize_keys(keys: &[T::PoolAuthorityId]) {
		if !keys.is_empty() {
			assert!(Keys::<T>::get().is_empty(), "Keys are already initialized!");
			let bounded_keys = <BoundedSlice<'_, _, T::MaxKeys>>::try_from(keys)
				.expect("More than the maximum number of keys provided");
			Keys::<T>::put(bounded_keys);
		}
	}

	fn send_mining_stat() -> Result<(), &'static str> {
		log::debug!(target: LOG_TARGET, "send_mining_stat");
		let mut local_keys = T::PoolAuthorityId::all();
		log::debug!(target: LOG_TARGET, "Number of PoolAuthorityId keys: {}", local_keys.len());

		let pool_key = local_keys[0].clone();
		log::debug!(target: LOG_TARGET, "pool_key = {}", hex::encode(&pool_key.encode()));
		let pool_id = T::AccountId::decode(&mut &pool_key.encode()[..]).unwrap();
		log::debug!(target: LOG_TARGET, "pool_id = {}", hex::encode(&pool_id.encode()));

		let network_state = sp_io::offchain::network_state().map_err(|_| "OffchainErr::NetworkState")?;

		let base_key = Self::storage_key(&pool_id);
		log::debug!(target: LOG_TARGET, "base_storage_key={}", &base_key);

		let member_ids = <Pools<T>>::get(&pool_id);

		if member_ids.len() == 0 {
			log::debug!(target: LOG_TARGET, "pool is empty");
			return Ok(())
		}

		let mut pow_stat = Vec::new();
		let mut lock = StorageLock::<Time>::new(STAT_LOCK);
		{
			let _guard = lock.lock();

			for member_id in member_ids.clone() {
				let member_key = format!("{}::{}", &base_key, hex::encode(&member_id.encode()));
				log::debug!(target: LOG_TARGET, "Collect stat: key={}", &member_key);

				let val = StorageValueRef::persistent(member_key.as_bytes());
				let stat = val.get();
				match stat {
					Ok(Some(stat)) => {
						log::debug!(target: LOG_TARGET, "Extract stat from local storage: stat={:?}", &u32::from_le_bytes(stat));
						// MiningStat { authority_index: 0, network_state, pool_id, pow_stat: v }
						pow_stat.push((member_id, u32::from_le_bytes(stat)));
					},
					Ok(None) => {
						log::debug!(target: LOG_TARGET, "No stat in local storage");
						return Ok(())
					},
					Err(e) => {
						log::debug!(target: LOG_TARGET, "Error extracting sts from local storage");
						return Err("Err")
					},
				};
			}

			let mut mining_stat = MiningStat { authority_index: 0, network_state, pool_id, pow_stat };

			log::debug!(target: LOG_TARGET, "Sign mining_stat call");
			let signature = pool_key.sign(&mining_stat.encode()).ok_or("OffchainErr::FailedSigning")?;

			log::debug!(target: LOG_TARGET, "Call::submit_mining_stat");
			let call = Call::submit_mining_stat { mining_stat, signature };

			SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
				.map_err(|_| "OffchainErr::SubmitTransaction")?;

			for member_id in member_ids {
				let member_key = format!("{}::{}", &base_key, hex::encode(&member_id.encode()));
				log::debug!(target: LOG_TARGET, "clear stat: key={}", &member_key);

				let mut val = StorageValueRef::persistent(member_key.as_bytes());
				val.clear();
			}

			log::debug!(target: LOG_TARGET, "Call::submit_mining_stat - ok");
		}

		Ok(())
	}

	fn storage_key(pool_id: &T::AccountId) -> String {
		let key = format!("stat::{}", hex::encode(pool_id.encode()));
		key
	}

	fn set_sent(block_number: T::BlockNumber) -> bool {
		let res;
		let mut lock = StorageLock::<Time>::new(STAT_LOCK);
		{
			let _guard = lock.lock();
			let val = StorageValueRef::persistent(b"stat::last_sent");

			res = val.mutate(|last_send: Result<Option<T::BlockNumber>, StorageRetrievalError>| {
				match last_send {
					// If we already have a value in storage and the block number is recent enough
					// we avoid sending another transaction at this time.

					// Ok(Some(block)) if block_number < block + T::GracePeriod::get() =>
					Ok(Some(block)) if block_number < block + T::StatPeriod::get() =>
						Err("RECENTLY_SENT"),
					// In every other case we attempt to acquire the lock and send a transaction.
					_ => Ok(block_number),
				}
			});
			log::debug!(target: LOG_TARGET, "Last sent block written to local storage: {}", res.is_ok());
		}
		// TODO: check res correctly
		res.is_ok()
	}
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

impl<
	T: Config + frame_system::Config<AccountId = AccountId>,
	Difficulty: FullCodec + Default + Clone + Ord + From<U256>,
	AccountId: FullCodec + Clone + Ord + 'static
>
	MiningPoolStatApi<Difficulty, AccountId> for Pallet<T> {

	/// Return the target difficulty of the next block.
	fn difficulty(pool_id: &AccountId) -> Difficulty {
		let maybe_dfclty = Self::pow_difficulty(pool_id);
		if let Some(dfclty) = maybe_dfclty {
			dfclty.into()
		}
		else {
			Difficulty::from(U256::from(20))
		}
	}

	fn get_stat(pool_id: &T::AccountId) -> Option<(Percent, Percent, Vec<(T::AccountId, u32)>)> {
		let pool_part = <PoolRewards<T>>::get(pool_id.clone());
		let members_stat = <PowStat<T>>::get(pool_id);
		let cur_block_number = <frame_system::Pallet<T>>::block_number();

		let mut counter = 0u32;
		let mut n = 0u32;
		loop {
			n += 1;
			let block_num = cur_block_number - n.into();
			if block_num < 1u32.into() || n > 100u32 {
				break;
			}
			if let Some(ref author_id) = pallet_validator_set::Authors::<T>::get(block_num) {
				if pool_id == author_id {
					counter += 1;
				}
			}
		}
		let win_rate = Percent::from_rational(counter, n);

		// let a = pallet_validator_set::Authors::<T>::get(cur_block_number);
		Some((pool_part, win_rate, members_stat))
	}
}