// Copyright 2025 3Dpass
// SPDX-License-Identifier: MIT

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
use alloc::vec;
use precompile_utils::logs;
use precompile_utils::prelude::*;
use sp_core::H256;
use sp_std::{marker::PhantomData, vec::Vec};
use sp_arithmetic::traits::SaturatedConversion;
use codec::Encode;

// No alias, use precompile_utils::data::String directly
use pallet_evm::AddressMapping;
use precompile_utils::data::UnboundedString;

use sp_consensus_poscan::{ObjectState, ObjectCategory};
use frame_support::pallet_prelude::ConstU32;
use alloc::string::String;

pub const SELECTOR_GET_OBJECT: [u8; 4] = [0xA1, 0xB2, 0xC0, 0x01];

#[derive(Debug, Clone)]
pub struct PoScanPrecompile<Runtime>(PhantomData<Runtime>);

/// Event topic for ObjectSubmitted(address,uint32)
const TOPIC_OBJECT_SUBMITTED: [u8; 32] = keccak256!("ObjectSubmitted(address,uint32)");

#[precompile_utils::precompile]
impl<Runtime> PoScanPrecompile<Runtime>
where
    Runtime: pallet_evm::Config + pallet_poscan::Config,
    <Runtime as frame_system::Config>::Call: frame_support::dispatch::Dispatchable<PostInfo = frame_support::dispatch::PostDispatchInfo>
        + frame_support::dispatch::GetDispatchInfo,
    <<Runtime as frame_system::Config>::Call as frame_support::dispatch::Dispatchable>::Origin: From<Option<Runtime::AccountId>>,
    <Runtime as frame_system::Config>::Call: From<pallet_poscan::Call<Runtime>>,
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
        H256, // owner
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
            // --- owner as EVM address ---
            let owner = H256::from_slice(&obj.owner.encode());
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
            Ok((false, 0, 0, 0, 0, 0, H256::zero(), false, vec![], 0, vec![]))
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
    ) -> EvmResult<Vec<(u32, UnboundedString, u8, u128)>> {
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
                out.push((prop_idx, name, class, prop.max_value));
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
    #[precompile::public("putObject(uint8,uint8,bool,bytes,uint8,bytes32[],(uint32,uint128)[])")]
    fn put_object(
        handle: &mut impl PrecompileHandle,
        category: u8,
        algo3d: u8,
        is_private: bool,
        obj: Vec<u8>,
        num_approvals: u8,
        hashes: Vec<H256>,
        properties: Vec<(u32, u128)>,
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
        // BoundedVec conversions
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
            },
        );
        match result {
            Ok(_) => {
                // Emit ObjectSubmitted event
                // Note: If you have access to the object index, include it as a topic or data
                // Here, we emit with submitter and 0 as a placeholder for objIdx
                let _ = logs::log1(
                    handle.context().address,
                    TOPIC_OBJECT_SUBMITTED,
                    handle.context().caller.as_fixed_bytes().to_vec(),
                );
                Ok(true)
            },
            Err(e) => Err(e.into()),
        }
    }
} 