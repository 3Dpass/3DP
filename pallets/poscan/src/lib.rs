#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/v3/runtime/frame>
pub use pallet::*;
extern crate alloc;
use core::num::ParseIntError;
use sp_std::vec::Vec;
use sp_std::collections::vec_deque::VecDeque;
use spin::Mutex;

#[macro_use]
extern crate lazy_static;


#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub struct MiningProposal {
	pub a: i32,
	pub pre_obj: Vec<u8>,
}

lazy_static! {
    pub static ref DEQUE: Mutex<VecDeque<MiningProposal>> = {
        let m = VecDeque::new();
        Mutex::new(m)
    };
}


#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

fn decode_hex(buf: &[u8]) -> Result<Vec<u8>, ParseIntError> {
	let s = core::str::from_utf8(buf).unwrap();
	(0..s.len())
		.step_by(2)
		.map(|i| u8::from_str_radix(&s[i..i + 2], 16))
		.collect()
}

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
	use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*};
	use sp_std::vec::Vec;
	use frame_support::sp_runtime::print as prn;
	// use frame_support::runtime_print;
	use crate::{DEQUE, MiningProposal};
	use super::decode_hex;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	// The pallet's runtime storage items.
	#[pallet::storage]
	pub(super) type Proofs<T: Config> = StorageMap<_, Blake2_128Concat, Vec<u8>, (T::AccountId, T::BlockNumber), ValueQuery>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event emitted when a proof has been claimed. [who, claim]
		ClaimCreated(T::AccountId, Vec<u8>),
		/// Event emitted when a claim is revoked by the owner. [who, claim]
		ClaimRevoked(T::AccountId, Vec<u8>),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// The proof has already been claimed.
		ProofAlreadyClaimed,
		/// The proof does not exist, so it cannot be revoked.
		NoSuchProof,
		/// The proof is claimed by another account, so caller can't revoke it.
		NotProofOwner,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(1_000_000_000)]
		pub fn put_mining_object(
			origin: OriginFor<T>,
			proof: Vec<u8>,
		) -> DispatchResultWithPostInfo {
			// Check that the extrinsic was signed and get the signer.
			// This function will return an error if the extrinsic is not signed.
			// https://docs.substrate.io/v3/runtime/origins
			// let sender = ensure_signed(origin)?;

			// Verify that the specified proof has not already been claimed.
			// ensure!(!Proofs::<T>::contains_key(&proof), Error::<T>::ProofAlreadyClaimed);

			// Get the block number from the FRAME System pallet.
			// let _current_block = <frame_system::Pallet<T>>::block_number();

			let content = decode_hex(&proof).unwrap();

			// let mut buf: Vec<u8>;
			// let res = p3d::p3d_process(&content, p3d::AlgoType::Grid2d, 6i16, 2i16 );
			// match res {
			// 	Ok(v) => {
			// 		for i in 0..v.len()-1 {
			// 			prn(v[i].as_str());
			// 		}
			// 		buf = v.concat().as_bytes().to_vec();
			// 	},
			// 	Err(_) => {
			// 		runtime_print!(">>> Error");
			// 		return Err(DispatchError::Other(&"Error in p3d"));
			// 	},
			// }
			//
			// buf.extend(&proof);
			// Store the proof with the sender and block number.
			// Proofs::<T>::insert(&buf, (&sender, current_block, ));

			prn("put_minig_obj recieved object");

			let mut lock = DEQUE.lock();
			(*lock).push_back(MiningProposal {a: 1, pre_obj: content});


			// Emit an event that the claim was created.
			// Self::deposit_event(Event::ClaimCreated(sender, proof));

			Ok(().into())
		}

		#[pallet::weight(10_000)]
		pub fn revoke_claim(
			origin: OriginFor<T>,
			proof: Vec<u8>,
		) -> DispatchResultWithPostInfo {
			// Check that the extrinsic was signed and get the signer.
			// This function will return an error if the extrinsic is not signed.
			// https://docs.substrate.io/v3/runtime/origins
			let sender = ensure_signed(origin)?;

			// Verify that the specified proof has been claimed.
			ensure!(Proofs::<T>::contains_key(&proof), Error::<T>::NoSuchProof);

			// Get owner of the claim.
			let (owner, _) = Proofs::<T>::get(&proof);

			// Verify that sender of the current call is the claim owner.
			ensure!(sender == owner, Error::<T>::NotProofOwner);

			// Remove claim from storage.
			Proofs::<T>::remove(&proof);

			// Emit an event that the claim was erased.
			Self::deposit_event(Event::ClaimRevoked(sender, proof));
			Ok(().into())
		}
	}
}
