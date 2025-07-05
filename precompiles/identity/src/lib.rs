// Copyright 2025 3Dpass
// Identity precompile for Solidity-friendly access to pallet-identity

#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

use precompile_utils::prelude::*;
use sp_std::{marker::PhantomData, vec, vec::Vec, boxed::Box};
use pallet_evm::AddressMapping;
use sp_core::U256;
use frame_support::sp_runtime::SaturatedConversion;
use codec::Encode;
use frame_support::BoundedVec;
use precompile_utils::EvmResult;
use precompile_utils::{revert};
use fp_evm::PrecompileFailure;
use sp_runtime::traits::{StaticLookup, UniqueSaturatedInto};
use alloc::string::String;
use alloc::string::ToString;

// Event selectors for Identity pallet compatibility
pub const SELECTOR_LOG_IDENTITY_SET: [u8; 32] = keccak256!("IdentitySet(address)");
pub const SELECTOR_LOG_IDENTITY_CLEARED: [u8; 32] = keccak256!("IdentityCleared(address)");
pub const SELECTOR_LOG_JUDGEMENT_REQUESTED: [u8; 32] = keccak256!("JudgementRequested(address,uint32)");
pub const SELECTOR_LOG_JUDGEMENT_UNREQUESTED: [u8; 32] = keccak256!("JudgementUnrequested(address,uint32)");
pub const SELECTOR_LOG_JUDGEMENT_GIVEN: [u8; 32] = keccak256!("JudgementGiven(address,uint32)");
pub const SELECTOR_LOG_SUB_IDENTITY_ADDED: [u8; 32] = keccak256!("SubIdentityAdded(address,address)");
pub const SELECTOR_LOG_SUB_IDENTITY_REMOVED: [u8; 32] = keccak256!("SubIdentityRemoved(address,address)");
pub const SELECTOR_LOG_SUB_IDENTITY_REVOKED: [u8; 32] = keccak256!("SubIdentityRevoked(address)");

/// Identity precompile main struct
#[derive(Debug, Clone)]
pub struct IdentityPrecompile<Runtime>(PhantomData<Runtime>);

#[precompile_utils::precompile]
impl<Runtime> IdentityPrecompile<Runtime>
where
    Runtime: pallet_evm::Config + pallet_identity::Config,
    Runtime::Call: frame_support::dispatch::Dispatchable<PostInfo = frame_support::dispatch::PostDispatchInfo> + frame_support::dispatch::GetDispatchInfo,
    <Runtime::Call as frame_support::dispatch::Dispatchable>::Origin: From<Option<Runtime::AccountId>>,
    Runtime::Call: From<pallet_identity::Call<Runtime>>,
    sp_core::U256: From<BalanceOf<Runtime>>,
{
    #[precompile::public("identity(address)")]
    fn identity(
        _handle: &mut impl PrecompileHandle,
        account: Address,
    ) -> EvmResult<(
        bool, // isValid
        Vec<(u32, (bool, bool, U256, bool, bool, bool, bool, bool))>, // JudgementInfo[]
        U256, // deposit
        (
            Vec<((bool, UnboundedBytes), (bool, UnboundedBytes))>, // Additional[]
            (bool, UnboundedBytes), // display
            (bool, UnboundedBytes), // legal
            (bool, UnboundedBytes), // web
            (bool, UnboundedBytes), // riot
            (bool, UnboundedBytes), // email
            bool, // hasPgpFingerprint
            UnboundedBytes, // pgpFingerprint
            (bool, UnboundedBytes), // image
            (bool, UnboundedBytes), // twitter
        ) // IdentityInfo
    )> {
        let account_id = <Runtime as pallet_evm::Config>::AddressMapping::into_account_id(account.into());
        let opt = pallet_identity::Pallet::<Runtime>::identity(&account_id);
        if let Some(reg) = opt {
            let judgements = reg.judgements.into_iter().map(|(idx, j)| {
                // Fix 1: Ensure the tuple returned by encode_judgement matches the expected type in the Vec
                (idx, encode_judgement::<Runtime>(j))
            }).collect();
            let deposit = U256::from(reg.deposit);
            let info = reg.info;
            let additional = info.additional.into_iter().map(|(k, v)| {
                (encode_data_unbounded(k), encode_data_unbounded(v))
            }).collect();
            let display = encode_data_unbounded(info.display);
            let legal = encode_data_unbounded(info.legal);
            let web = encode_data_unbounded(info.web);
            let riot = encode_data_unbounded(info.riot);
            let email = encode_data_unbounded(info.email);
            let has_pgp = info.pgp_fingerprint.is_some();
            let pgp_fingerprint = info.pgp_fingerprint.map(|f| UnboundedBytes::from(f.to_vec())).unwrap_or_else(|| UnboundedBytes::from(vec![]));
            let image = encode_data_unbounded(info.image);
            let twitter = encode_data_unbounded(info.twitter);
            Ok((
                true,
                judgements,
                deposit,
                (additional, display, legal, web, riot, email, has_pgp, pgp_fingerprint, image, twitter)
            ))
        } else {
            Ok((false, vec![], U256::zero(), (vec![], (false, UnboundedBytes::from(vec![])), (false, UnboundedBytes::from(vec![])), (false, UnboundedBytes::from(vec![])), (false, UnboundedBytes::from(vec![])), (false, UnboundedBytes::from(vec![])), false, UnboundedBytes::from(vec![]), (false, UnboundedBytes::from(vec![])), (false, UnboundedBytes::from(vec![])))))
        }
    }

    #[precompile::public("superOf(address)")]
    fn super_of(
        _handle: &mut impl PrecompileHandle,
        who: Address,
    ) -> EvmResult<(
        bool, // isValid
        Vec<u8>, // account (AccountId32 as bytes)
        (bool, UnboundedBytes) // Data
    )> {
        let account_id = Runtime::AddressMapping::into_account_id(who.into());
        if let Some((super_acc, data)) = pallet_identity::Pallet::<Runtime>::super_of(&account_id) {
            let super_bytes = super_acc.encode();
            let data_tuple = encode_data_unbounded(data);
            Ok((true, super_bytes, data_tuple))
        } else {
            Ok((false, vec![], (false, UnboundedBytes::from(vec![]))))
        }
    }

    #[precompile::public("subsOf(address)")]
    fn subs_of(
        _handle: &mut impl PrecompileHandle,
        who: Address,
    ) -> EvmResult<(
        U256, // deposit
        Vec<Vec<u8>> // accounts (AccountId32 as bytes)
    )> {
        let account_id = Runtime::AddressMapping::into_account_id(who.into());
        let (deposit, subs) = pallet_identity::Pallet::<Runtime>::subs_of(&account_id);
        let deposit_u256 = U256::from(deposit);
        let sub_bytes: Vec<Vec<u8>> = subs.into_iter().map(|acc| acc.encode()).collect();
        Ok((deposit_u256, sub_bytes))
    }

    #[precompile::public("registrars()")]
    fn registrars(
        _handle: &mut impl PrecompileHandle,
    ) -> EvmResult<Vec<(
        bool, // isValid
        u32, // index
        Vec<u8>, // account (AccountId32 as bytes)
        U256, // fee
        (bool, bool, bool, bool, bool, bool, bool, bool) // IdentityFields
    )>> {
        let regs = pallet_identity::Pallet::<Runtime>::registrars();
        let mut out = Vec::with_capacity(regs.len());
        for (i, reg) in regs.into_iter().enumerate() {
            if let Some(info) = reg {
                let acc_bytes = info.account.encode();
                let fee = U256::from(info.fee);
                let fields = encode_identity_fields(info.fields.0.bits());
                out.push((true, i as u32, acc_bytes, fee, fields));
            } else {
                out.push((false, i as u32, vec![], U256::zero(), (false, false, false, false, false, false, false, false)));
            }
        }
        Ok(out)
    }

    #[precompile::public("suspendedRegistrars()")]
    #[precompile::view]
    fn suspended_registrars(
        _handle: &mut impl PrecompileHandle,
    ) -> EvmResult<Vec<u32>> {
        // Query the pallet_identity::SuspendedRegistrars storage item
        let suspended = pallet_identity::Pallet::<Runtime>::susp_registrars();
        Ok(suspended.to_vec())
    }

    #[precompile::public("setIdentity((((bool,bytes),(bool,bytes))[],(bool,bytes),(bool,bytes),(bool,bytes),(bool,bytes),(bool,bytes),bool,bytes,(bool,bytes),(bool,bytes)))")]
    fn set_identity(
        handle: &mut impl PrecompileHandle,
        info: (
            Vec<((bool, UnboundedBytes), (bool, UnboundedBytes))>, // additional
            (bool, UnboundedBytes), // display
            (bool, UnboundedBytes), // legal
            (bool, UnboundedBytes), // web
            (bool, UnboundedBytes), // riot
            (bool, UnboundedBytes), // email
            bool, // hasPgpFingerprint
            UnboundedBytes, // pgpFingerprint
            (bool, UnboundedBytes), // image
            (bool, UnboundedBytes), // twitter
        ),
    ) -> EvmResult<bool> {
        let who: Runtime::AccountId = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let info = decode_identity_info::<Runtime>(info)?;
        
        // Record gas cost for event emission
        handle.record_log_costs_manual(3, 32)?;
        
        RuntimeHelper::<Runtime>::try_dispatch(
            handle,
            Some(who.clone()).into(),
            pallet_identity::Call::<Runtime>::set_identity { info: Box::new(info) },
        )?;
        
        // Emit IdentitySet event
        log3(
            handle.context().address,
            SELECTOR_LOG_IDENTITY_SET,
            handle.context().caller,
            handle.context().caller,
            EvmDataWriter::new()
                .write(Address(handle.context().caller))
                .build(),
        )
        .record(handle)?;
        
        Ok(true)
    }

    #[precompile::public("setSubs((bytes32,(bool,bytes))[])")]
    fn set_subs(
        handle: &mut impl PrecompileHandle,
        subs: Vec<(Bytes32, (bool, UnboundedBytes))>,
    ) -> EvmResult<bool> {
        let who: Runtime::AccountId = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let subs: Result<Vec<_>, PrecompileFailure> = subs.into_iter().map(|(acc, data)| {
            Ok((decode_account_id::<Runtime>(acc.0.to_vec())?, decode_data(data)))
        }).collect();
        let subs = subs?;
        RuntimeHelper::<Runtime>::try_dispatch(
            handle,
            Some(who.clone()).into(),
            pallet_identity::Call::<Runtime>::set_subs { subs },
        )?;
        Ok(true)
    }

    #[precompile::public("clearIdentity()")]
    fn clear_identity(
        handle: &mut impl PrecompileHandle,
    ) -> EvmResult<bool> {
        let who: Runtime::AccountId = Runtime::AddressMapping::into_account_id(handle.context().caller);
        
        // Record gas cost for event emission
        handle.record_log_costs_manual(3, 32)?;
        
        RuntimeHelper::<Runtime>::try_dispatch(
            handle,
            Some(who.clone()).into(),
            pallet_identity::Call::<Runtime>::clear_identity {},
        )?;
        
        // Emit IdentityCleared event
        log3(
            handle.context().address,
            SELECTOR_LOG_IDENTITY_CLEARED,
            handle.context().caller,
            handle.context().caller,
            EvmDataWriter::new()
                .write(Address(handle.context().caller))
                .build(),
        )
        .record(handle)?;
        
        Ok(true)
    }

    #[precompile::public("requestJudgement(uint32,uint256)")]
    fn request_judgement(
        handle: &mut impl PrecompileHandle,
        reg_index: u32,
        max_fee: U256,
    ) -> EvmResult<bool> {
        let who: Runtime::AccountId = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let max_fee = decode_balance::<Runtime>(max_fee)?;
        
        // Record gas cost for event emission
        handle.record_log_costs_manual(3, 32)?;
        
        RuntimeHelper::<Runtime>::try_dispatch(
            handle,
            Some(who.clone()).into(),
            pallet_identity::Call::<Runtime>::request_judgement { reg_index, max_fee },
        )?;
        
        // Emit JudgementRequested event
        log3(
            handle.context().address,
            SELECTOR_LOG_JUDGEMENT_REQUESTED,
            handle.context().caller,
            handle.context().caller,
            EvmDataWriter::new()
                .write(Address(handle.context().caller))
                .write(sp_core::U256([reg_index as u64, 0, 0, 0]))
                .build(),
        )
        .record(handle)?;
        
        Ok(true)
    }

    #[precompile::public("cancelRequest(uint32)")]
    fn cancel_request(
        handle: &mut impl PrecompileHandle,
        reg_index: u32,
    ) -> EvmResult<bool> {
        let who: Runtime::AccountId = Runtime::AddressMapping::into_account_id(handle.context().caller);
        
        // Record gas cost for event emission
        handle.record_log_costs_manual(3, 32)?;
        
        RuntimeHelper::<Runtime>::try_dispatch(
            handle,
            Some(who.clone()).into(),
            pallet_identity::Call::<Runtime>::cancel_request { reg_index },
        )?;
        
        // Emit JudgementUnrequested event
        log3(
            handle.context().address,
            SELECTOR_LOG_JUDGEMENT_UNREQUESTED,
            handle.context().caller,
            handle.context().caller,
            EvmDataWriter::new()
                .write(Address(handle.context().caller))
                .write(sp_core::U256([reg_index as u64, 0, 0, 0]))
                .build(),
        )
        .record(handle)?;
        
        Ok(true)
    }

    #[precompile::public("setFee(uint32,uint256)")]
    fn set_fee(
        handle: &mut impl PrecompileHandle,
        reg_index: u32,
        fee: U256,
    ) -> EvmResult<bool> {
        let who: Runtime::AccountId = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let fee = decode_balance::<Runtime>(fee)?;
        RuntimeHelper::<Runtime>::try_dispatch(
            handle,
            Some(who.clone()).into(),
            pallet_identity::Call::<Runtime>::set_fee { index: reg_index, fee },
        )?;
        Ok(true)
    }

    #[precompile::public("setAccountId(uint32,bytes32)")]
    fn set_account_id(
        handle: &mut impl PrecompileHandle,
        reg_index: u32,
        new_account: Bytes32,
    ) -> EvmResult<bool> {
        let who: Runtime::AccountId = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let new_account = decode_account_id::<Runtime>(new_account.0.to_vec())?;
        RuntimeHelper::<Runtime>::try_dispatch(
            handle,
            Some(who.clone()).into(),
            pallet_identity::Call::<Runtime>::set_account_id { index: reg_index, new: new_account },
        )?;
        Ok(true)
    }

    #[precompile::public("setFields(uint32,(bool,bool,bool,bool,bool,bool,bool,bool))")]
    fn set_fields(
        handle: &mut impl PrecompileHandle,
        reg_index: u32,
        fields: (bool, bool, bool, bool, bool, bool, bool, bool),
    ) -> EvmResult<bool> {
        let who: Runtime::AccountId = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let fields = encode_identity_fields_to_struct(fields);
        RuntimeHelper::<Runtime>::try_dispatch(
            handle,
            Some(who.clone()).into(),
            pallet_identity::Call::<Runtime>::set_fields { index: reg_index, fields },
        )?;
        Ok(true)
    }

    #[precompile::public("provideJudgement(uint32,bytes32,(bool,bool,uint256,bool,bool,bool,bool,bool),bytes32)")]
    fn provide_judgement(
        handle: &mut impl PrecompileHandle,
        reg_index: u32,
        target: Bytes32,
        judgement: (bool, bool, U256, bool, bool, bool, bool, bool),
        _identity: Bytes32, // Not used in Substrate call, but present in Solidity interface
    ) -> EvmResult<bool> {
        let who: Runtime::AccountId = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let target = decode_account_id::<Runtime>(target.0.to_vec())?;
        let target = <Runtime as frame_system::Config>::Lookup::unlookup(target); // Fix: use lookup source
        let judgement = decode_judgement::<Runtime>(judgement)?;
        
        // Record gas cost for event emission
        handle.record_log_costs_manual(3, 32)?;
        
        RuntimeHelper::<Runtime>::try_dispatch(
            handle,
            Some(who.clone()).into(),
            pallet_identity::Call::<Runtime>::provide_judgement { reg_index, target, judgement },
        )?;
        
        // Emit JudgementGiven event
        log3(
            handle.context().address,
            SELECTOR_LOG_JUDGEMENT_GIVEN,
            handle.context().caller,
            handle.context().caller,
            EvmDataWriter::new()
                .write(Address(handle.context().caller))
                .write(sp_core::U256([reg_index as u64, 0, 0, 0]))
                .build(),
        )
        .record(handle)?;
        
        Ok(true)
    }

    #[precompile::public("addSub(bytes32,(bool,bytes))")]
    fn add_sub(
        handle: &mut impl PrecompileHandle,
        sub: Bytes32,
        data: (bool, UnboundedBytes),
    ) -> EvmResult<bool> {
        let who: Runtime::AccountId = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let sub = decode_account_id::<Runtime>(sub.0.to_vec())?;
        let sub_lookup = <Runtime as frame_system::Config>::Lookup::unlookup(sub);
        let data = decode_data(data);
        
        // Record gas cost for event emission
        handle.record_log_costs_manual(3, 32)?;
        
        RuntimeHelper::<Runtime>::try_dispatch(
            handle,
            Some(who.clone()).into(),
            pallet_identity::Call::<Runtime>::add_sub { sub: sub_lookup, data },
        )?;
        
        // Emit SubIdentityAdded event
        log3(
            handle.context().address,
            SELECTOR_LOG_SUB_IDENTITY_ADDED,
            handle.context().caller,
            handle.context().caller,
            EvmDataWriter::new()
                .write(Address(handle.context().caller))
                .write(Address(handle.context().caller))
                .build(),
        )
        .record(handle)?;
        
        Ok(true)
    }

    #[precompile::public("renameSub(bytes32,(bool,bytes))")]
    fn rename_sub(
        handle: &mut impl PrecompileHandle,
        sub: Bytes32,
        data: (bool, UnboundedBytes),
    ) -> EvmResult<bool> {
        let who: Runtime::AccountId = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let sub = decode_account_id::<Runtime>(sub.0.to_vec())?;
        let sub_lookup = <Runtime as frame_system::Config>::Lookup::unlookup(sub);
        let data = decode_data(data);
        RuntimeHelper::<Runtime>::try_dispatch(
            handle,
            Some(who.clone()).into(),
            pallet_identity::Call::<Runtime>::rename_sub { sub: sub_lookup, data },
        )?;
        Ok(true)
    }

    #[precompile::public("removeSub(bytes32)")]
    fn remove_sub(
        handle: &mut impl PrecompileHandle,
        sub: Bytes32,
    ) -> EvmResult<bool> {
        let who: Runtime::AccountId = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let sub = decode_account_id::<Runtime>(sub.0.to_vec())?;
        let sub_lookup = <Runtime as frame_system::Config>::Lookup::unlookup(sub);
        
        // Record gas cost for event emission
        handle.record_log_costs_manual(3, 32)?;
        
        RuntimeHelper::<Runtime>::try_dispatch(
            handle,
            Some(who.clone()).into(),
            pallet_identity::Call::<Runtime>::remove_sub { sub: sub_lookup },
        )?;
        
        // Emit SubIdentityRemoved event
        log3(
            handle.context().address,
            SELECTOR_LOG_SUB_IDENTITY_REMOVED,
            handle.context().caller,
            handle.context().caller,
            EvmDataWriter::new()
                .write(Address(handle.context().caller))
                .write(Address(handle.context().caller))
                .build(),
        )
        .record(handle)?;
        
        Ok(true)
    }

    #[precompile::public("quitSub()")]
    fn quit_sub(
        handle: &mut impl PrecompileHandle,
    ) -> EvmResult<bool> {
        let who: Runtime::AccountId = Runtime::AddressMapping::into_account_id(handle.context().caller);
        
        // Record gas cost for event emission
        handle.record_log_costs_manual(3, 32)?;
        
        RuntimeHelper::<Runtime>::try_dispatch(
            handle,
            Some(who.clone()).into(),
            pallet_identity::Call::<Runtime>::quit_sub {},
        )?;
        
        // Emit SubIdentityRevoked event
        log3(
            handle.context().address,
            SELECTOR_LOG_SUB_IDENTITY_REVOKED,
            handle.context().caller,
            handle.context().caller,
            EvmDataWriter::new()
                .write(Address(handle.context().caller))
                .build(),
        )
        .record(handle)?;
        
        Ok(true)
    }
}



// Helper: Convert pallet_identity::Data to (bool, UnboundedBytes)
fn encode_data_unbounded(data: pallet_identity::Data) -> (bool, UnboundedBytes) {
    match data {
        pallet_identity::Data::None => (false, UnboundedBytes::from(vec![])),
        pallet_identity::Data::Raw(bv) => (true, UnboundedBytes::from(bv.into_inner())),
        pallet_identity::Data::BlakeTwo256(arr)
        | pallet_identity::Data::Sha256(arr)
        | pallet_identity::Data::Keccak256(arr)
        | pallet_identity::Data::ShaThree256(arr) => (true, UnboundedBytes::from(arr.to_vec())),
    }
}

// Helper: Encode Judgement as (isUnknown, isFeePaid, feePaidDeposit, isReasonable, isKnownGood, isOutOfDate, isLowQuality, isErroneous)
fn encode_judgement<Runtime: pallet_identity::Config>(j: pallet_identity::Judgement<BalanceOf<Runtime>>) -> (bool, bool, U256, bool, bool, bool, bool, bool) {
    match j {
        pallet_identity::Judgement::Unknown => (true, false, U256::zero(), false, false, false, false, false),
        pallet_identity::Judgement::FeePaid(fee) => (false, true, U256::from(fee.saturated_into::<u128>()), false, false, false, false, false),
        pallet_identity::Judgement::Reasonable => (false, false, U256::zero(), true, false, false, false, false),
        pallet_identity::Judgement::KnownGood => (false, false, U256::zero(), false, true, false, false, false),
        pallet_identity::Judgement::OutOfDate => (false, false, U256::zero(), false, false, true, false, false),
        pallet_identity::Judgement::LowQuality => (false, false, U256::zero(), false, false, false, true, false),
        pallet_identity::Judgement::Erroneous => (false, false, U256::zero(), false, false, false, false, true),
    }
}

// Helper: Encode IdentityFields as (display, legal, web, riot, email, pgpFingerprint, image, twitter)
fn encode_identity_fields(bits: u64) -> (bool, bool, bool, bool, bool, bool, bool, bool) {
    (
        bits & (1 << 0) != 0,
        bits & (1 << 1) != 0,
        bits & (1 << 2) != 0,
        bits & (1 << 3) != 0,
        bits & (1 << 4) != 0,
        bits & (1 << 5) != 0,
        bits & (1 << 6) != 0,
        bits & (1 << 7) != 0,
    )
}

// Type alias for the correct Balance type
pub type BalanceOf<Runtime> = <<Runtime as pallet_identity::Config>::Currency as frame_support::traits::Currency<<Runtime as frame_system::Config>::AccountId>>::Balance;

// --- Decoding helpers for extrinsics ---
// --- Fix 2: decode_account_id returns associated AccountId ---
fn decode_account_id<Runtime: pallet_identity::Config>(bytes: Vec<u8>) -> EvmResult<<Runtime as frame_system::Config>::AccountId> {
    use codec::Decode;
    if bytes.len() != 32 {
        return Err(revert("AccountId must be 32 bytes"));
    }
    <Runtime as frame_system::Config>::AccountId::decode(&mut &bytes[..])
        .map_err(|_| revert("Failed to decode AccountId"))
}

fn decode_data(data: (bool, UnboundedBytes)) -> pallet_identity::Data {
    if !data.0 {
        pallet_identity::Data::None
    } else if data.1.as_bytes().len() == 32 {
        // Try to match hash types
        pallet_identity::Data::BlakeTwo256(data.1.as_bytes().try_into().unwrap_or([0u8; 32]))
    } else {
        pallet_identity::Data::Raw(BoundedVec::truncate_from(data.1.as_bytes().to_vec()))
    }
}



// --- Fix 3: Use BoundedVec::try_from for additional fields in decode_identity_info ---
fn decode_identity_info<Runtime: pallet_identity::Config>(info: (
    Vec<((bool, UnboundedBytes), (bool, UnboundedBytes))>,
    (bool, UnboundedBytes),
    (bool, UnboundedBytes),
    (bool, UnboundedBytes),
    (bool, UnboundedBytes),
    (bool, UnboundedBytes),
    bool,
    UnboundedBytes,
    (bool, UnboundedBytes),
    (bool, UnboundedBytes),
)) -> EvmResult<pallet_identity::IdentityInfo<Runtime::MaxAdditionalFields>> {
    let additional_vec: Vec<_> = info.0.into_iter().map(|(k, v)| (decode_data(k), decode_data(v))).collect();
    let additional = BoundedVec::try_from(additional_vec)
        .map_err(|_| revert("BoundedVec overflow in additional fields"))?;
    if info.6 && info.7.as_bytes().len() != 20 {
        return Err(revert("pgp_fingerprint must be 20 bytes if present"));
    }
    Ok(pallet_identity::IdentityInfo {
        additional,
        display: decode_data(info.1),
        legal: decode_data(info.2),
        web: decode_data(info.3),
        riot: decode_data(info.4),
        email: decode_data(info.5),
        pgp_fingerprint: if info.6 { Some(<[u8; 20]>::try_from(info.7.as_bytes()).map_err(|_| revert("pgp_fingerprint must be 20 bytes"))?) } else { None },
        image: decode_data(info.8),
        twitter: decode_data(info.9),
    })
}

fn decode_balance<Runtime: pallet_identity::Config>(val: U256) -> EvmResult<BalanceOf<Runtime>> {
    // Use UniqueSaturatedInto for robust balance conversion
    Ok(val.as_u128().unique_saturated_into())
}

fn decode_judgement<Runtime: pallet_identity::Config>(j: (bool, bool, U256, bool, bool, bool, bool, bool)) -> EvmResult<pallet_identity::Judgement<BalanceOf<Runtime>>> {
    // Count how many discriminants are set
    let discriminants = [j.0, j.1, j.3, j.4, j.5, j.6, j.7];
    let count = discriminants.iter().filter(|&&b| b).count();
    if count > 1 {
        return Err(revert("Only one Judgement discriminant can be set"));
    }
    use pallet_identity::Judgement;
    if j.0 { Ok(Judgement::Unknown) }
    else if j.1 { Ok(Judgement::FeePaid(decode_balance::<Runtime>(j.2)?)) }
    else if j.3 { Ok(Judgement::Reasonable) }
    else if j.4 { Ok(Judgement::KnownGood) }
    else if j.5 { Ok(Judgement::OutOfDate) }
    else if j.6 { Ok(Judgement::LowQuality) }
    else if j.7 { Ok(Judgement::Erroneous) }
    else { Err(revert("Invalid Judgement discriminant")) }
}

fn encode_identity_fields_to_struct(fields: (bool, bool, bool, bool, bool, bool, bool, bool)) -> pallet_identity::IdentityFields {
    let mut bits = 0u64;
    if fields.0 { bits |= 1 << 0; }
    if fields.1 { bits |= 1 << 1; }
    if fields.2 { bits |= 1 << 2; }
    if fields.3 { bits |= 1 << 3; }
    if fields.4 { bits |= 1 << 4; }
    if fields.5 { bits |= 1 << 5; }
    if fields.6 { bits |= 1 << 6; }
    if fields.7 { bits |= 1 << 7; }
    pallet_identity::IdentityFields(unsafe { core::mem::transmute(bits) })
} 

/// Wrapper for bytes32 to support EvmData
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bytes32(pub [u8; 32]);

impl precompile_utils::EvmData for Bytes32 {
    fn read(reader: &mut precompile_utils::EvmDataReader) -> MayRevert<Self> {
        let bytes: Vec<u8> = reader.read()?;
        if bytes.len() != 32 {
            return Err(Revert::new(RevertReason::Custom("bytes32 argument must be exactly 32 bytes".to_string())));
        }
        let arr: [u8; 32] = bytes.as_slice().try_into().unwrap();
        Ok(Bytes32(arr))
    }
    fn write(writer: &mut precompile_utils::EvmDataWriter, value: Self) {
        *writer = core::mem::replace(writer, precompile_utils::EvmDataWriter::default()).write(value.0.to_vec());
    }
    fn has_static_size() -> bool { true }
    fn solidity_type() -> String { "bytes32".to_string() }
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
            ("identity", "identity(address)", "identity(address)"),
            ("superOf", "superOf(address)", "superOf(address)"),
            ("subsOf", "subsOf(address)", "subsOf(address)"),
            ("registrars", "registrars()", "registrars()"),
            ("suspendedRegistrars", "suspendedRegistrars()", "suspendedRegistrars()"),
            ("setIdentity", "setIdentity(((bool,bytes),(bool,bytes))[],(bool,bytes),(bool,bytes),(bool,bytes),(bool,bytes),(bool,bytes),bool,bytes,(bool,bytes),(bool,bytes))", "setIdentity(((bool,bytes),(bool,bytes))[],(bool,bytes),(bool,bytes),(bool,bytes),(bool,bytes),(bool,bytes),bool,bytes,(bool,bytes),(bool,bytes))"),
            ("setSubs", "setSubs((bytes32,(bool,bytes))[])", "setSubs((bytes32,(bool,bytes))[])"),
            ("clearIdentity", "clearIdentity()", "clearIdentity()"),
            ("requestJudgement", "requestJudgement(uint32,uint256)", "requestJudgement(uint32,uint256)"),
            ("cancelRequest", "cancelRequest(uint32)", "cancelRequest(uint32)"),
            ("setFee", "setFee(uint32,uint256)", "setFee(uint32,uint256)"),
            ("setAccountId", "setAccountId(uint32,bytes32)", "setAccountId(uint32,bytes32)"),
            ("setFields", "setFields(uint32,(bool,bool,bool,bool,bool,bool,bool,bool))", "setFields(uint32,(bool,bool,bool,bool,bool,bool,bool,bool))"),
            ("provideJudgement", "provideJudgement(uint32,bytes32,(bool,bool,uint256,bool,bool,bool,bool,bool),bytes32)", "provideJudgement(uint32,bytes32,(bool,bool,uint256,bool,bool,bool,bool,bool),bytes32)"),
            ("addSub", "addSub(bytes32,(bool,bytes))", "addSub(bytes32,(bool,bytes))"),
            ("renameSub", "renameSub(bytes32,(bool,bytes))", "renameSub(bytes32,(bool,bytes))"),
            ("removeSub", "removeSub(bytes32)", "removeSub(bytes32)"),
            ("quitSub", "quitSub()", "quitSub()"),
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