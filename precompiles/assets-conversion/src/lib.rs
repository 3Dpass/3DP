// Copyright 2025 3Dpass
// This file is part of 3Dpass.

#![cfg_attr(not(feature = "std"), no_std)]

use precompile_utils::prelude::*;
use sp_core::{H160, U256};
use sp_std::{marker::PhantomData, vec::Vec};
use pallet_evm::AddressMapping;
use frame_support::BoundedVec;
use scale_info::prelude::format;

use pallet_asset_conversion::{Pools, NativeOrAssetId};

// Event selectors for Uniswap V2 compatibility
pub const SELECTOR_LOG_SWAP: [u8; 32] = keccak256!("Swap(address,uint256,uint256,uint256,uint256,address)");
pub const SELECTOR_LOG_MINT: [u8; 32] = keccak256!("Mint(address,uint256,uint256)");
pub const SELECTOR_LOG_BURN: [u8; 32] = keccak256!("Burn(address,uint256,uint256,address)");
pub const SELECTOR_LOG_SYNC: [u8; 32] = keccak256!("Sync(uint112,uint112)");
pub const SELECTOR_LOG_PAIR_CREATED: [u8; 32] = keccak256!("PairCreated(address,address,address,uint256)");

// Note: SELECTOR_QUERY_PAIRS was removed as it's not used in the Rust code

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
    Runtime: pallet_evm::Config + pallet_asset_conversion::Config + pallet_timestamp::Config,
    <Runtime as pallet_asset_conversion::Config>::AssetId: Into<u32> + Copy + Ord + From<u32>,
    <Runtime as pallet_asset_conversion::Config>::PoolAssetId: Into<u32> + Copy,
    Runtime::Call: frame_support::dispatch::Dispatchable<PostInfo = frame_support::dispatch::PostDispatchInfo> + frame_support::dispatch::GetDispatchInfo,
    <Runtime::Call as frame_support::dispatch::Dispatchable>::Origin: From<Option<Runtime::AccountId>>,
    Runtime::Call: From<pallet_asset_conversion::Call<Runtime>>,
    <Runtime as pallet_asset_conversion::Config>::MultiAssetId: From<NativeOrAssetId<<Runtime as pallet_asset_conversion::Config>::AssetId>>,
    <Runtime as pallet_asset_conversion::Config>::AssetBalance: core::convert::TryFrom<U256> + Into<U256> + precompile_utils::EvmData,
    sp_core::U256: From<<Runtime as pallet_asset_conversion::Config>::AssetBalance>,
    <Runtime as pallet_timestamp::Config>::Moment: Into<U256>,
{
    #[precompile::public("nextPoolAssetId()")]
    fn next_pool_asset_id(_handle: &mut impl PrecompileHandle) -> EvmResult<(bool, u32)> {
        let opt = pallet_asset_conversion::NextPoolAssetId::<Runtime>::get();
        match opt {
            Some(id) => Ok((true, id.into())),
            None => Ok((false, 0)),
        }
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
    ) -> EvmResult<(U256, U256, U256)>
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
        let mint_to_evm = mint_to; // Keep original EVM address for events
        let mint_to: Runtime::AccountId = Runtime::AddressMapping::into_account_id(mint_to.into());
        // Map submitter
        let who: Runtime::AccountId = Runtime::AddressMapping::into_account_id(handle.context().caller);
        
        // Record gas cost for event emission
        handle.record_log_costs_manual(3, 32)?;
        
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
        
        // Return Uniswap V2 compatible amounts (amountA, amountB, liquidity)
        // For now, return the desired amounts and a placeholder liquidity value
        // In a full implementation, you'd calculate the actual amounts and liquidity
        let liquidity = amount1_desired; // Placeholder: should calculate actual LP tokens minted
        
        // Emit Mint event (Uniswap V2 compatibility)
        log3(
            handle.context().address,
            SELECTOR_LOG_MINT,
            mint_to_evm.0,
            mint_to_evm.0,
            EvmDataWriter::new()
                .write(mint_to_evm)
                .write(U256::from(amount1_desired))
                .write(U256::from(amount2_desired))
                .build(),
        )
        .record(handle)?;
        
        Ok((amount1_desired.into(), amount2_desired.into(), liquidity.into()))
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
    ) -> EvmResult<(U256, U256)>
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
        let withdraw_to_evm = withdraw_to; // Keep original EVM address for events
        let withdraw_to: Runtime::AccountId = Runtime::AddressMapping::into_account_id(withdraw_to.into());
        // Map submitter
        let who: Runtime::AccountId = Runtime::AddressMapping::into_account_id(handle.context().caller);
        
        // Record gas cost for event emission
        handle.record_log_costs_manual(3, 32)?;
        
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
        
        // Return Uniswap V2 compatible amounts (amountA, amountB)
        // For now, return the minimum receive amounts
        // In a full implementation, you'd calculate the actual amounts received
        
        // Emit Burn event (Uniswap V2 compatibility)
        log3(
            handle.context().address,
            SELECTOR_LOG_BURN,
            withdraw_to_evm.0,
            withdraw_to_evm.0,
            EvmDataWriter::new()
                .write(withdraw_to_evm)
                .write(U256::from(amount1_min_receive))
                .write(U256::from(amount2_min_receive))
                .build(),
        )
        .record(handle)?;
        
        Ok((amount1_min_receive.into(), amount2_min_receive.into()))
    }

    #[precompile::public("swapExactTokensForTokens(uint256,uint256,address[],address,uint256)")]
    fn swap_exact_tokens_for_tokens(
        handle: &mut impl PrecompileHandle,
        amount_in: U256,
        amount_out_min: U256,
        path: Vec<Address>,
        send_to: Address,
        deadline: U256,
    ) -> EvmResult<Vec<U256>>
    where
        <Runtime as pallet_asset_conversion::Config>::AssetBalance: core::convert::TryFrom<U256>,
        <Runtime as pallet_asset_conversion::Config>::MultiAssetId: From<NativeOrAssetId<<Runtime as pallet_asset_conversion::Config>::AssetId>>,
    {
        use core::convert::TryFrom;
        
        // Check deadline (Uniswap V2 compatibility)
        let current_timestamp = pallet_timestamp::Pallet::<Runtime>::now().into();
        if deadline < current_timestamp {
            return Err(revert("Transaction deadline exceeded"));
        }
        
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
        let send_to_evm = send_to; // Keep original EVM address for events
        let send_to: Runtime::AccountId = Runtime::AddressMapping::into_account_id(send_to.into());
        // Map submitter
        let who: Runtime::AccountId = Runtime::AddressMapping::into_account_id(handle.context().caller);
        
        // Always set keep_alive to true for Uniswap V2 compatibility
        let keep_alive = true;
        
        // Record gas cost for event emission
        handle.record_log_costs_manual(3, 32)?;
        
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
        
        // Return amounts array (Uniswap V2 compatibility)
        // For now, return a simple array with amount_in and amount_out_min
        // In a full implementation, you'd calculate the actual amounts for each hop
        let mut amounts = Vec::new();
        amounts.push(amount_in.into());
        amounts.push(amount_out_min.into());
        
        // Emit Swap event (Uniswap V2 compatibility)
        log3(
            handle.context().address,
            SELECTOR_LOG_SWAP,
            send_to_evm.0,
            send_to_evm.0,
            EvmDataWriter::new()
                .write(send_to_evm)
                .write(U256::from(amount_in))
                .write(U256::zero()) // amount0In
                .write(U256::from(amount_out_min)) // amount1Out
                .write(U256::zero()) // amount0Out
                .build(),
        )
        .record(handle)?;
        
        Ok(amounts)
    }

    #[precompile::public("swapTokensForExactTokens(uint256,uint256,address[],address,uint256)")]
    fn swap_tokens_for_exact_tokens(
        handle: &mut impl PrecompileHandle,
        amount_out: U256,
        amount_in_max: U256,
        path: Vec<Address>,
        send_to: Address,
        deadline: U256,
    ) -> EvmResult<Vec<U256>>
    where
        <Runtime as pallet_asset_conversion::Config>::AssetBalance: core::convert::TryFrom<U256>,
        <Runtime as pallet_asset_conversion::Config>::MultiAssetId: From<NativeOrAssetId<<Runtime as pallet_asset_conversion::Config>::AssetId>>,
    {
        use core::convert::TryFrom;
        
        // Check deadline (Uniswap V2 compatibility)
        let current_timestamp = pallet_timestamp::Pallet::<Runtime>::now().into();
        if deadline < current_timestamp {
            return Err(revert("Transaction deadline exceeded"));
        }
        
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
        let send_to_evm = send_to; // Keep original EVM address for events
        let send_to: Runtime::AccountId = Runtime::AddressMapping::into_account_id(send_to.into());
        // Map submitter
        let who: Runtime::AccountId = Runtime::AddressMapping::into_account_id(handle.context().caller);
        
        // Always set keep_alive to true for Uniswap V2 compatibility
        let keep_alive = true;
        
        // Record gas cost for event emission
        handle.record_log_costs_manual(3, 32)?;
        
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
        
        // Return amounts array (Uniswap V2 compatibility)
        // For now, return a simple array with amount_in_max and amount_out
        // In a full implementation, you'd calculate the actual amounts for each hop
        let mut amounts = Vec::new();
        amounts.push(amount_in_max.into());
        amounts.push(amount_out.into());
        
        // Emit Swap event (Uniswap V2 compatibility)
        log3(
            handle.context().address,
            SELECTOR_LOG_SWAP,
            send_to_evm.0,
            send_to_evm.0,
            EvmDataWriter::new()
                .write(send_to_evm)
                .write(U256::from(amount_in_max))
                .write(U256::zero()) // amount0In
                .write(U256::from(amount_out)) // amount1Out
                .write(U256::zero()) // amount0Out
                .build(),
        )
        .record(handle)?;
        
        Ok(amounts)
    }

    #[precompile::public("allPairsLength()")]
    fn all_pairs_length(_handle: &mut impl PrecompileHandle) -> EvmResult<U256> {
        // Count the total number of pairs
        let mut count = 0u32;
        for _ in Pools::<Runtime>::iter() {
            count += 1;
        }
        Ok(count.into())
    }

    #[precompile::public("allPairs(uint256)")]
    fn all_pairs(
        _handle: &mut impl PrecompileHandle,
        index: U256,
    ) -> EvmResult<Address> {
        let index_u32: u32 = index.try_into().map_err(|_| revert("index: overflow (must fit u32)"))?;
        
        // Find the pair at the specified index
        let mut current_index = 0u32;
        for (_pool_id, pool_info) in Pools::<Runtime>::iter() {
            if current_index == index_u32 {
                let lp_token = pool_info.lp_token;
                let pair_address = format_lp_token_address(&lp_token);
                return Ok(pair_address);
            }
            current_index += 1;
        }
        
        Err(revert("index: out of bounds"))
    }

    #[precompile::public("getPair(address,address)")]
    fn get_pair(
        _handle: &mut impl PrecompileHandle,
        token_a: Address,
        token_b: Address,
    ) -> EvmResult<Address> {
        // Parse token addresses
        let asset_a: <Runtime as pallet_asset_conversion::Config>::MultiAssetId = match parse_asset_address(&token_a) {
            Some(PalletAssetConversionNativeOrAssetId::Native) =>
                NativeOrAssetId::Native.into(),
            Some(PalletAssetConversionNativeOrAssetId::Asset(id)) =>
                NativeOrAssetId::Asset(id.into()).into(),
            None => return Err(revert("tokenA: invalid address (must be native or local asset, 20 bytes)")),
        };
        let asset_b: <Runtime as pallet_asset_conversion::Config>::MultiAssetId = match parse_asset_address(&token_b) {
            Some(PalletAssetConversionNativeOrAssetId::Native) =>
                NativeOrAssetId::Native.into(),
            Some(PalletAssetConversionNativeOrAssetId::Asset(id)) =>
                NativeOrAssetId::Asset(id.into()).into(),
            None => return Err(revert("tokenB: invalid address (must be native or local asset, 20 bytes)")),
        };
        
        // Check both orderings of the pair
        let pool_id_ab = (asset_a.clone(), asset_b.clone());
        let pool_id_ba = (asset_b, asset_a);
        
        if let Some(pool_info) = Pools::<Runtime>::get(pool_id_ab) {
            let pair_address = format_lp_token_address(&pool_info.lp_token);
            return Ok(pair_address);
        }
        
        if let Some(pool_info) = Pools::<Runtime>::get(pool_id_ba) {
            let pair_address = format_lp_token_address(&pool_info.lp_token);
            return Ok(pair_address);
        }
        
        // Return zero address if pair doesn't exist (Uniswap V2 behavior)
        Ok(Address(H160::zero()))
    }

    #[precompile::public("createPair(address,address)")]
    fn create_pair(
        handle: &mut impl PrecompileHandle,
        token_a: Address,
        token_b: Address,
    ) -> EvmResult<Address> {
        // Parse token addresses
        let asset_a: <Runtime as pallet_asset_conversion::Config>::MultiAssetId = match parse_asset_address(&token_a) {
            Some(PalletAssetConversionNativeOrAssetId::Native) =>
                NativeOrAssetId::Native.into(),
            Some(PalletAssetConversionNativeOrAssetId::Asset(id)) =>
                NativeOrAssetId::Asset(id.into()).into(),
            None => return Err(revert("tokenA: invalid address (must be native or local asset, 20 bytes)")),
        };
        let asset_b: <Runtime as pallet_asset_conversion::Config>::MultiAssetId = match parse_asset_address(&token_b) {
            Some(PalletAssetConversionNativeOrAssetId::Native) =>
                NativeOrAssetId::Native.into(),
            Some(PalletAssetConversionNativeOrAssetId::Asset(id)) =>
                NativeOrAssetId::Asset(id.into()).into(),
            None => return Err(revert("tokenB: invalid address (must be native or local asset, 20 bytes)")),
        };
        
        // Map submitter
        let who: Runtime::AccountId = Runtime::AddressMapping::into_account_id(handle.context().caller);
        
        // Record gas cost for event emission
        handle.record_log_costs_manual(3, 32)?;
        
        // Dispatch call to create pool
        RuntimeHelper::<Runtime>::try_dispatch(
            handle,
            Some(who).into(),
            pallet_asset_conversion::Call::<Runtime>::create_pool {
                asset1: asset_a.clone(),
                asset2: asset_b.clone(),
            },
        )?;
        
        // Get the created pair address
        let pool_id = (asset_a, asset_b);
        let pair_address = if let Some(pool_info) = Pools::<Runtime>::get(pool_id) {
            format_lp_token_address(&pool_info.lp_token)
        } else {
            // Fallback: try reverse order
            let pool_id_reverse = (asset_b, asset_a);
            if let Some(pool_info) = Pools::<Runtime>::get(pool_id_reverse) {
                format_lp_token_address(&pool_info.lp_token)
            } else {
                return Err(revert("Failed to create pair"));
            }
        };
        
        // Get total pair count for the event
        let mut pair_count = 0u32;
        for _ in Pools::<Runtime>::iter() {
            pair_count += 1;
        }
        
        // Emit PairCreated event (Uniswap V2 compatibility)
        log3(
            handle.context().address,
            SELECTOR_LOG_PAIR_CREATED,
            handle.context().caller,
            handle.context().caller,
            EvmDataWriter::new()
                .write(token_a)
                .write(token_b)
                .write(pair_address)
                .write(U256::from(<Runtime as pallet_asset_conversion::Config>::AssetBalance::from(pair_count)))
                .build(),
        )
        .record(handle)?;
        
        Ok(pair_address)
    }
}

#[cfg(test)]
mod mock;

fn format_lp_token_address<T: Into<u32> + Copy>(lp_token: &T) -> Address {
    let mut bytes = [0u8; 20];
    bytes[0..4].copy_from_slice(&[0xfb, 0xfb, 0xfb, 0xfb]);
    bytes[4..8].copy_from_slice(&[0x00, 0x00, 0x00, 0x00]);
    let asset_id: u32 = (*lp_token).into();
    let asset_bytes = asset_id.to_be_bytes();
    bytes[16..20].copy_from_slice(&asset_bytes);
    Address(H160::from(bytes))
} 