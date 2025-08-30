#![cfg(test)]

use super::*;
use frame_support::{assert_ok, assert_noop};
use mock::{new_test_ext, SerialNumbers, Origin, Test};

#[test]
fn create_and_verify_serial_number_works() {
    new_test_ext().execute_with(|| {
        // Create SN with block_index 0
        assert_ok!(SerialNumbers::create_serial_number(Origin::signed(1), 0));
        let sn_count = SerialNumbers::sn_count();
        assert_eq!(sn_count, 1);
        let details = SerialNumbers::serial_numbers(0).unwrap();
        // Verify SN
        let valid = SerialNumbers::verify_serial_number(details.sn_hash, details.created);
        assert!(valid);
    });
}

#[test]
fn create_multiple_serial_numbers_per_block() {
    new_test_ext().execute_with(|| {
        // Create multiple SNs in the same block
        assert_ok!(SerialNumbers::create_serial_number(Origin::signed(1), 0));
        assert_ok!(SerialNumbers::create_serial_number(Origin::signed(1), 1));
        assert_ok!(SerialNumbers::create_serial_number(Origin::signed(1), 2));
        
        let sn_count = SerialNumbers::sn_count();
        assert_eq!(sn_count, 3);
        
        // Verify all SNs are unique
        let sns: Vec<_> = (0..3).map(|i| SerialNumbers::serial_numbers(i).unwrap().sn_hash).collect();
        assert_eq!(sns.len(), 3);
        assert_ne!(sns[0], sns[1]);
        assert_ne!(sns[1], sns[2]);
        assert_ne!(sns[0], sns[2]);
    });
}

#[test]
fn stateless_verification_works() {
    new_test_ext().execute_with(|| {
        let owner = 1;
        let block = 1;
        
        // Generate SNs off-chain
        let sns = SerialNumbers::generate_serial_numbers_for_block(&owner, block, 5);
        assert_eq!(sns.len(), 5);
        
        // Verify each SN statelessly
        for (i, sn_hash) in sns.iter().enumerate() {
            let valid = SerialNumbers::verify_serial_number_stateless(*sn_hash, &owner, block, i as u32);
            assert!(valid);
        }
    });
}

#[test]
fn turn_sn_expired_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(SerialNumbers::create_serial_number(Origin::signed(1), 0));
        assert_ok!(SerialNumbers::turn_sn_expired(Origin::signed(1), 0));
        let details = SerialNumbers::serial_numbers(0).unwrap();
        assert!(details.is_expired);
        // Verify SN is now invalid
        let valid = SerialNumbers::verify_serial_number(details.sn_hash, details.created);
        assert!(!valid);
    });
}

#[test]
fn only_owner_can_expire() {
    new_test_ext().execute_with(|| {
        assert_ok!(SerialNumbers::create_serial_number(Origin::signed(1), 0));
        assert_noop!(SerialNumbers::turn_sn_expired(Origin::signed(2), 0), pallet::Error::<Test>::NotOwner);
    });
}

#[test]
fn use_serial_number_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(SerialNumbers::create_serial_number(Origin::signed(1), 0));
        let details = SerialNumbers::serial_numbers(0).unwrap();
        
        // Use the serial number (by the owner)
        assert_ok!(SerialNumbers::use_serial_number(Origin::signed(1), details.sn_hash));
        
        // Check it's marked as used
        assert!(SerialNumbers::is_serial_number_used(details.sn_hash));
        
        // Verify it's no longer valid
        let valid = SerialNumbers::verify_serial_number(details.sn_hash, details.created);
        assert!(!valid);
    });
}

#[test]
fn cannot_use_serial_number_twice() {
    new_test_ext().execute_with(|| {
        assert_ok!(SerialNumbers::create_serial_number(Origin::signed(1), 0));
        let details = SerialNumbers::serial_numbers(0).unwrap();
        
        // Use the serial number first time (by the owner)
        assert_ok!(SerialNumbers::use_serial_number(Origin::signed(1), details.sn_hash));
        
        // Try to use it again (by the owner)
        assert_noop!(
            SerialNumbers::use_serial_number(Origin::signed(1), details.sn_hash),
            pallet::Error::<Test>::SerialNumberAlreadyUsed
        );
    });
}

#[test]
fn cannot_exceed_max_serial_numbers_per_block() {
    new_test_ext().execute_with(|| {
        // Create 10 SNs (the max allowed)
        for i in 0..10 {
            assert_ok!(SerialNumbers::create_serial_number(Origin::signed(1), i));
        }
        
        // Try to create one more
        assert_noop!(
            SerialNumbers::create_serial_number(Origin::signed(1), 10),
            pallet::Error::<Test>::TooManySerialNumbersPerBlock
        );
    });
}

#[test]
fn invalid_block_index_rejected() {
    new_test_ext().execute_with(|| {
        // Try to create SN with invalid block_index
        assert_noop!(
            SerialNumbers::create_serial_number(Origin::signed(1), 10),
            pallet::Error::<Test>::InvalidBlockIndex
        );
    });
}

#[test]
fn get_serial_numbers_and_owners_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(SerialNumbers::create_serial_number(Origin::signed(1), 0));
        assert_ok!(SerialNumbers::create_serial_number(Origin::signed(2), 0));
        let all = SerialNumbers::get_serial_numbers(None);
        assert_eq!(all.len(), 2);
        let sn0 = SerialNumbers::get_serial_numbers(Some(0));
        assert_eq!(sn0.len(), 1);
        let owner1 = SerialNumbers::get_sn_owners(1);
        assert_eq!(owner1, vec![0]);
        let owner2 = SerialNumbers::get_sn_owners(2);
        assert_eq!(owner2, vec![1]);
    });
} 

#[test]
fn transfer_ownership_works() {
    new_test_ext().execute_with(|| {
        // Create SN with block_index 0 by account 1
        assert_ok!(SerialNumbers::create_serial_number(Origin::signed(1), 0));
        let details = SerialNumbers::serial_numbers(0).unwrap();
        assert_eq!(details.initial_owner, 1);
        assert_eq!(details.owner, 1);

        // Transfer to account 2
        assert_ok!(SerialNumbers::transfer_ownership(Origin::signed(1), 0, 2));
        let details = SerialNumbers::serial_numbers(0).unwrap();
        assert_eq!(details.initial_owner, 1); // initial_owner never changes
        assert_eq!(details.owner, 2);

        // Old owner cannot use or expire
        assert_noop!(SerialNumbers::use_serial_number(Origin::signed(1), details.sn_hash), pallet::Error::<Test>::NotOwner);
        assert_noop!(SerialNumbers::turn_sn_expired(Origin::signed(1), 0), pallet::Error::<Test>::NotOwner);
        // Old owner cannot transfer again
        assert_noop!(SerialNumbers::transfer_ownership(Origin::signed(1), 0, 3), pallet::Error::<Test>::NotOwner);

        // New owner can use and expire
        assert_ok!(SerialNumbers::use_serial_number(Origin::signed(2), details.sn_hash));
        // Mark as expired (should succeed even if already used)
        assert_ok!(SerialNumbers::turn_sn_expired(Origin::signed(2), 0));
    });
} 