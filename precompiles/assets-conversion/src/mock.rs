// Stubs for test-only address helpers
use precompile_utils::prelude::Address;
use sp_core::H160;
fn native_token_address() -> Address {
    // Use the same constant as in lib.rs
    use crate::NATIVE_TOKEN_ADDRESS;
    Address(H160::from(NATIVE_TOKEN_ADDRESS))
}
fn format_non_native_token_address(asset_id: &u32) -> Address {
    // Mimic the local asset address format: [0xfb, 0xfb, 0xfb, 0xfa] + ... + asset_id (last 4 bytes)
    let mut arr = [0u8; 20];
    arr[0..4].copy_from_slice(&[0xfb, 0xfb, 0xfb, 0xfa]);
    arr[16..20].copy_from_slice(&asset_id.to_be_bytes());
    Address(H160::from(arr))
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate hex;
    use crate::{
        SELECTOR_LOG_SWAP,
        SELECTOR_LOG_MINT,
        SELECTOR_LOG_BURN,
        SELECTOR_LOG_SYNC,
        SELECTOR_LOG_PAIR_CREATED,
        format_lp_token_address,
    };
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
        let result = crate::parse_asset_address(&addr);
        assert_eq!(result, Some(crate::PalletAssetConversionNativeOrAssetId::Native));
    }

    #[test]
    fn test_parse_asset_address_local_asset() {
        // Local asset address: 0xfBFBfbFA000000000000000000000000000000de (asset id 222)
        let mut arr = [0u8; 20];
        arr[0..4].copy_from_slice(&[0xfb, 0xfb, 0xfb, 0xfa]);
        arr[16..20].copy_from_slice(&222u32.to_be_bytes());
        let addr = Address(H160::from(arr));
        let result = crate::parse_asset_address(&addr);
        assert_eq!(result, Some(crate::PalletAssetConversionNativeOrAssetId::Asset(222)));
    }

    #[test]
    fn test_parse_asset_address_invalid() {
        // Invalid address (random)
        let arr = [0x11u8; 20];
        let addr = Address(H160::from(arr));
        let result = crate::parse_asset_address(&addr);
        assert_eq!(result, None);
    }

    #[test]
    pub fn print_event_selectors() {
        println!("Swap: 0x{}", hex::encode(SELECTOR_LOG_SWAP));
        println!("Mint: 0x{}", hex::encode(SELECTOR_LOG_MINT));
        println!("Burn: 0x{}", hex::encode(SELECTOR_LOG_BURN));
        println!("Sync: 0x{}", hex::encode(SELECTOR_LOG_SYNC));
        println!("PairCreated: 0x{}", hex::encode(SELECTOR_LOG_PAIR_CREATED));
    }
} 