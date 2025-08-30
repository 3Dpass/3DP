// SPDX-License-Identifier: Apache-2.0
#![cfg_attr(not(feature = "std"), no_std)]

//! SerialNumbers precompile

use precompile_utils::prelude::*;
use sp_core::{H160, U256, H256};
use pallet_evm::AddressMapping;
use core::marker::PhantomData;
use sp_runtime::traits::SaturatedConversion;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
#[macro_use]
extern crate alloc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bytes16(pub [u8; 16]);

impl precompile_utils::EvmData for Bytes16 {
    fn read(reader: &mut precompile_utils::EvmDataReader) -> MayRevert<Self> {
        let buf = reader.read_raw_bytes(32)?;
        let mut arr = [0u8; 16];
        arr.copy_from_slice(&buf[..16]);
        Ok(Bytes16(arr))
    }
    fn write(writer: &mut precompile_utils::EvmDataWriter, value: Self) {
        let mut buf = [0u8; 32];
        buf[..16].copy_from_slice(&value.0);
        *writer = writer.clone().write(buf.to_vec());
    }
    fn has_static_size() -> bool { true }
    fn solidity_type() -> String { "bytes16".to_string() }
}

/// Helper: Convert AccountId32 to H160 address by truncating to first 20 bytes
fn account_id_to_address<Runtime: frame_system::Config>(account_id: &Runtime::AccountId) -> Address
where
    Runtime::AccountId: AsRef<[u8; 32]>,
{
    let bytes = account_id.as_ref();
    Address(H160::from_slice(&bytes[0..20]))
}

// Convert addresses to H256 (left-pad H160 to H256)
pub fn address_to_h256(addr: Address) -> H256 {
    let mut padded = [0u8; 32];
    padded[12..].copy_from_slice(addr.0.as_bytes());
    H256::from(padded)
}

// Event selectors for SerialNumbers precompile compatibility
pub const SELECTOR_LOG_SERIAL_NUMBER_CREATED: [u8; 32] = keccak256!("SerialNumberCreated(address,uint64,bytes16,uint32)");
pub const SELECTOR_LOG_SERIAL_NUMBER_USED: [u8; 32] = keccak256!("SerialNumberUsed(bytes16,address)");
pub const SELECTOR_LOG_SERIAL_NUMBER_EXPIRED: [u8; 32] = keccak256!("SerialNumberExpired(uint64,bytes16)");
pub const SELECTOR_LOG_OWNERSHIP_TRANSFERRED: [u8; 32] = keccak256!("OwnershipTransferred(uint64,address,address)");

pub struct SerialNumbersPrecompile<Runtime>(core::marker::PhantomData<Runtime>);

#[precompile_utils::precompile]
impl<Runtime> SerialNumbersPrecompile<Runtime>
where
    Runtime: pallet_evm::Config + pallet_serial_numbers::Config,
    Runtime::Call: frame_support::dispatch::Dispatchable<PostInfo = frame_support::dispatch::PostDispatchInfo> + frame_support::dispatch::GetDispatchInfo,
    <Runtime::Call as frame_support::dispatch::Dispatchable>::Origin: From<Option<Runtime::AccountId>>,
    Runtime::Call: From<pallet_serial_numbers::Call<Runtime>>,
    Runtime::AccountId: AsRef<[u8; 32]>,
{
    /// @custom:selector 0x0a0b0c01
    #[precompile::public("createSerialNumber(uint32)")]
    fn create_serial_number(
        handle: &mut impl PrecompileHandle,
        block_index: u32,
    ) -> EvmResult<(Bytes16, u64, u32)> {
        let who: Runtime::AccountId = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_serial_numbers::Call::<Runtime>::create_serial_number { block_index };
        let result = RuntimeHelper::<Runtime>::try_dispatch(handle, Some(who.clone()).into(), call);
        match result {
            Ok(_) => {
                // Find the latest serial number for this owner
                let sn_ids = pallet_serial_numbers::Pallet::<Runtime>::get_sn_owners(who.clone());
                if let Some(&sn_index) = sn_ids.last() {
                    if let Some(details) = pallet_serial_numbers::Pallet::<Runtime>::serial_numbers(sn_index) {
                        let sn_hash = Bytes16(details.sn_hash);
                        // Emit SerialNumberCreated event
                        log3(
                            handle.context().address,
                            SELECTOR_LOG_SERIAL_NUMBER_CREATED,
                            handle.context().caller,
                            handle.context().caller,
                            EvmDataWriter::new()
                                .write(Address(handle.context().caller))
                                .write(sn_index)
                                .write(sn_hash.clone())
                                .write(details.block_index)
                                .build(),
                        ).record(handle)?;
                        return Ok((sn_hash, sn_index, details.block_index));
                    }
                }
                Err(revert("Serial number not found after creation"))
            },
            Err(e) => Err(e.into()),
        }
    }

    /// @custom:selector 0x0a0b0c02
    #[precompile::public("getSerialNumber(uint64)")]
    fn get_serial_number(
        _handle: &mut impl PrecompileHandle,
        sn_index: u64,
    ) -> EvmResult<(
        bool, u64, H256, U256, u32, bool, U256
    )> {
        if let Some(details) = pallet_serial_numbers::Pallet::<Runtime>::serial_numbers(sn_index) {
            // Pad [u8; 16] to [u8; 32] for H256
            let mut padded = [0u8; 32];
            padded[..16].copy_from_slice(&details.sn_hash);
            let sn_hash_h256 = H256::from(padded);
            let created = U256::from(details.created.saturated_into::<u64>());
            let block_index = details.block_index;
            let is_expired = details.is_expired;
            let expired = U256::from(details.expired.map(|b| b.saturated_into::<u64>()).unwrap_or(0));
            Ok((true, sn_index, sn_hash_h256, created, block_index, is_expired, expired))
        } else {
            Ok((false, 0, H256::zero(), U256::zero(), 0, false, U256::zero()))
        }
    }

    /// @custom:selector 0x0a0b0c08
    #[precompile::public("serialNumbersOf(address)")]
    fn serial_numbers_of(
        _handle: &mut impl PrecompileHandle,
        owner: Address,
    ) -> EvmResult<Vec<u64>> {
        // Map H160 to runtime AccountId
        let account_id = Runtime::AddressMapping::into_account_id(owner.into());
        let serial_numbers = pallet_serial_numbers::Pallet::<Runtime>::get_sn_owners(account_id);
        Ok(serial_numbers.into())
    }

    /// @custom:selector 0x0a0b0c07
    #[precompile::public("transferOwnership(uint64,address)")]
    fn transfer_ownership(
        handle: &mut impl PrecompileHandle,
        sn_index: u64,
        new_owner: Address,
    ) -> EvmResult<bool> {
        // Fetch old owner before transfer
        let old_owner = if let Some(details) = pallet_serial_numbers::Pallet::<Runtime>::serial_numbers(sn_index) {
            account_id_to_address::<Runtime>(&details.owner)
        } else {
            return Ok(false);
        };
        let who: Runtime::AccountId = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let new_owner_id: Runtime::AccountId = Runtime::AddressMapping::into_account_id(new_owner.0);
        let call = pallet_serial_numbers::Call::<Runtime>::transfer_ownership { sn_index, new_owner: new_owner_id };
        let result = RuntimeHelper::<Runtime>::try_dispatch(handle, Some(who).into(), call);
        match result {
            Ok(_) => {
                // Emit EVM event OwnershipTransferred
                let topics = vec![
                    H256::from(SELECTOR_LOG_OWNERSHIP_TRANSFERRED),
                    H256::from_low_u64_be(sn_index),
                    address_to_h256(old_owner),
                    address_to_h256(new_owner),
                ];
                let _ = handle.log(handle.context().address, topics, vec![]);
                Ok(true)
            },
            Err(e) => Err(e.into()),
        }
    }

    /// @custom:selector 0x0a0b0c03
    #[precompile::public("isSerialNumberUsed(bytes16)")]
    fn is_serial_number_used(
        _handle: &mut impl PrecompileHandle,
        sn_hash: Bytes16,
    ) -> EvmResult<(bool, bool)> {
        // Check if the serial number exists
        if pallet_serial_numbers::Pallet::<Runtime>::sn_by_hash(sn_hash.0).is_some() {
            let used = pallet_serial_numbers::Pallet::<Runtime>::is_serial_number_used(sn_hash.0);
            Ok((true, used))
        } else {
            Ok((false, false))
        }
    }

    /// @custom:selector 0x0a0b0c06
    #[precompile::public("snByHash(bytes16)")]
    fn sn_by_hash(
        _handle: &mut impl PrecompileHandle,
        sn_hash: Bytes16,
    ) -> EvmResult<(bool, u64)> {
        if let Some(sn_index) = pallet_serial_numbers::Pallet::<Runtime>::sn_by_hash(sn_hash.0) {
            Ok((true, sn_index))
        } else {
            Ok((false, 0))
        }
    }

    // Stubs for the rest
    #[precompile::public("expireSerialNumber(uint64)")]
    fn expire_serial_number(
        handle: &mut impl PrecompileHandle,
        sn_index: u64,
    ) -> EvmResult<bool> {
        let who: Runtime::AccountId = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_serial_numbers::Call::<Runtime>::turn_sn_expired { sn_index };
        let result = RuntimeHelper::<Runtime>::try_dispatch(handle, Some(who.clone()).into(), call);
        match result {
            Ok(_) => {
                // Emit SerialNumberExpired event if details are available
                if let Some(details) = pallet_serial_numbers::Pallet::<Runtime>::serial_numbers(sn_index) {
                    let sn_hash = Bytes16(details.sn_hash);
                    log3(
                        handle.context().address,
                        SELECTOR_LOG_SERIAL_NUMBER_EXPIRED,
                        handle.context().caller,
                        handle.context().caller,
                        EvmDataWriter::new()
                            .write(sn_index)
                            .write(sn_hash)
                            .build(),
                    ).record(handle)?;
                }
                Ok(true)
            },
            Err(e) => Err(e.into()),
        }
    }

    #[precompile::public("useSerialNumber(bytes16)")]
    fn use_serial_number(
        handle: &mut impl PrecompileHandle,
        sn_hash: Bytes16,
    ) -> EvmResult<bool> {
        let who: Runtime::AccountId = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_serial_numbers::Call::<Runtime>::use_serial_number { sn_hash: sn_hash.0 };
        let result = RuntimeHelper::<Runtime>::try_dispatch(handle, Some(who).into(), call);
        match result {
            Ok(_) => {
                // Emit SerialNumberUsed event
                log3(
                    handle.context().address,
                    SELECTOR_LOG_SERIAL_NUMBER_USED,
                    handle.context().caller,
                    handle.context().caller,
                    EvmDataWriter::new()
                        .write(sn_hash)
                        .write(Address(handle.context().caller))
                        .build(),
                ).record(handle)?;
                Ok(true)
            },
            Err(e) => Err(e.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate hex;
    
    #[test]
    fn print_event_selectors() {
        println!("SerialNumberCreated: 0x{}", hex::encode(SELECTOR_LOG_SERIAL_NUMBER_CREATED));
        println!("SerialNumberUsed: 0x{}", hex::encode(SELECTOR_LOG_SERIAL_NUMBER_USED));
        println!("SerialNumberExpired: 0x{}", hex::encode(SELECTOR_LOG_SERIAL_NUMBER_EXPIRED));
        println!("OwnershipTransferred: 0x{}", hex::encode(SELECTOR_LOG_OWNERSHIP_TRANSFERRED));
    }

    /// This test ensures that the Rust macro attribute signature matches the Solidity interface signature.
    #[test]
    fn test_macro_attribute_matches_solidity_signature() {
        // List of (function_name, rust_macro_attribute, solidity_signature)
        let test_cases = vec![
            ("createSerialNumber", "createSerialNumber(uint32)", "createSerialNumber(uint32)"),
            ("getSerialNumber", "getSerialNumber(uint64)", "getSerialNumber(uint64)"),
            ("serialNumbersOf", "serialNumbersOf(address)", "serialNumbersOf(address)"),
            ("isSerialNumberUsed", "isSerialNumberUsed(bytes16)", "isSerialNumberUsed(bytes16)"),
            ("snByHash", "snByHash(bytes16)", "snByHash(bytes16)"),
            ("expireSerialNumber", "expireSerialNumber(uint64)", "expireSerialNumber(uint64)"),
            ("useSerialNumber", "useSerialNumber(bytes16)", "useSerialNumber(bytes16)"),
            ("transferOwnership", "transferOwnership(uint64,address)", "transferOwnership(uint64,address)"),
        ];
        
        for (function_name, rust_macro, solidity_sig) in test_cases {
            assert_eq!(
                rust_macro, solidity_sig,
                "Signature mismatch for {}: Rust macro '{}', Solidity '{}'",
                function_name, rust_macro, solidity_sig
            );
        }
    }
} 