//! Tests for TC candidates pallet.

#[cfg(test)]
mod tests {
	use crate::mock::*;
	use crate::pallet::*;
	use frame_support::{assert_noop, dispatch::DispatchError};
	use frame_system::RawOrigin;
	use sp_core::H256;
	use sp_io::hashing::twox_128;

	#[test]
	fn is_candidate_returns_false_when_empty() {
		new_test_ext().execute_with(|| {
			assert!(!TcCandidates::is_candidate(&1));
			assert!(!TcCandidates::is_candidate(&2));
		});
	}

	#[test]
	fn referendum_info_storage_key_format() {
		new_test_ext().execute_with(|| {
			let key = TcCandidates::referendum_info_storage_key(0);
			let referenda_prefix = twox_128(b"Referenda");
			assert!(key.len() >= referenda_prefix.len());
			assert_eq!(&key[..referenda_prefix.len()], referenda_prefix);
		});
	}

	#[test]
	fn submit_candidacy_unsigned_fails() {
		new_test_ext().execute_with(|| {
			assert_noop!(
				TcCandidates::submit_candidacy(
					RawOrigin::None.into(),
					H256::zero(),
					0,
					H256::zero(),
					Vec::new(),
				),
				DispatchError::BadOrigin
			);
		});
	}

	#[test]
	fn submit_candidacy_invalid_proof_fails() {
		new_test_ext().execute_with(|| {
			// Invalid proof (not valid SCALE-encoded StorageProof) -> InvalidProof
			assert_noop!(
				TcCandidates::submit_candidacy(
					RawOrigin::Signed(1).into(),
					H256::zero(),
					0,
					H256::zero(),
					vec![0xde, 0xad], // garbage
				),
				Error::<Test>::InvalidProof
			);
		});
	}

	#[cfg(feature = "std")]
	#[test]
	fn submit_candidacy_valid_proof_but_not_ongoing_fails() {
		use codec::Encode;
		use sp_core::storage::StateVersion;
		use sp_runtime::traits::BlakeTwo256;
		use sp_state_machine::{new_in_mem_hash_key, prove_read};

		new_test_ext().execute_with(|| {
			// Build a trie with one key (referendum_info_storage_key(0)) and value [0].
			// DecodeOngoing in mock always returns None, so we get NotOngoing.
			let key = TcCandidates::referendum_info_storage_key(0);
			let value = vec![0u8];
			let mut backend = new_in_mem_hash_key::<BlakeTwo256>();
			backend.insert(
				vec![(None, vec![(key.clone(), Some(value))])],
				StateVersion::default(),
			);
			let state_root = H256::from_slice(backend.root().as_ref());
			let proof = prove_read(backend, [key.as_slice()]).unwrap();
			let proof_bytes = proof.encode();

			assert_noop!(
				TcCandidates::submit_candidacy(
					RawOrigin::Signed(1).into(),
					state_root,
					0,
					H256::zero(),
					proof_bytes,
				),
				Error::<Test>::NotOngoing
			);
		});
	}
}
