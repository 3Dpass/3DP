// Copyright 2025 3Dpass
// This file is part of 3Dpass.

#![cfg_attr(not(feature = "std"), no_std)]

use precompile_utils::prelude::*;
use sp_core::H160;
use sp_core::U256;
use sp_std::{marker::PhantomData, vec::Vec};
use pallet_evm::AddressMapping;
use frame_support::BoundedVec;
use scale_info::prelude::format;

use pallet_asset_conversion::{Pools, MultiAssetIdConverter, MultiAssetIdConversionResult, NativeOrAssetId};

pub const SELECTOR_QUERY_PAIRS: [u8; 4] = [0xA1, 0xB2, 0xC3, 0xD4];

// --- Discriminant helpers for asset address parsing ---
const NATIVE_TOKEN_ADDRESS: [u8; 20] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x02];
const LOCAL_ASSET_PREFIX: [u8; 4] = [0xfb, 0xfb, 0xfb, 0xfa];

/// Enum for asset discriminant
#[derive(Debug, Clone, PartialEq, Eq)]
enum PalletAssetConversionNativeOrAssetId {
    Native,
    Asset(u32),
}

/// Parse an H160 address into a discriminant (Native or Asset(u32)), or None if invalid
fn parse_asset_address(address: &Address) -> Option<PalletAssetConversionNativeOrAssetId> {
    let bytes = address.0.as_bytes();
    if bytes.len() != 20 {
        return None; // Error: address must be 20 bytes
    }
    if bytes == &NATIVE_TOKEN_ADDRESS {
        Some(PalletAssetConversionNativeOrAssetId::Native)
    } else if bytes[0..4] == LOCAL_ASSET_PREFIX {
        let asset_id = u32::from_be_bytes([
            bytes[16], bytes[17], bytes[18], bytes[19]
        ]);
        Some(PalletAssetConversionNativeOrAssetId::Asset(asset_id))
    } else {
        None // Error: address does not match native or local asset pattern
    }
}

#[derive(Debug, Clone)]
pub struct AssetsConversionPrecompile<Runtime>(PhantomData<Runtime>);

#[precompile_utils::precompile]
impl<Runtime> AssetsConversionPrecompile<Runtime>
where
    Runtime: pallet_evm::Config + pallet_asset_conversion::Config,
    <Runtime as pallet_asset_conversion::Config>::AssetId: Into<u32> + Copy + Ord + From<u32>,
    <Runtime as pallet_asset_conversion::Config>::PoolAssetId: Into<u32> + Copy,
    Runtime::Call: frame_support::dispatch::Dispatchable<PostInfo = frame_support::dispatch::PostDispatchInfo> + frame_support::dispatch::GetDispatchInfo,
    <Runtime::Call as frame_support::dispatch::Dispatchable>::Origin: From<Option<Runtime::AccountId>>,
    Runtime::Call: From<pallet_asset_conversion::Call<Runtime>>,
    <Runtime as pallet_asset_conversion::Config>::MultiAssetId: From<NativeOrAssetId<<Runtime as pallet_asset_conversion::Config>::AssetId>>,
    <Runtime as pallet_asset_conversion::Config>::AssetBalance: core::convert::TryFrom<U256>,
{
    #[precompile::public("queryPairs()")]
    fn query_pairs(
        _handle: &mut impl PrecompileHandle,
    ) -> EvmResult<Vec<(
        bool, // isValid
        Address, // pair_id
        (bool, Address), // token0: (hasData, id)
        bool, // token0_is_native
        (bool, Address), // token1: (hasData, id)
        bool  // token1_is_native
    )>> {
        let mut out = Vec::new();
        for (pool_id, pool_info) in Pools::<Runtime>::iter() {
            let (asset0, asset1) = pool_id;
            let lp_token = pool_info.lp_token;
            let pair_id = format_lp_token_address(&lp_token);

            // token0
            let (token0_id, token0_is_native, token0_has_data) = if <Runtime as pallet_asset_conversion::Config>::MultiAssetIdConverter::is_native(&asset0) {
                (native_token_address(), true, true)
            } else if let MultiAssetIdConversionResult::Converted(asset_id) =
                <Runtime as pallet_asset_conversion::Config>::MultiAssetIdConverter::try_convert(&asset0)
            {
                (format_non_native_token_address(&asset_id), false, true)
            } else {
                (Address(H160::zero()), false, false)
            };

            // token1
            let (token1_id, token1_is_native, token1_has_data) = if <Runtime as pallet_asset_conversion::Config>::MultiAssetIdConverter::is_native(&asset1) {
                (native_token_address(), true, true)
            } else if let MultiAssetIdConversionResult::Converted(asset_id) =
                <Runtime as pallet_asset_conversion::Config>::MultiAssetIdConverter::try_convert(&asset1)
            {
                (format_non_native_token_address(&asset_id), false, true)
            } else {
                (Address(H160::zero()), false, false)
            };

            let is_valid = token0_has_data && token1_has_data;
            out.push((is_valid, pair_id, (token0_has_data, token0_id), token0_is_native, (token1_has_data, token1_id), token1_is_native));
        }
        Ok(out)
    }

    #[precompile::public("nextPoolAssetId()")]
    fn next_pool_asset_id(_handle: &mut impl PrecompileHandle) -> EvmResult<(bool, u32)> {
        let opt = pallet_asset_conversion::NextPoolAssetId::<Runtime>::get();
        match opt {
            Some(id) => Ok((true, id.into())),
            None => Ok((false, 0)),
        }
    }

    #[precompile::public("createPool(address,address)")]
    fn create_pool(
        handle: &mut impl PrecompileHandle,
        asset1: Address,
        asset2: Address,
    ) -> EvmResult<bool> {
        let asset1_id = match parse_asset_address(&asset1) {
            Some(PalletAssetConversionNativeOrAssetId::Native) =>
                NativeOrAssetId::Native.into(),
            Some(PalletAssetConversionNativeOrAssetId::Asset(id)) =>
                NativeOrAssetId::Asset(id.into()).into(),
            None => return Err(revert("asset1: invalid address (must be native or local asset, 20 bytes)")),
        };
        let asset2_id = match parse_asset_address(&asset2) {
            Some(PalletAssetConversionNativeOrAssetId::Native) =>
                NativeOrAssetId::Native.into(),
            Some(PalletAssetConversionNativeOrAssetId::Asset(id)) =>
                NativeOrAssetId::Asset(id.into()).into(),
            None => return Err(revert("asset2: invalid address (must be native or local asset, 20 bytes)")),
        };
        // Map submitter
        let who: Runtime::AccountId = Runtime::AddressMapping::into_account_id(handle.context().caller);
        // Dispatch call
        RuntimeHelper::<Runtime>::try_dispatch(
            handle,
            Some(who).into(),
            pallet_asset_conversion::Call::<Runtime>::create_pool {
                asset1: asset1_id,
                asset2: asset2_id,
            },
        )?;
        Ok(true)
    }

    #[precompile::public("addLiquidity(address,address,uint256,uint256,uint256,uint256,address)")]
    fn add_liquidity(
        handle: &mut impl PrecompileHandle,
        asset1: Address,
        asset2: Address,
        amount1_desired: U256,
        amount2_desired: U256,
        amount1_min: U256,
        amount2_min: U256,
        mint_to: Address,
    ) -> EvmResult<bool>
    where
        <Runtime as pallet_asset_conversion::Config>::AssetBalance: core::convert::TryFrom<U256>,
    {
        use core::convert::TryFrom;
        let asset1_id = match parse_asset_address(&asset1) {
            Some(PalletAssetConversionNativeOrAssetId::Native) =>
                NativeOrAssetId::Native.into(),
            Some(PalletAssetConversionNativeOrAssetId::Asset(id)) =>
                NativeOrAssetId::Asset(id.into()).into(),
            None => return Err(revert("asset1: invalid address (must be native or local asset, 20 bytes)")),
        };
        let asset2_id = match parse_asset_address(&asset2) {
            Some(PalletAssetConversionNativeOrAssetId::Native) =>
                NativeOrAssetId::Native.into(),
            Some(PalletAssetConversionNativeOrAssetId::Asset(id)) =>
                NativeOrAssetId::Asset(id.into()).into(),
            None => return Err(revert("asset2: invalid address (must be native or local asset, 20 bytes)")),
        };
        let amount1_desired = <Runtime as pallet_asset_conversion::Config>::AssetBalance::try_from(amount1_desired)
            .map_err(|_| revert("amount1_desired: overflow or negative (must fit AssetBalance type)"))?;
        let amount2_desired = <Runtime as pallet_asset_conversion::Config>::AssetBalance::try_from(amount2_desired)
            .map_err(|_| revert("amount2_desired: overflow or negative (must fit AssetBalance type)"))?;
        let amount1_min = <Runtime as pallet_asset_conversion::Config>::AssetBalance::try_from(amount1_min)
            .map_err(|_| revert("amount1_min: overflow or negative (must fit AssetBalance type)"))?;
        let amount2_min = <Runtime as pallet_asset_conversion::Config>::AssetBalance::try_from(amount2_min)
            .map_err(|_| revert("amount2_min: overflow or negative (must fit AssetBalance type)"))?;
        // Parse mint_to
        let mint_to: Runtime::AccountId = Runtime::AddressMapping::into_account_id(mint_to.into());
        // Map submitter
        let who: Runtime::AccountId = Runtime::AddressMapping::into_account_id(handle.context().caller);
        // Dispatch call
        RuntimeHelper::<Runtime>::try_dispatch(
            handle,
            Some(who).into(),
            pallet_asset_conversion::Call::<Runtime>::add_liquidity {
                asset1: asset1_id,
                asset2: asset2_id,
                amount1_desired,
                amount2_desired,
                amount1_min,
                amount2_min,
                mint_to,
            },
        )?;
        Ok(true)
    }

    #[precompile::public("removeLiquidity(address,address,uint256,uint256,uint256,address)")]
    fn remove_liquidity(
        handle: &mut impl PrecompileHandle,
        asset1: Address,
        asset2: Address,
        lp_token_burn: U256,
        amount1_min_receive: U256,
        amount2_min_receive: U256,
        withdraw_to: Address,
    ) -> EvmResult<bool>
    where
        <Runtime as pallet_asset_conversion::Config>::AssetBalance: core::convert::TryFrom<U256>,
    {
        use core::convert::TryFrom;
        let asset1_id = match parse_asset_address(&asset1) {
            Some(PalletAssetConversionNativeOrAssetId::Native) =>
                NativeOrAssetId::Native.into(),
            Some(PalletAssetConversionNativeOrAssetId::Asset(id)) =>
                NativeOrAssetId::Asset(id.into()).into(),
            None => return Err(revert("asset1: invalid address (must be native or local asset, 20 bytes)")),
        };
        let asset2_id = match parse_asset_address(&asset2) {
            Some(PalletAssetConversionNativeOrAssetId::Native) =>
                NativeOrAssetId::Native.into(),
            Some(PalletAssetConversionNativeOrAssetId::Asset(id)) =>
                NativeOrAssetId::Asset(id.into()).into(),
            None => return Err(revert("asset2: invalid address (must be native or local asset, 20 bytes)")),
        };
        let lp_token_burn = <Runtime as pallet_asset_conversion::Config>::AssetBalance::try_from(lp_token_burn)
            .map_err(|_| revert("lpTokenBurn: overflow or negative (must fit AssetBalance type)"))?;
        let amount1_min_receive = <Runtime as pallet_asset_conversion::Config>::AssetBalance::try_from(amount1_min_receive)
            .map_err(|_| revert("amount1MinReceive: overflow or negative (must fit AssetBalance type)"))?;
        let amount2_min_receive = <Runtime as pallet_asset_conversion::Config>::AssetBalance::try_from(amount2_min_receive)
            .map_err(|_| revert("amount2MinReceive: overflow or negative (must fit AssetBalance type)"))?;
        // Parse withdraw_to
        let withdraw_to: Runtime::AccountId = Runtime::AddressMapping::into_account_id(withdraw_to.into());
        // Map submitter
        let who: Runtime::AccountId = Runtime::AddressMapping::into_account_id(handle.context().caller);
        // Dispatch call
        RuntimeHelper::<Runtime>::try_dispatch(
            handle,
            Some(who).into(),
            pallet_asset_conversion::Call::<Runtime>::remove_liquidity {
                asset1: asset1_id,
                asset2: asset2_id,
                lp_token_burn,
                amount1_min_receive,
                amount2_min_receive,
                withdraw_to,
            },
        )?;
        Ok(true)
    }

    #[precompile::public("swapExactTokensForTokens(address[],uint256,uint256,address,bool)")]
    fn swap_exact_tokens_for_tokens(
        handle: &mut impl PrecompileHandle,
        path: Vec<Address>,
        amount_in: U256,
        amount_out_min: U256,
        send_to: Address,
        keep_alive: bool,
    ) -> EvmResult<bool>
    where
        <Runtime as pallet_asset_conversion::Config>::AssetBalance: core::convert::TryFrom<U256>,
        <Runtime as pallet_asset_conversion::Config>::MultiAssetId: From<NativeOrAssetId<<Runtime as pallet_asset_conversion::Config>::AssetId>>,
    {
        use core::convert::TryFrom;
        let mut parsed_path = Vec::with_capacity(path.len());
        for (i, addr) in path.iter().enumerate() {
            let asset = match parse_asset_address(addr) {
                Some(PalletAssetConversionNativeOrAssetId::Native) =>
                    NativeOrAssetId::Native,
                Some(PalletAssetConversionNativeOrAssetId::Asset(id)) =>
                    NativeOrAssetId::Asset(id.into()),
                None => return Err(revert(format!("path[{}]: invalid address (must be native or local asset, 20 bytes)", i))),
            };
            parsed_path.push(<Runtime as pallet_asset_conversion::Config>::MultiAssetId::from(asset));
        }
        let parsed_path = BoundedVec::try_from(parsed_path)
            .map_err(|_| revert("path: too long (exceeds BoundedVec limit)"))?;
        let amount_in = <Runtime as pallet_asset_conversion::Config>::AssetBalance::try_from(amount_in)
            .map_err(|_| revert("amountIn: overflow or negative (must fit AssetBalance type)"))?;
        let amount_out_min = <Runtime as pallet_asset_conversion::Config>::AssetBalance::try_from(amount_out_min)
            .map_err(|_| revert("amountOutMin: overflow or negative (must fit AssetBalance type)"))?;
        // Parse send_to
        let send_to: Runtime::AccountId = Runtime::AddressMapping::into_account_id(send_to.into());
        // Map submitter
        let who: Runtime::AccountId = Runtime::AddressMapping::into_account_id(handle.context().caller);
        // Dispatch call
        RuntimeHelper::<Runtime>::try_dispatch(
            handle,
            Some(who).into(),
            pallet_asset_conversion::Call::<Runtime>::swap_exact_tokens_for_tokens {
                path: parsed_path,
                amount_in,
                amount_out_min,
                send_to,
                keep_alive,
            },
        )?;
        Ok(true)
    }

    #[precompile::public("swapTokensForExactTokens(address[],uint256,uint256,address,bool)")]
    fn swap_tokens_for_exact_tokens(
        handle: &mut impl PrecompileHandle,
        path: Vec<Address>,
        amount_out: U256,
        amount_in_max: U256,
        send_to: Address,
        keep_alive: bool,
    ) -> EvmResult<bool>
    where
        <Runtime as pallet_asset_conversion::Config>::AssetBalance: core::convert::TryFrom<U256>,
        <Runtime as pallet_asset_conversion::Config>::MultiAssetId: From<NativeOrAssetId<<Runtime as pallet_asset_conversion::Config>::AssetId>>,
    {
        use core::convert::TryFrom;
        let mut parsed_path = Vec::with_capacity(path.len());
        for (i, addr) in path.iter().enumerate() {
            let asset = match parse_asset_address(addr) {
                Some(PalletAssetConversionNativeOrAssetId::Native) =>
                    NativeOrAssetId::Native,
                Some(PalletAssetConversionNativeOrAssetId::Asset(id)) =>
                    NativeOrAssetId::Asset(id.into()),
                None => return Err(revert(format!("path[{}]: invalid address (must be native or local asset, 20 bytes)", i))),
            };
            parsed_path.push(<Runtime as pallet_asset_conversion::Config>::MultiAssetId::from(asset));
        }
        let parsed_path = BoundedVec::try_from(parsed_path)
            .map_err(|_| revert("path: too long (exceeds BoundedVec limit)"))?;
        let amount_out = <Runtime as pallet_asset_conversion::Config>::AssetBalance::try_from(amount_out)
            .map_err(|_| revert("amountOut: overflow or negative (must fit AssetBalance type)"))?;
        let amount_in_max = <Runtime as pallet_asset_conversion::Config>::AssetBalance::try_from(amount_in_max)
            .map_err(|_| revert("amountInMax: overflow or negative (must fit AssetBalance type)"))?;
        // Parse send_to
        let send_to: Runtime::AccountId = Runtime::AddressMapping::into_account_id(send_to.into());
        // Map submitter
        let who: Runtime::AccountId = Runtime::AddressMapping::into_account_id(handle.context().caller);
        // Dispatch call
        RuntimeHelper::<Runtime>::try_dispatch(
            handle,
            Some(who).into(),
            pallet_asset_conversion::Call::<Runtime>::swap_tokens_for_exact_tokens {
                path: parsed_path,
                amount_out,
                amount_in_max,
                send_to,
                keep_alive,
            },
        )?;
        Ok(true)
    }
}

fn format_lp_token_address<T: Into<u32> + Copy>(lp_token: &T) -> Address {
    let mut bytes = [0u8; 20];
    bytes[0..4].copy_from_slice(&[0xfb, 0xfb, 0xfb, 0xfb]);
    bytes[4..8].copy_from_slice(&[0x00, 0x00, 0x00, 0x00]);
    let asset_id: u32 = (*lp_token).into();
    let asset_bytes = asset_id.to_be_bytes();
    bytes[16..20].copy_from_slice(&asset_bytes);
    Address(H160::from(bytes))
}

fn native_token_address() -> Address {
    let mut bytes = [0u8; 20];
    bytes[18] = 0x08;
    bytes[19] = 0x02;
    Address(H160::from(bytes))
}

fn format_non_native_token_address<T: Into<u32> + Copy>(asset: &T) -> Address {
    let mut bytes = [0u8; 20];
    bytes[0..4].copy_from_slice(&[0xfb, 0xfb, 0xfb, 0xfa]);
    bytes[4..8].copy_from_slice(&[0x00, 0x00, 0x00, 0x00]);
    let asset_id: u32 = (*asset).into();
    let asset_bytes = asset_id.to_be_bytes();
    bytes[16..20].copy_from_slice(&asset_bytes);
    Address(H160::from(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use codec::{Encode, Decode};

    #[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
    enum MockAssetId { Native, Asset(u32) }

    struct MockConverter;
    impl MockConverter {
        fn is_native(asset: &MockAssetId) -> bool {
            matches!(asset, MockAssetId::Native)
        }
        fn try_convert(asset: &MockAssetId) -> Option<u32> {
            if let MockAssetId::Asset(id) = asset { Some(*id) } else { None }
        }
    }

    #[test]
    fn test_address_logic() {
        // Case 1: token0 is native, token1 is non-native
        let native = MockAssetId::Native;
        let asset = MockAssetId::Asset(222);
        let lp_token = 16u32;

        let pair_id = format_lp_token_address(&lp_token);
        let (token0_id, token0_is_native) = if MockConverter::is_native(&native) {
            (native_token_address(), true)
        } else if let Some(asset_id) = MockConverter::try_convert(&native) {
            (format_non_native_token_address(&asset_id), false)
        } else {
            panic!("unsupported asset");
        };
        let (token1_id, token1_is_native) = if MockConverter::is_native(&asset) {
            (native_token_address(), true)
        } else if let Some(asset_id) = MockConverter::try_convert(&asset) {
            (format_non_native_token_address(&asset_id), false)
        } else {
            panic!("unsupported asset");
        };

        println!("Case 1: token0 native, token1 non-native");
        println!("pair_id: 0x{:x}", pair_id.0);
        println!("token0_id: 0x{:x}, token0_is_native: {}", token0_id.0, token0_is_native);
        println!("token1_id: 0x{:x}, token1_is_native: {}", token1_id.0, token1_is_native);

        assert_eq!(format!("{:x}", pair_id.0), "fbfbfbfb00000000000000000000000000000010");
        assert_eq!(format!("{:x}", token0_id.0), "0000000000000000000000000000000000000802");
        assert!(token0_is_native);
        assert_eq!(format!("{:x}", token1_id.0), "fbfbfbfa000000000000000000000000000000de");
        assert!(!token1_is_native);

        // Case 2: token0 is non-native, token1 is native
        let asset = MockAssetId::Asset(222);
        let native = MockAssetId::Native;
        let lp_token = 16u32;

        let pair_id = format_lp_token_address(&lp_token);
        let (token0_id, token0_is_native) = if MockConverter::is_native(&asset) {
            (native_token_address(), true)
        } else if let Some(asset_id) = MockConverter::try_convert(&asset) {
            (format_non_native_token_address(&asset_id), false)
        } else {
            panic!("unsupported asset");
        };
        let (token1_id, token1_is_native) = if MockConverter::is_native(&native) {
            (native_token_address(), true)
        } else if let Some(asset_id) = MockConverter::try_convert(&native) {
            (format_non_native_token_address(&asset_id), false)
        } else {
            panic!("unsupported asset");
        };

        println!("Case 2: token0 non-native, token1 native");
        println!("pair_id: 0x{:x}", pair_id.0);
        println!("token0_id: 0x{:x}, token0_is_native: {}", token0_id.0, token0_is_native);
        println!("token1_id: 0x{:x}, token1_is_native: {}", token1_id.0, token1_is_native);

        assert_eq!(format!("{:x}", pair_id.0), "fbfbfbfb00000000000000000000000000000010");
        assert_eq!(format!("{:x}", token0_id.0), "fbfbfbfa000000000000000000000000000000de");
        assert!(!token0_is_native);
        assert_eq!(format!("{:x}", token1_id.0), "0000000000000000000000000000000000000802");
        assert!(token1_is_native);

        // Case 3: both token0 and token1 are non-native
        let asset0 = MockAssetId::Asset(111);
        let asset1 = MockAssetId::Asset(222);
        let lp_token = 16u32;

        let pair_id = format_lp_token_address(&lp_token);
        let (token0_id, token0_is_native) = if MockConverter::is_native(&asset0) {
            (native_token_address(), true)
        } else if let Some(asset_id) = MockConverter::try_convert(&asset0) {
            (format_non_native_token_address(&asset_id), false)
        } else {
            panic!("unsupported asset");
        };
        let (token1_id, token1_is_native) = if MockConverter::is_native(&asset1) {
            (native_token_address(), true)
        } else if let Some(asset_id) = MockConverter::try_convert(&asset1) {
            (format_non_native_token_address(&asset_id), false)
        } else {
            panic!("unsupported asset");
        };

        println!("Case 3: both token0 and token1 non-native");
        println!("pair_id: 0x{:x}", pair_id.0);
        println!("token0_id: 0x{:x}, token0_is_native: {}", token0_id.0, token0_is_native);
        println!("token1_id: 0x{:x}, token1_is_native: {}", token1_id.0, token1_is_native);

        assert_eq!(format!("{:x}", pair_id.0), "fbfbfbfb00000000000000000000000000000010");
        assert_eq!(format!("{:x}", token0_id.0), "fbfbfbfa0000000000000000000000000000006f");
        assert!(!token0_is_native);
        assert_eq!(format!("{:x}", token1_id.0), "fbfbfbfa000000000000000000000000000000de");
        assert!(!token1_is_native);
    }

    #[test]
    fn test_next_lp_token_address() {
        // Next LP token assetId is 2
        let next_lp_token = 2u32;
        let next_lp_token_addr = format_lp_token_address(&next_lp_token);
        println!("next_lp_token_addr: 0x{:x}", next_lp_token_addr.0);
        assert_eq!(format!("{:x}", next_lp_token_addr.0), "fbfbfbfb00000000000000000000000000000002");
    }

    #[test]
    fn test_parse_asset_address_native() {
        // Native address: 0x0000000000000000000000000000000000000802
        let native_bytes = [0u8; 18].into_iter().chain([0x08, 0x02]).collect::<Vec<_>>();
        let mut arr = [0u8; 20];
        arr.copy_from_slice(&native_bytes);
        let addr = Address(H160::from(arr));
        let result = parse_asset_address(&addr);
        assert_eq!(result, Some(PalletAssetConversionNativeOrAssetId::Native));
    }

    #[test]
    fn test_parse_asset_address_local_asset() {
        // Local asset address: 0xfBFBfbFA000000000000000000000000000000de (asset id 222)
        let mut arr = [0u8; 20];
        arr[0..4].copy_from_slice(&[0xfb, 0xfb, 0xfb, 0xfa]);
        arr[16..20].copy_from_slice(&222u32.to_be_bytes());
        let addr = Address(H160::from(arr));
        let result = parse_asset_address(&addr);
        assert_eq!(result, Some(PalletAssetConversionNativeOrAssetId::Asset(222)));
    }

    #[test]
    fn test_parse_asset_address_invalid() {
        // Invalid address (random)
        let arr = [0x11u8; 20];
        let addr = Address(H160::from(arr));
        let result = parse_asset_address(&addr);
        assert_eq!(result, None);
    }

    // TODO: Add a mock runtime and test create_pool logic if possible
} 