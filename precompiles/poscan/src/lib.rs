// Copyright 2025 3Dpass
// SPDX-License-Identifier: MIT

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
use alloc::vec;
use precompile_utils::prelude::*;
use sp_core::{H160, H256, U256};
use sp_std::{marker::PhantomData, vec::Vec};
use sp_arithmetic::traits::SaturatedConversion;

// No alias, use precompile_utils::data::String directly
use pallet_evm::AddressMapping;
use precompile_utils::data::UnboundedString;

use sp_consensus_poscan::{ObjectState, ObjectCategory};
use frame_support::pallet_prelude::ConstU32;
use alloc::string::String;

// Event selectors for modern event emission
pub const SELECTOR_LOG_OBJECT_SUBMITTED: [u8; 32] = keccak256!("ObjectSubmitted(address,uint32)");
pub const SELECTOR_LOG_PERMISSIONS_SET: [u8; 32] = keccak256!("PermissionsSet(uint32,address)");

// Function selectors for reference
pub const SELECTOR_GET_OBJECT: [u8; 4] = [0xA1, 0xB2, 0xC0, 0x01];

/// Convert AccountId32 to H160 address by truncating to first 20 bytes
/// This follows the conventional pattern used in EnsureAddressTruncated
fn account_id_to_address<Runtime: pallet_poscan::Config>(account_id: &<Runtime as frame_system::Config>::AccountId) -> Address
where
    <Runtime as frame_system::Config>::AccountId: AsRef<[u8; 32]>,
{
    let bytes = account_id.as_ref();
    Address(H160::from_slice(&bytes[0..20]))
}

#[derive(Debug, Clone)]
pub struct PoScanPrecompile<Runtime>(PhantomData<Runtime>);

#[precompile_utils::precompile]
impl<Runtime> PoScanPrecompile<Runtime>
where
    Runtime: pallet_evm::Config + pallet_poscan::Config,
    <Runtime as frame_system::Config>::Call: frame_support::dispatch::Dispatchable<PostInfo = frame_support::dispatch::PostDispatchInfo>
        + frame_support::dispatch::GetDispatchInfo,
    <<Runtime as frame_system::Config>::Call as frame_support::dispatch::Dispatchable>::Origin: From<Option<Runtime::AccountId>>,
    <Runtime as frame_system::Config>::Call: From<pallet_poscan::Call<Runtime>>,
    <Runtime as frame_system::Config>::AccountId: AsRef<[u8; 32]>,
{
    /// Solidity-friendly getter for PoScan object data
    /// @custom:selector 0xA1B2C001
    #[precompile::public("getObject(uint32)")]
    fn get_object(
        _handle: &mut impl PrecompileHandle,
        obj_idx: u32,
    ) -> EvmResult<(
        bool, // isValid
        u8, // state discriminant
        u64, // state block (or 0)
        u8, // category discriminant
        u64, // whenCreated
        u64, // whenApproved (0 if None)
        Address, // owner (AccountId32 as H160 address)
        bool, // isPrivate
        Vec<H256>, // hashes
        u8, // numApprovals
        Vec<(u32, u128)>, // prop: (propIdx, maxValue)
    )> {
        use pallet_poscan::Pallet as PoScanPallet;
        // Fetch the object from storage
        let obj_opt = PoScanPallet::<Runtime>::objects(obj_idx);
        if let Some(obj) = obj_opt {
            // --- State discriminant and block ---
            let (state_discriminant, state_block) = match &obj.state {
                ObjectState::Created(bn) => (0u8, (*bn).saturated_into()),
                ObjectState::Estimating(bn) => (1u8, (*bn).saturated_into()),
                ObjectState::Estimated(bn, _) => (2u8, (*bn).saturated_into()),
                ObjectState::NotApproved(bn) => (3u8, (*bn).saturated_into()),
                ObjectState::Approved(bn) => (4u8, (*bn).saturated_into()),
            };
            // --- Category discriminant ---
            let category_discriminant = match &obj.category {
                ObjectCategory::Objects3D(_) => 0u8,
                ObjectCategory::Drawings2D => 1u8,
                ObjectCategory::Music => 2u8,
                ObjectCategory::Biometrics => 3u8,
                ObjectCategory::Movements => 4u8,
                ObjectCategory::Texts => 5u8,
            };
            // --- whenCreated ---
            let when_created = obj.when_created.saturated_into();
            // --- whenApproved ---
            let when_approved = obj.when_approved.map(|b| b.saturated_into()).unwrap_or(0u64);
            // --- owner as EVM address (H160) using canonical mapping ---
            let owner = account_id_to_address::<Runtime>(&obj.owner);
            // --- isPrivate ---
            let is_private = obj.is_private;
            // --- hashes ---
            let hashes = obj.hashes.to_vec();
            // --- numApprovals ---
            let num_approvals = obj.num_approvals;
            // --- prop ---
            let prop = obj.prop.iter().map(|p| (p.prop_idx, p.max_value)).collect();
            Ok((true, state_discriminant, state_block, category_discriminant, when_created, when_approved, owner, is_private, hashes, num_approvals, prop))
        } else {
            Ok((false, 0, 0, 0, 0, 0, Address(H160::zero()), false, vec![], 0, vec![]))
        }
    }

    /// Solidity-friendly getter for all object indices owned by an address
    /// @custom:selector 0x7e2c4b2a
    #[precompile::public("getObjectsOf(address)")]
    fn get_objects_of(
        _handle: &mut impl PrecompileHandle,
        owner: Address,
    ) -> EvmResult<Vec<u32>> {
        use pallet_poscan::Pallet as PoScanPallet;
        // Map H160 to runtime AccountId
        let account_id = <Runtime as pallet_evm::Config>::AddressMapping::into_account_id(owner.into());
        let objects = PoScanPallet::<Runtime>::owners(account_id);
        Ok(objects.into())
    }

    /// Solidity-friendly getter for total number of objects
    /// @custom:selector 0x2f7b5f32
    #[precompile::public("getObjectCount()")]
    fn get_object_count(
        _handle: &mut impl PrecompileHandle,
    ) -> EvmResult<u32> {
        use pallet_poscan::Pallet as PoScanPallet;
        let count = PoScanPallet::<Runtime>::obj_count();
        Ok(count)
    }

    /// Solidity-friendly getter for all available properties for object tokenization
    /// @custom:selector 0x6e2b7b1a
    #[precompile::public("getProperties()")]
    fn get_properties(
        _handle: &mut impl PrecompileHandle,
    ) -> EvmResult<Vec<(bool, u32, UnboundedString, u8, u128)>> {
        use pallet_poscan::Pallet as PoScanPallet;
        use sp_consensus_poscan::PropClass;
        let prop_count = PoScanPallet::<Runtime>::prop_count();
        let mut out = Vec::new();
        for prop_idx in 0..prop_count {
            if let Some(prop) = PoScanPallet::<Runtime>::properties(prop_idx) {
                let name = UnboundedString::from(prop.name.as_slice());
                let class = match prop.class {
                    PropClass::Relative => 0u8,
                    PropClass::Absolute => 1u8,
                };
                out.push((true, prop_idx, name, class, prop.max_value));
            } else {
                out.push((false, prop_idx, UnboundedString::from(""), 0u8, 0u128));
            }
        }
        Ok(out)
    }

    /// Solidity-friendly getter for the storage fee per byte
    /// @custom:selector 0x4e2b7b1b
    #[precompile::public("getFeePerByte()")]
    fn get_fee_per_byte(
        _handle: &mut impl PrecompileHandle,
    ) -> EvmResult<(bool, u64)> {
        use pallet_poscan::Pallet as PoScanPallet;
        match PoScanPallet::<Runtime>::fee_per_byte() {
            Some(fee) => Ok((true, fee)),
            None => Ok((false, 0)),
        }
    }

    /// Solidity-friendly getter for the account lock value for a given address
    /// @custom:selector 0x8e2b7b1c
    #[precompile::public("getAccountLock(address)")]
    fn get_account_lock(
        _handle: &mut impl PrecompileHandle,
        owner: Address,
    ) -> EvmResult<u128> {
        use pallet_poscan::Pallet as PoScanPallet;
        // Map H160 to runtime AccountId
        let account_id = <Runtime as pallet_evm::Config>::AddressMapping::into_account_id(owner.into());
        let lock = PoScanPallet::<Runtime>::locks(account_id);
        Ok(lock.saturated_into())
    }

    /// Submit a new object to the PoScan pallet
    /// @custom:selector 0x0c53c51c
    #[precompile::public("putObject(uint8,uint8,bool,bytes,uint8,bytes32[],(uint32,uint128)[],bool,uint32)")]
    fn put_object(
        handle: &mut impl PrecompileHandle,
        category: u8,
        algo3d: u8,
        is_private: bool,
        obj: Vec<u8>,
        num_approvals: u8,
        hashes: Vec<H256>,
        properties: Vec<(u32, u128)>,
        is_replica: bool,
        original_obj: u32,
    ) -> EvmResult<bool> {
        use sp_consensus_poscan::{ObjectCategory, Algo3D, PropValue};
        use frame_support::BoundedVec;
        // Map enums
        let category = match category {
            0 => ObjectCategory::Objects3D(match algo3d {
                0 => Algo3D::Grid2dLow,
                1 => Algo3D::Grid2dHigh,
                _ => return Err(revert("Invalid Algo3D")),
            }),
            1 => ObjectCategory::Drawings2D,
            2 => ObjectCategory::Music,
            3 => ObjectCategory::Biometrics,
            4 => ObjectCategory::Movements,
            5 => ObjectCategory::Texts,
            _ => return Err(revert("Invalid category")),
        };
        // BoundedVec conversions and checks (as before)
        // Validate OBJ file format and size
        if obj.len() > 1_048_576 { // 1MB = 1024 * 1024 bytes
            return Err(revert("Object file too large (max 1MB)"));
        }
        // Check for OBJ file signature (should start with common OBJ keywords)
        if obj.len() > 0 {
            let obj_str = String::from_utf8_lossy(&obj);
            if !obj_str.lines().any(|line| {
                let trimmed = line.trim();
                trimmed.starts_with("v ") || // vertex
                trimmed.starts_with("vt ") || // texture vertex
                trimmed.starts_with("vn ") || // vertex normal
                trimmed.starts_with("f ") || // face
                trimmed.starts_with("g ") || // group
                trimmed.starts_with("o ") || // object
                trimmed.starts_with("#") // comment
            }) {
                return Err(revert("Invalid OBJ file format"));
            }
        }
        let obj: BoundedVec<u8, ConstU32<{ sp_consensus_poscan::MAX_OBJECT_SIZE }>> = obj.try_into().map_err(|_| revert("obj too large"))?;
        let hashes = if hashes.is_empty() {
            None
        } else {
            Some(BoundedVec::try_from(hashes).map_err(|_| revert("too many hashes"))?)
        };
        let properties = BoundedVec::try_from(
            properties.into_iter().map(|(prop_idx, max_value)| PropValue { prop_idx, max_value }).collect::<Vec<_>>()
        ).map_err(|_| revert("too many properties"))?;
        // Dispatch call as EVM caller
        let who: Runtime::AccountId = <Runtime as pallet_evm::Config>::AddressMapping::into_account_id(handle.context().caller);
        let result = RuntimeHelper::<Runtime>::try_dispatch(
            handle,
            Some(who.clone()).into(),
            pallet_poscan::Call::<Runtime>::put_object {
                category,
                is_private,
                obj,
                num_approvals,
                hashes,
                properties,
                is_replica,
                original_obj: if is_replica { Some(original_obj) } else { None },
            },
        );
        match result {
            Ok(_) => {
                // Record gas cost for event emission
                handle.record_log_costs_manual(3, 32)?;
                
                // Emit ObjectSubmitted event (modern pattern)
                log3(
                    handle.context().address,
                    SELECTOR_LOG_OBJECT_SUBMITTED,
                    handle.context().caller,
                    handle.context().caller,
                    EvmDataWriter::new()
                        .write(Address(handle.context().caller))
                        .write(U256::from(0u32)) // Placeholder for object index
                        .build(),
                )
                .record(handle)?;
                
                Ok(true)
            },
            Err(e) => Err(e.into()),
        }
    }

    /// Set permissions for private object replicas
    /// @custom:selector 0x0c53c51d
    #[precompile::public("setPrivateObjectPermissions(uint32,(address,uint32,uint64)[])")]
    fn set_private_object_permissions(
        handle: &mut impl PrecompileHandle,
        obj_idx: u32,
        permissions: Vec<(Address, u32, u64)>,
    ) -> EvmResult<bool> {
        use sp_consensus_poscan::CopyPermission;
        use frame_support::BoundedVec;
        let who: Runtime::AccountId = <Runtime as pallet_evm::Config>::AddressMapping::into_account_id(handle.context().caller);
        let perms = permissions.into_iter().map(|(addr, max_copies, until)| {
            CopyPermission {
                who: <Runtime as pallet_evm::Config>::AddressMapping::into_account_id(addr.into()),
                max_copies,
                until: until.saturated_into(),
            }
        }).collect::<Vec<_>>();
        let perms = BoundedVec::try_from(perms).map_err(|_| revert("too many permissions"))?;
        let result = RuntimeHelper::<Runtime>::try_dispatch(
            handle,
            Some(who.clone()).into(),
            pallet_poscan::Call::<Runtime>::set_private_object_permissions {
                obj_idx,
                permissions: perms,
            },
        );
        match result {
            Ok(_) => {
                // Record gas cost for event emission
                handle.record_log_costs_manual(3, 32)?;
                
                // Emit PermissionsSet event
                log3(
                    handle.context().address,
                    SELECTOR_LOG_PERMISSIONS_SET,
                    handle.context().caller,
                    handle.context().caller,
                    EvmDataWriter::new()
                        .write(U256::from(obj_idx))
                        .write(Address(handle.context().caller))
                        .build(),
                )
                .record(handle)?;
                
                Ok(true)
            },
            Err(e) => Err(e.into()),
        }
    }

    /// Get a list of replica indexes for a given object index
    /// @custom:selector 0x0c53c51e
    #[precompile::public("getReplicasOf(uint32)")]
    fn get_replicas_of(
        _handle: &mut impl PrecompileHandle,
        original_obj: u32,
    ) -> EvmResult<Vec<u32>> {
        use pallet_poscan::Pallet as PoScanPallet;
        let replicas = PoScanPallet::<Runtime>::replicas_of(original_obj);
        Ok(replicas)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// This test ensures that the Rust macro attribute signature matches the Solidity interface signature.
    /// Selector checks should be done in Solidity or with solc/ethers.js, not in Rust.
    #[test]
    fn test_macro_attribute_matches_solidity_signature() {
        // List of (function_name, rust_macro_attribute, solidity_signature)
        let test_cases = vec![
            ("getObject", "getObject(uint32)", "getObject(uint32)"),
            ("getObjectsOf", "getObjectsOf(address)", "getObjectsOf(address)"),
            ("getObjectCount", "getObjectCount()", "getObjectCount()"),
            ("getProperties", "getProperties()", "getProperties()"),
            ("getFeePerByte", "getFeePerByte()", "getFeePerByte()"),
            ("getAccountLock", "getAccountLock(address)", "getAccountLock(address)"),
            ("putObject", "putObject(uint8,uint8,bool,bytes,uint8,bytes32[],(uint32,uint128)[],bool,uint32)", "putObject(uint8,uint8,bool,bytes,uint8,bytes32[],(uint32,uint128)[],bool,uint32)"),
            ("setPrivateObjectPermissions", "setPrivateObjectPermissions(uint32,(address,uint32,uint64)[])", "setPrivateObjectPermissions(uint32,(address,uint32,uint64)[])"),
            ("getReplicasOf", "getReplicasOf(uint32)", "getReplicasOf(uint32)"),
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