#![cfg_attr(not(feature = "std"), no_std)]

//! # TC Candidates Pallet
//!
//! Only accounts that have been the proposer of an **enacted** runtime upgrade
//! (set_code) referendum can be added to the Technical Committee. This pallet
//! maintains the list of eligible candidates (`EnactedSetCodeProposers`) and
//! allows users to join it by submitting a **state proof** that at a past block
//! they were the proposer of a referendum that is now Approved and whose
//! proposal was `System::set_code`.

pub use pallet::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
	use codec::{Decode, Encode};
	use sp_std::convert::TryInto;
	use frame_support::{
		pallet_prelude::*,
		traits::StorageVersion,
	};
	use frame_system::pallet_prelude::*;
	#[cfg(feature = "std")]
	use sp_state_machine::{read_proof_check, StorageProof};
	use sp_runtime::traits::BlakeTwo256;
	use sp_std::vec::Vec;

	/// Current storage version.
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

	/// Referendum index (u32).
	pub type ReferendumIndex = u32;

	/// Trait to decode referendum storage value into (proposer, proposal_hash) for Ongoing variant.
	pub trait DecodeOngoingProposerHash {
		type AccountId: Clone + Eq;
		type Hash: Clone + Eq;
		/// Decode raw storage bytes of ReferendumInfo; return Some(proposer, proposal_hash) if Ongoing.
		fn decode_ongoing(bytes: &[u8]) -> Option<(Self::AccountId, Self::Hash)>;
	}

	/// Trait to check if a referendum index is currently in Approved state.
	pub trait ReferendumApprovedChecker {
		fn is_approved(index: ReferendumIndex) -> bool;
	}

	/// Trait to check if a proposal hash corresponds to System::set_code.
	pub trait CheckSetCodeCall {
		type Hash: Clone;
		fn is_set_code(hash: &Self::Hash) -> bool;
	}

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// Pallet name of the Referenda instance (e.g. "Referenda" or "RankedPolls") for storage key.
		#[pallet::constant]
		type ReferendaPalletName: Get<&'static str>;

		/// Decode referendum storage value to (proposer, proposal_hash) for Ongoing.
		type DecodeOngoing: DecodeOngoingProposerHash<
			AccountId = Self::AccountId,
			Hash = Self::Hash,
		>;

		/// Check if a referendum is currently Approved.
		type ReferendumApproved: ReferendumApprovedChecker;

		/// Check if a proposal hash is System::set_code.
		type SetCodeChecker: CheckSetCodeCall<Hash = Self::Hash>;
	}

	/// Set of accounts that are eligible to be TC members (proposers of enacted set_code referenda).
	/// Only populated by `submit_candidacy` (and optionally genesis).
	#[pallet::storage]
	#[pallet::getter(fn enacted_set_code_proposers)]
	pub type EnactedSetCodeProposers<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, (), OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Account was added to TC candidates (enacted set_code proposers).
		CandidacySubmitted { who: T::AccountId, referendum_index: ReferendumIndex },
		/// Account was added to TC candidates by Root (bypasses proof check).
		CandidacyForceAdded { who: T::AccountId },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// State proof verification failed.
		InvalidProof,
		/// Referendum at the given block was not Ongoing (or decode failed).
		NotOngoing,
		/// Signer is not the proposer of the referendum in the proof.
		NotProposer,
		/// Proposal hash in proof does not match the one provided.
		ProposalHashMismatch,
		/// Referendum is not currently Approved.
		ReferendumNotApproved,
		/// Proposal is not System::set_code.
		NotSetCode,
		/// Already a TC candidate.
		AlreadyCandidate,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Submit TC candidacy by proving that at a past block the signer was the proposer
		/// of a referendum that is now Approved and whose proposal was `System::set_code`.
		///
		/// The caller must provide:
		/// - `state_root`: state root of the block when the referendum was still Ongoing.
		/// - `referendum_index`: the referendum index.
		/// - `proposal_hash`: the proposal hash (must match the one in the proof and be set_code).
		/// - `set_code_proof`: storage read proof for `ReferendumInfoFor(referendum_index)` at that block.
		///
		/// The proof is typically obtained via RPC (e.g. `tc_candidates_getSetCodeProof`).
		#[pallet::call_index(0)]
		#[pallet::weight(10_000)]
		pub fn submit_candidacy(
			origin: OriginFor<T>,
			state_root: T::Hash,
			referendum_index: ReferendumIndex,
			proposal_hash: T::Hash,
			set_code_proof: Vec<u8>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			ensure!(
				!EnactedSetCodeProposers::<T>::contains_key(&who),
				Error::<T>::AlreadyCandidate,
			);

			// Proof verification requires std (read_proof_check in sp_state_machine).
			// On-chain (wasm) we reject; host/rpc tests can verify.
			#[cfg(not(feature = "std"))]
			return Err(Error::<T>::InvalidProof.into());

			#[cfg(feature = "std")]
			{
				let storage_key = Self::referendum_info_storage_key(referendum_index);
				let proof: StorageProof = Decode::decode(&mut set_code_proof.as_slice())
					.map_err(|_| Error::<T>::InvalidProof)?;
				let arr: [u8; 32] = state_root.as_ref().try_into().map_err(|_| Error::<T>::InvalidProof)?;
				let root = sp_core::H256::from(arr);
				let verified = read_proof_check::<BlakeTwo256, _>(
					root,
					proof,
					sp_std::iter::once(storage_key.as_slice()),
				)
				.map_err(|_| Error::<T>::InvalidProof)?;
				let value = verified
					.get(storage_key.as_slice())
					.and_then(|v| v.as_ref())
					.ok_or(Error::<T>::InvalidProof)?;
				let (proposer, hash_in_proof) =
					T::DecodeOngoing::decode_ongoing(value).ok_or(Error::<T>::NotOngoing)?;

				ensure!(proposer == who, Error::<T>::NotProposer);
				ensure!(hash_in_proof == proposal_hash, Error::<T>::ProposalHashMismatch);
				ensure!(
					T::ReferendumApproved::is_approved(referendum_index),
					Error::<T>::ReferendumNotApproved,
				);
				ensure!(
					T::SetCodeChecker::is_set_code(&proposal_hash),
					Error::<T>::NotSetCode,
				);

				EnactedSetCodeProposers::<T>::insert(&who, ());
				Self::deposit_event(Event::CandidacySubmitted {
					who: who.clone(),
					referendum_index,
				});
			}

			Ok(())
		}

		/// Add an account to the TC candidate list without proof (Root only).
		///
		/// Bypasses the enacted set_code referendum check. Use for bootstrapping or recovery.
		#[pallet::call_index(1)]
		#[pallet::weight(10_000)]
		pub fn add_candidate_force(origin: OriginFor<T>, who: T::AccountId) -> DispatchResult {
			ensure_root(origin)?;

			ensure!(
				!EnactedSetCodeProposers::<T>::contains_key(&who),
				Error::<T>::AlreadyCandidate,
			);

			EnactedSetCodeProposers::<T>::insert(&who, ());
			Self::deposit_event(Event::CandidacyForceAdded { who: who.clone() });

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Build the storage key for `ReferendumInfoFor(referendum_index)`.
		/// Key = twox_128(pallet_name) ++ twox_128("ReferendumInfoFor") ++ blake2_128_concat(index).
		pub fn referendum_info_storage_key(index: ReferendumIndex) -> Vec<u8> {
			let pallet_hash = sp_io::hashing::twox_128(T::ReferendaPalletName::get().as_bytes());
			let storage_hash = sp_io::hashing::twox_128(b"ReferendumInfoFor");
			let index_encoded = index.encode();
			let index_hash = sp_io::hashing::blake2_128(&index_encoded);
			let mut key = Vec::with_capacity(pallet_hash.len() + storage_hash.len() + index_hash.len() + index_encoded.len());
			key.extend_from_slice(&pallet_hash);
			key.extend_from_slice(&storage_hash);
			key.extend_from_slice(&index_hash);
			key.extend_from_slice(&index_encoded);
			key
		}

		/// Check if an account is in the TC candidate list (eligible for TC membership).
		pub fn is_candidate(account: &T::AccountId) -> bool {
			EnactedSetCodeProposers::<T>::contains_key(account)
		}
	}
}
