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
use sp_std::cmp::Ordering;
use sp_staking::offence::{Offence, OffenceError, ReportOffence};
use sp_std::{
	collections::{
		btree_set::BTreeSet,
		btree_map::{BTreeMap, Entry},
	},
	prelude::*};
use sp_core::U256;

use sp_core::offchain::OpaqueNetworkState;
use codec::{Decode, Encode, MaxEncodedLen, FullCodec};
use frame_system::Account;

use rewards_api::RewardLocksApi;
use mining_pool_stat_api::{MiningPoolStatApi, CheckMemberError};
use pallet_identity::{Registration, Judgement, IdentityInfo, Data};

pub const LOG_TARGET: &'static str = "mining-pool";

pub type PoolId<T> = <T as frame_system::Config>::AccountId;

pub type BalanceOf<T> =
<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

const STAT_LOCK: &'static [u8] = b"mining-pool::lock";

use sp_application_crypto::KeyTypeId;
pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"pool");

pub mod sr25519 {
	pub mod app_sr25519 {
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

// #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
#[derive(Clone, Ord, PartialOrd, RuntimeDebug)] //Ord, PartialOrd,
pub(crate) struct IdentInfo {
	pub(crate) good_judjements: u32,
	pub(crate) total_judjements: u32,
	
	pub(crate) email: Option<String>,
	pub(crate) twitter: Option<String>,
	pub(crate) discord: Option<String>,
	pub(crate) telegram: Option<String>,
}

impl PartialEq for IdentInfo {
	fn eq(&self, other: &Self) -> bool {
		if let (Some(a), Some(b)) = (self.email.as_ref(), other.email.as_ref()) { a == b }
		else if let (Some(a), Some(b)) = (self.twitter.as_ref(), other.twitter.as_ref()) { a == b }
		else if let (Some(a), Some(b)) = (self.discord.as_ref(), other.discord.as_ref()) { a == b }
		else if let (Some(a), Some(b)) = (self.telegram.as_ref(), other.telegram.as_ref()) { a == b }
		else {
			false
		}
	}
}

impl Eq for IdentInfo {}

/// Error which may occur while executing the off-chain code.
#[derive(PartialEq)]
enum OffchainErr {
	FailedSigning,
	FailedToAcquireLock,
	NetworkState,
	SubmitTransaction,
	NotIdentitied,
}

impl sp_std::fmt::Debug for OffchainErr {
	fn fmt(&self, fmt: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		match *self {
			OffchainErr::FailedSigning => write!(fmt, "Failed to sign heartbeat"),
			OffchainErr::FailedToAcquireLock => write!(fmt, "Failed to acquire lock"),
			OffchainErr::NetworkState => write!(fmt, "Failed to fetch network state"),
			OffchainErr::SubmitTransaction => write!(fmt, "Failed to submit transaction"),
			OffchainErr::NotIdentitied => write!(fmt, "Account is not identified"),
		}
	}
}

enum IdentityErr<AccountId> {
	/// Account is not identified
	NoIdentity,
	/// Not enough good judjement
	NotEnoughJudjement,
	/// Identity duplicates found
	PoolDuplicates(Vec<AccountId>),
	/// Identity duplicates found
	Duplicates(Vec<(AccountId, AccountId)>),
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_system::pallet_prelude::*;

	/// Configure the pallet by specifying the parameters and types on which it
	/// depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_session::Config + pallet_validator_set::Config + pallet_session::Config + pallet_identity::Config + CreateSignedTransaction<Call<Self>>{
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

		#[pallet::constant]
		type MaxPools: Get<u32>;

		#[pallet::constant]
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
	#[pallet::getter(fn suspended_pools)]
	pub type SuspendedPools<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn pool_rewards)]
	pub type PoolRewards<T: Config> = StorageMap<_, Twox64Concat, PoolId<T>, Percent, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn mining_stat)]
	pub type PowStat<T: Config> = StorageMap<_, Twox64Concat, PoolId<T>, Vec<(T::AccountId, u32)>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn pow_difficulty)]
	pub type PowDifficulty<T: Config> = StorageMap<_, Twox64Concat, PoolId<T>, U256, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn with_kyc)]
	pub type PoolMode<T: Config> = StorageMap<_, Twox64Concat, PoolId<T>, bool, ValueQuery>;

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
		/// Pool removed.
		PoolSuspended(T::AccountId),
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
		/// Pool size exсeeds max.
		PoolSizeMax,
		/// Member size exсeeds max.
		MemberSizeMax,
		/// Pool/Member nas no registrar's label.
		NotRegistered,
		/// No pool
		PoolNotExists,
		/// No member in pool
		MemberNotExists,
		/// Pool already exists.
		PoolAlreadyExists,
		/// Account is not identified
		NoIdentity,
		/// Not enough good judjement
		NotEnoughJudjement,
		/// Account with the same identity exists
		IdentityDuplicate,
		/// Pool is suspended
		PoolSuspended,
		/// Duplicated pools with the same identities suspended
		PoolDuplicatesSuspended,
		/// Duplicated embers with the same identities removed
		MemberDuplicatesRemoved,

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

		fn on_finalize(n: T::BlockNumber) {
			if n % 20u32.into() == T::BlockNumber::zero() {
				// Escape members by identity reasons
				Self::sync_identity();
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		pub fn set_pool_interest(origin: OriginFor<T>, percent: Percent) -> DispatchResult {
			let pool_id = ensure_signed(origin)?;
			ensure!(<Pools<T>>::contains_key(&pool_id), Error::<T>::PoolNotExists);
			ensure!(percent <= T::MaxPoolPercent::get(), Error::<T>::TooHighPoolRewards);

			<PoolRewards<T>>::insert(pool_id, percent);
			log::debug!(target: LOG_TARGET, "set pool interest - ok");

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn create_pool(origin: OriginFor<T>) -> DispatchResult {
			let pool_id = ensure_signed(origin)?;
			ensure!(!<Pools<T>>::contains_key(&pool_id), Error::<T>::PoolAlreadyExists);
			ensure!((<Pools<T>>::iter_keys().count() as u32) < T::MaxPools::get(), Error::<T>::PoolSizeMax);
			match Self::check_pool_identity(&pool_id) {
				Err(IdentityErr::NoIdentity) => return Err(Error::<T>::NoIdentity.into()),
				Err(IdentityErr::NotEnoughJudjement) => return Err(Error::<T>::NotEnoughJudjement.into()),
				Err(IdentityErr::PoolDuplicates(_)) |
				Err(IdentityErr::Duplicates(_)) => return Err(Error::<T>::IdentityDuplicate.into()),
				Ok(()) => {},
			}

			<Pools<T>>::insert(&pool_id, Vec::<T::AccountId>::new());
			<PoolMode<T>>::insert(&pool_id, true);
			log::debug!(target: LOG_TARGET, "pool created");

			Self::deposit_event(Event::PoolCreated(pool_id.clone()));

			return Ok(());
		}

		#[pallet::weight(0)]
		pub fn set_pool_difficulty(origin: OriginFor<T>, difficulty: U256) -> DispatchResult {
			let pool_id = ensure_signed(origin)?;
			ensure!(<Pools<T>>::contains_key(&pool_id), Error::<T>::PoolNotExists);

			<PowDifficulty<T>>::insert(pool_id, difficulty);
			log::debug!(target: LOG_TARGET, "set_pool_difficulty - ok");

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn set_pool_mode(origin: OriginFor<T>, with_kyc: bool) -> DispatchResult {
			let pool_id = ensure_signed(origin)?;
			ensure!(<Pools<T>>::contains_key(&pool_id), Error::<T>::PoolNotExists);

			<PoolMode<T>>::insert(pool_id, with_kyc);
			log::debug!(target: LOG_TARGET, "set_pool_mode - ok");

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn add_member(origin: OriginFor<T>, member_id: T::AccountId) -> DispatchResult {
			let pool_id = ensure_signed(origin)?;
			ensure!(<Pools<T>>::contains_key(&pool_id), Error::<T>::PoolNotExists);
			ensure!(!<Pools<T>>::get(&pool_id).contains(&member_id), Error::<T>::Duplicate);
			ensure!((<Pools<T>>::get(&pool_id).len() as u32) < T::MaxMembers::get(), Error::<T>::MemberSizeMax);
			let with_kyc = <PoolMode<T>>::get(&pool_id);
			match Self::check_identity(&member_id, with_kyc) {
				Err(IdentityErr::NoIdentity) => return Err(Error::<T>::NoIdentity.into()),
				Err(IdentityErr::NotEnoughJudjement) => return Err(Error::<T>::NotEnoughJudjement.into()),
				Err(IdentityErr::PoolDuplicates(_)) |
				Err(IdentityErr::Duplicates(_)) => return Err(Error::<T>::IdentityDuplicate.into()),
				Ok(()) => {},
			}

			<Pools<T>>::mutate(pool_id, |v| v.push(member_id.clone()));
			log::debug!(target: LOG_TARGET, "member added by pool");

			return Ok(());
		}

		#[pallet::weight(0)]
		pub fn add_member_self(origin: OriginFor<T>, pool_id: T::AccountId) -> DispatchResult {
			let member_id = ensure_signed(origin)?;
			ensure!(<Pools<T>>::contains_key(&pool_id), Error::<T>::PoolNotExists);
			ensure!(!<Pools<T>>::get(&pool_id).contains(&member_id), Error::<T>::Duplicate);
			ensure!((<Pools<T>>::get(&pool_id).len() as u32) < T::MaxMembers::get(), Error::<T>::MemberSizeMax);
			let with_kyc = <PoolMode<T>>::get(&pool_id);
			match Self::check_identity(&member_id, with_kyc) {
				Err(IdentityErr::NoIdentity) => return Err(Error::<T>::NoIdentity.into()),
				Err(IdentityErr::NotEnoughJudjement) => return Err(Error::<T>::NotEnoughJudjement.into()),
				Err(IdentityErr::PoolDuplicates(_)) |
				Err(IdentityErr::Duplicates(_)) => return Err(Error::<T>::IdentityDuplicate.into()),
				Ok(()) => {},
			}

			<Pools<T>>::mutate(pool_id, |v| v.push(member_id.clone()));
			log::debug!(target: LOG_TARGET, "member added by self");
			Ok(())
		}

		#[pallet::weight(0)]
		pub fn close_pool(
			origin: OriginFor<T>,
		) -> DispatchResult {
			let pool_id = ensure_signed(origin)?;
			ensure!(<Pools<T>>::contains_key(&pool_id), Error::<T>::PoolNotExists);

			<Pools<T>>::remove(pool_id.clone());
			<PoolMode<T>>::remove(pool_id);
			log::debug!(target: LOG_TARGET, "pool removed");

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn remove_member(
			origin: OriginFor<T>,
			member_id: T::AccountId,
		) -> DispatchResult {
			let pool_id = ensure_signed(origin)?;
			ensure!(<Pools<T>>::contains_key(&pool_id), Error::<T>::PoolNotExists);
			ensure!(!<Pools<T>>::get(&pool_id).contains(&member_id), Error::<T>::MemberNotExists);

			<Pools<T>>::mutate(pool_id, |v| v.retain(|m| *m != member_id.clone()));
			Ok(())
		}

		#[pallet::weight(0)]
		pub fn remove_member_self(
			origin: OriginFor<T>,
			pool_id: T::AccountId,
		) -> DispatchResult {
			let member_id = ensure_signed(origin)?;
			ensure!(<Pools<T>>::contains_key(&pool_id), Error::<T>::PoolNotExists);
			ensure!(<Pools<T>>::get(&pool_id).contains(&member_id), Error::<T>::MemberNotExists);

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
			PowStat::<T>::insert(&pool_id, &members);
			log::debug!(target: LOG_TARGET, "submit_mining_stat stat - ok");
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

	fn check_identity(account_id: &T::AccountId, with_kyc: bool) -> Result<(), IdentityErr<T::AccountId>> {
		let ident = Self::get_ident(&account_id);
		let ident = match ident {
			Some(ref id_info) => (3 * id_info.total_judjements >= 2 * id_info.total_judjements)
							.then_some(ident).ok_or(IdentityErr::NotEnoughJudjement),
			None if with_kyc => Err(IdentityErr::NoIdentity),
			None => Ok(None),
		}?;
		let dups = Self::check_duplicates(&account_id, &ident);
		dups.is_empty().then_some(()).ok_or(IdentityErr::Duplicates(dups))
	}

	fn check_pool_identity(pool_id: &T::AccountId) -> Result<(), IdentityErr<T::AccountId>> {
		let mut dups = Vec::new();
		Self::check_identity(pool_id, true)?;
		let pool_ident = Self::get_ident(pool_id);

		for p_id in Pools::<T>::iter_keys() {
			let ident = Self::get_ident(&p_id);
			if ident == pool_ident {
				log::debug!(target: LOG_TARGET, "Found pool duplicate pooi_id={:?}", pool_id);
				dups.push(p_id);
			}
		}
		dups.is_empty().then_some(()).ok_or(IdentityErr::PoolDuplicates(dups))
	}

	fn check_duplicates(
		account_id: &T::AccountId,
		acc_ident: &Option<IdentInfo>,
	) -> Vec<(T::AccountId, T::AccountId)> {
		use sp_std::collections::btree_map::BTreeMap;
		let mut duplicates = Vec::new();

		let mut pools = BTreeMap::new();
		for (pool_id, member_ids) in <Pools<T>>::iter() {
			for member_id in member_ids {
				let ident = Self::get_ident(&member_id);
				pools.insert((pool_id.clone(), member_id), ident.clone());
			}
		}

		for ((pool_id, member_id), ident) in pools {
			if let Some(acc_ident) = acc_ident {
				let ident = Self::get_ident(&member_id);
				match ident {
					Some(ident) =>
						if ident == *acc_ident {
							log::debug!(target: LOG_TARGET, "Found duplicate in pooi_id={:?} member_id={:?}", pool_id, member_id);
							duplicates.push((pool_id, member_id));
						},
					None => {
						if member_id == *account_id {
							log::debug!(target: LOG_TARGET, "Found duplicate by account_id in pooi_id={:?} member_id={:?}", pool_id, member_id);
							duplicates.push((pool_id, member_id));
						}
					}
				}
			}
			else {
				if member_id == *account_id {
					log::debug!(target: LOG_TARGET, "Found duplicate by account_id in pooi_id={:?} member_id={:?}", pool_id, member_id);
					duplicates.push((pool_id, member_id));
				}
			}
		}
		duplicates
	}

	fn get_ident(account_id: &T::AccountId) -> Option<IdentInfo> {
		let reg = pallet_identity::Pallet::<T>::identity(&account_id);

		if let Some(reg) = reg {
			let mut good_judjements = 0;
			let mut total_judjements = 0;
			for (rgstr_idx, judge) in reg.judgements.clone() {
				total_judjements += 1;
				match judge {
					Judgement::Reasonable | Judgement::KnownGood => {
						log::debug!(target: LOG_TARGET, "member is identified by registrar {}", rgstr_idx);
						good_judjements += 1;
					},
					_ => {},
				}
			}

			let email = match reg.info.email {
				Data::Raw(ref email) => String::from_utf8(email.to_vec()).ok(),
				_ => None,
			};
			let twitter = match reg.info.twitter {
				Data::Raw(ref twitter) => String::from_utf8(twitter.to_vec()).ok(),
				_ => None,
			};
			let accs = Self::get_additional_info(&reg.info.clone(), &["Discord", "Telegram"]);
			let (discord, telegram) = (accs[0].clone(), accs[1].clone());
			Some(IdentInfo {good_judjements, total_judjements, email, twitter, discord, telegram})
		}
		else {
			None
		}
	}

	fn get_additional_info<const N: usize>(
		info: &IdentityInfo<T::MaxAdditionalFields>,
		field_names: &[&str; N],
	) -> [Option<String>; N]
	{
		const DFLT: Option<String> = None;
		let mut res: [Option<String>; N] = [DFLT; N];

		for item in info.additional.iter() {
			match item {
				(Data::Raw(k), Data::Raw(v)) => {
					log::debug!(target: LOG_TARGET, "Found additional info: {:?} -> {:?}", k, v);
					match String::from_utf8(k.to_vec()) {
						Ok(key)	=> {
							let index = field_names.iter().position(|&r| r == key);
							if let Some(i) = index {
								if let Ok(acc) = String::from_utf8(v.to_vec()) {
									res[i] = Some(acc);
								} else {
									log::error!(target: LOG_TARGET, "Cant decode additional info: {} -> {:?}", key, v);
								}
							}
						}
						Err(_) => {
							log::error!(target: LOG_TARGET, "Cant decode additional info key: {:?}", k);
						}
					}
				},
				_ => continue,
			}
		}
		res
	}

	fn sync_identity() {
		for pool_id in <Pools<T>>::iter_keys() {
			let with_kyc = PoolMode::<T>::get(&pool_id);
			match Self::check_identity(&pool_id, with_kyc) {
				Err(_) => {
					if !SuspendedPools::<T>::get().contains(&pool_id) {
						SuspendedPools::<T>::mutate(|v| v.push(pool_id.clone()));
						Self::deposit_event(Event::PoolSuspended(pool_id));
					}
				},
				Ok(()) => {
					SuspendedPools::<T>::mutate(|v| v.retain(|p| *p != pool_id));
				},
			}
		}

		let mut duplicates: BTreeMap<T::AccountId, Vec<T::AccountId>> = BTreeMap::new();
		let mut heap = BTreeMap::new();
		for pool_id in <Pools<T>>::iter_keys() {
			let with_kyc = <PoolMode<T>>::get(&pool_id);
			if with_kyc {
				for (pool_id, member_ids) in <Pools<T>>::iter() {
					for member_id in member_ids {
						let ident = Self::get_ident(&member_id);
						match heap.entry(ident) {
							Entry::Vacant(entry) => {
								entry.insert((pool_id.clone(), member_id.clone()));
							},
							Entry::Occupied(val) => {
								log::error!(target: LOG_TARGET, "Duplicated accounts: {:?} {:?}", &pool_id, &member_id);
								duplicates.entry(pool_id.clone()).or_default().push(member_id.clone());
								duplicates.entry(val.get().0.clone()).or_default().push(val.get().1.clone());
							},
						}
					}
				}
			}

			for (pool_id, dups) in duplicates.iter() {
				<Pools<T>>::mutate(pool_id, |v| v.retain(|m| !dups.contains(m)));
			}
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

	/// Return the target difficulty of the next block.
	fn member_status(pool_id: &AccountId, member_id: &AccountId) -> Result<(), CheckMemberError> {
		Ok(())
	}

	fn get_stat(pool_id: &T::AccountId) -> Option<(Percent, Percent, Vec<(T::AccountId, u32)>)> {
		if !<Pools<T>>::contains_key(&pool_id) {
			return None;
		}

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