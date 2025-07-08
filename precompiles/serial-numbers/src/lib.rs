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

// Event selectors for Solidity event emission
pub const SELECTOR_LOG_SERIAL_NUMBER_CREATED: [u8; 32] = keccak256!("SerialNumberCreated(address,uint64,bytes16,uint32)");
pub const SELECTOR_LOG_SERIAL_NUMBER_USED: [u8; 32] = keccak256!("SerialNumberUsed(bytes16,address)");
pub const SELECTOR_LOG_SERIAL_NUMBER_EXPIRED: [u8; 32] = keccak256!("SerialNumberExpired(uint64,bytes16)");

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
        bool, u64, H256, Address, U256, u32, bool, U256
    )> {
        if let Some(details) = pallet_serial_numbers::Pallet::<Runtime>::serial_numbers(sn_index) {
            // Pad [u8; 16] to [u8; 32] for H256
            let mut padded = [0u8; 32];
            padded[..16].copy_from_slice(&details.sn_hash);
            let sn_hash_h256 = H256::from(padded);
            let owner = account_id_to_address::<Runtime>(&details.owner);
            let created = U256::from(details.created.saturated_into::<u64>());
            let block_index = details.block_index;
            let is_expired = details.is_expired;
            let expired = U256::from(details.expired.map(|b| b.saturated_into::<u64>()).unwrap_or(0));
            Ok((true, sn_index, sn_hash_h256, owner, created, block_index, is_expired, expired))
        } else {
            Ok((false, 0, H256::zero(), Address(H160::zero()), U256::zero(), 0, false, U256::zero()))
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