#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod mock;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{dispatch::DispatchResult, pallet_prelude::*, BoundedVec};
    use frame_system::pallet_prelude::*;
    use sp_std::vec::Vec;
    use sp_io::hashing::blake2_128;
    use frame_support::traits::ConstU32;

    #[pallet::config]
    pub trait Config: frame_system::Config + scale_info::TypeInfo {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        
        /// Maximum number of serial numbers that can be generated per block per owner
        #[pallet::constant]
        type MaxSerialNumbersPerBlock: Get<u32>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn sn_count)]
    pub(super) type SNCount<T: Config> = StorageValue<_, u64, ValueQuery>;

    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub struct SerialNumberDetails<T: Config>
    where
        T: scale_info::TypeInfo,
    {
        pub sn_index: u64,
        pub sn_hash: [u8; 16], // blake2_128 hash
        pub owner: T::AccountId,
        pub created: T::BlockNumber,
        pub block_index: u32, // index within the block (0..MaxSerialNumbersPerBlock)
        pub is_expired: bool,
        pub expired: Option<T::BlockNumber>,
    }

    #[pallet::storage]
    #[pallet::getter(fn serial_numbers)]
    pub(super) type SerialNumbers<T: Config> = StorageMap<_, Blake2_128Concat, u64, SerialNumberDetails<T>>;

    #[pallet::storage]
    #[pallet::getter(fn sn_by_hash)]
    pub(super) type SNByHash<T: Config> = StorageMap<_, Blake2_128Concat, [u8; 16], u64>;

    #[pallet::storage]
    #[pallet::getter(fn sn_owners)]
    pub(super) type SNOwners<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, BoundedVec<u64, ConstU32<100>>, ValueQuery>;

    /// Track which serial numbers have been used/redeemed to prevent double-spending
    #[pallet::storage]
    #[pallet::getter(fn used_serial_numbers)]
    pub(super) type UsedSerialNumbers<T: Config> = StorageMap<_, Blake2_128Concat, [u8; 16], bool, ValueQuery>;

    /// Track how many serial numbers each owner has generated per block to enforce limits
    #[pallet::storage]
    pub(super) type OwnerBlockCount<T: Config> = StorageDoubleMap<_, Blake2_128Concat, T::AccountId, Blake2_128Concat, T::BlockNumber, u32, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        SerialNumberCreated(u64, [u8; 16], T::AccountId, u32), // sn_index, sn_hash, owner, block_index
        SerialNumberExpired(u64, [u8; 16]),
        SerialNumberUsed([u8; 16], T::AccountId), // sn_hash, user
    }

    #[pallet::error]
    pub enum Error<T> {
        SerialNumberNotFound,
        SerialNumberExpired,
        SerialNumberAlreadyUsed,
        NotOwner,
        TooManySerialNumbersPerBlock,
        InvalidBlockIndex,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(10_000)]
        pub fn create_serial_number(origin: OriginFor<T>, block_index: u32) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let current_block = <frame_system::Pallet<T>>::block_number();
            // Check if owner has already generated too many SNs this block
            let current_count = OwnerBlockCount::<T>::get(&who, current_block);
            ensure!(current_count < T::MaxSerialNumbersPerBlock::get(), Error::<T>::TooManySerialNumbersPerBlock);
            // Check if block_index is within allowed range
            ensure!(block_index < T::MaxSerialNumbersPerBlock::get(), Error::<T>::InvalidBlockIndex);
            let block_hash = <frame_system::Pallet<T>>::block_hash(current_block);
            // Generate deterministic serial number
            let sn_hash = Self::generate_serial_number(&who, &block_hash, block_index);
            // Check if this serial number already exists (shouldn't happen with proper indexing)
            ensure!(!SNByHash::<T>::contains_key(sn_hash), Error::<T>::SerialNumberNotFound);
            let sn_index = SNCount::<T>::get();
            // Store serial number details
            let details = SerialNumberDetails::<T> {
                sn_index,
                sn_hash,
                owner: who.clone(),
                created: current_block,
                block_index,
                is_expired: false,
                expired: None,
            };
            SerialNumbers::<T>::insert(sn_index, &details);
            SNByHash::<T>::insert(sn_hash, sn_index);
            SNOwners::<T>::mutate(&who, |ids| ids.try_push(sn_index).ok());
            OwnerBlockCount::<T>::mutate(&who, current_block, |count| *count += 1);
            SNCount::<T>::put(sn_index + 1);
            Self::deposit_event(Event::SerialNumberCreated(sn_index, sn_hash, who, block_index));
            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(10_000)]
        pub fn turn_sn_expired(origin: OriginFor<T>, sn_index: u64) -> DispatchResult {
            let who = ensure_signed(origin)?;
            SerialNumbers::<T>::try_mutate(sn_index, |maybe_details| {
                let details = maybe_details.as_mut().ok_or(Error::<T>::SerialNumberNotFound)?;
                ensure!(details.owner == who, Error::<T>::NotOwner);
                ensure!(!details.is_expired, Error::<T>::SerialNumberExpired);
                details.is_expired = true;
                details.expired = Some(<frame_system::Pallet<T>>::block_number());
                Self::deposit_event(Event::SerialNumberExpired(sn_index, details.sn_hash));
                Ok(())
            })
        }

        /// Mark a serial number as used (e.g., for redemption)
        #[pallet::call_index(2)]
        #[pallet::weight(10_000)]
        pub fn use_serial_number(origin: OriginFor<T>, sn_hash: [u8; 16]) -> DispatchResult {
            let who = ensure_signed(origin)?;
            // Check if already used (do this first)
            ensure!(!UsedSerialNumbers::<T>::get(sn_hash), Error::<T>::SerialNumberAlreadyUsed);
            // Check if serial number exists and is valid
            ensure!(Self::verify_serial_number(sn_hash, <frame_system::Pallet<T>>::block_number()), Error::<T>::SerialNumberNotFound);
            // Mark as used
            UsedSerialNumbers::<T>::insert(sn_hash, true);
            Self::deposit_event(Event::SerialNumberUsed(sn_hash, who));
            Ok(())
        }
    }

    impl<T: Config + scale_info::TypeInfo> Pallet<T> {
        /// Generate a deterministic serial number
        pub fn generate_serial_number(owner: &T::AccountId, block_hash: &T::Hash, block_index: u32) -> [u8; 16] {
            let mut input = owner.encode();
            input.extend(block_hash.encode());
            input.extend(block_index.to_le_bytes());
            blake2_128(&input)
        }

        /// Verify a serial number is valid for the given block
        pub fn verify_serial_number(sn_hash: [u8; 16], block: T::BlockNumber) -> bool {
            if let Some(sn_index) = SNByHash::<T>::get(sn_hash) {
                if let Some(details) = SerialNumbers::<T>::get(sn_index) {
                    if details.is_expired {
                        return false;
                    }
                    
                    // Check if already used
                    if UsedSerialNumbers::<T>::get(sn_hash) {
                        return false;
                    }
                    
                    // Recompute the hash to verify
                    let block_hash = <frame_system::Pallet<T>>::block_hash(block);
                    let expected_hash = Self::generate_serial_number(&details.owner, &block_hash, details.block_index);
                    return expected_hash == sn_hash;
                }
            }
            false
        }

        /// Verify a serial number without storing it (stateless verification)
        pub fn verify_serial_number_stateless(
            sn_hash: [u8; 16], 
            owner: &T::AccountId, 
            block: T::BlockNumber, 
            block_index: u32
        ) -> bool {
            let block_hash = <frame_system::Pallet<T>>::block_hash(block);
            let expected_hash = Self::generate_serial_number(owner, &block_hash, block_index);
            expected_hash == sn_hash
        }

        /// Generate multiple serial numbers for a block (off-chain helper)
        pub fn generate_serial_numbers_for_block(
            owner: &T::AccountId, 
            block: T::BlockNumber, 
            count: u32
        ) -> Vec<[u8; 16]> {
            let block_hash = <frame_system::Pallet<T>>::block_hash(block);
            let max_count = T::MaxSerialNumbersPerBlock::get();
            let actual_count = count.min(max_count);
            
            (0..actual_count)
                .map(|i| Self::generate_serial_number(owner, &block_hash, i))
                .collect()
        }

        pub fn get_serial_numbers(sn_index: Option<u64>) -> Vec<SerialNumberDetails<T>> {
            match sn_index {
                Some(idx) => SerialNumbers::<T>::get(idx).into_iter().collect(),
                None => SerialNumbers::<T>::iter().map(|(_, v)| v).collect(),
            }
        }

        pub fn get_sn_owners(owner: T::AccountId) -> Vec<u64> {
            SNOwners::<T>::get(owner).into()
        }

        /// Check if a serial number has been used
        pub fn is_serial_number_used(sn_hash: [u8; 16]) -> bool {
            UsedSerialNumbers::<T>::get(sn_hash)
        }
    }
}
