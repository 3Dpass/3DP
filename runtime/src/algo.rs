use codec::Encode;
use sp_core::H256;
use sp_consensus_poscan::decompress_obj;
use sp_std::vec;

use crate::Vec;

use sp_consensus_poscan::{
    POSCAN_ALGO_GRID2D_V2,
    POSCAN_ALGO_GRID2D_V3,
    POSCAN_ALGO_GRID2D_V3_1,
    POSCAN_ALGO_GRID2D_V3A,
};


pub fn get_obj_hashes_wasm(ver: &[u8; 16], data: &[u8], pre: &H256, depth: usize, patch_rot: bool) -> Vec<H256> {
    let mut buf: Vec<H256> = Vec::new();
    let mut obj = data.to_vec();
    if data[..4] == vec![b'l', b'z', b's', b's'] {
        obj = decompress_obj(&data[4..]);
    }
    let grid_size = 8;
    let (alg_type, n_sect) =
        match *ver {
            POSCAN_ALGO_GRID2D_V2 => (p3d::AlgoType::Grid2dV2, 12),
            POSCAN_ALGO_GRID2D_V3 |
            POSCAN_ALGO_GRID2D_V3_1 => (p3d::AlgoType::Grid2dV3, 12),
            POSCAN_ALGO_GRID2D_V3A => (p3d::AlgoType::Grid2dV3a, 12),
            _ => (p3d::AlgoType::Grid2d, 66),
        };

    let mut rot: Option<[u8; 4]> = None;
    let mut maybe_rot: Option<[u8;32]> = if *pre == H256::default() { None } else { pre.encode()[0..32].try_into().ok() };
    if let Some(r) = &mut maybe_rot {
        if patch_rot {
            for a in r[3..].iter() {
                // exclude rotation less than 20
                if *a > 20 {
                    r[3] = a.clone();
                    break;
                }
            }
        }
        rot = Some(r[0..4].try_into().unwrap());
    }
    let res = p3d::p3d_process_n(&obj, alg_type, depth, grid_size, n_sect, rot);

    match res {
        Ok(v) => {
            for item in v {
                if let Ok(h) = array_bytes::hex_n_into::<&str, H256, 32>(&item) {
                    buf.push(h);
                }
            }
        },
        Err(_) => {
            return Vec::new()
        },
    }

    buf
}