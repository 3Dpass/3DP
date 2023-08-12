#![cfg_attr(not(feature = "std"), no_std)]

use sp_core::{H256, Encode, Decode};
use sp_std::vec::Vec;
use sp_runtime_interface::runtime_interface;

#[cfg(feature = "std")]
use lazy_static::lazy_static;

#[cfg(feature = "std")]
use sp_std::{
	sync::Arc,
	boxed::Box,
	convert::TryInto,
	str::FromStr,
};
#[cfg(feature = "std")]
use sp_consensus_poscan::{POSCAN_ALGO_GRID2D_V2, POSCAN_ALGO_GRID2D_V3, POSCAN_ALGO_GRID2D_V3_1};

#[cfg(feature = "std")]
use lzss::{Lzss, SliceReader, VecWriter};

#[cfg(feature = "std")]
use parking_lot::Mutex;
#[cfg(feature = "std")]
use sp_blockchain::HeaderBackend;
#[cfg(feature = "std")]
use sp_runtime::generic;
#[cfg(feature = "std")]
use sp_runtime::traits::BlakeTwo256;
#[cfg(feature = "std")]
use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;
#[cfg(feature = "std")]
use sp_runtime::generic::DigestItem;

#[cfg(feature = "std")]
pub type BlockNumber = u32;
#[cfg(feature = "std")]
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
#[cfg(feature = "std")]
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
#[cfg(feature = "std")]
pub type BlockId = generic::BlockId<Block>;

#[cfg(feature = "std")]
lazy_static! {
    pub static ref CLIENT: Mutex<Option<
			(
				Box<Arc<dyn Send + Sync + HeaderBackend<Block>>>,
				bool,
			)>> = {
        Mutex::new(None)
    };
}

#[derive(Encode, Decode, Clone)]
pub enum Error {
	NoClient,
	NoAuthor,
	NoDigestItems,
	InvalidDigestItem,
	NoParentBlock,
}

#[runtime_interface]
pub trait HashableObject {
	fn calc_obj_hashes(ver: &[u8; 16], data: &[u8], pre: &H256) -> Vec<H256> {
		get_obj_hashes(ver, data, pre)
	}

	fn is_light() -> Result<bool, Error> {
		if CLIENT.lock().is_none() {
			Err(Error::NoClient)
		}
		else {
			let client = CLIENT.lock().clone().unwrap();
			Ok(client.1)
		}
	}

	fn prev_mining_data() -> Result<(u32, H256, H256, Vec<u8>, Vec<H256>, Vec<u8>), Error> {
		if CLIENT.lock().is_none() {
			Err(Error::NoClient)
		}
		else {
			let client = CLIENT.lock().clone().unwrap();
			let prev_num = client.0.info().best_number;
			let block_id = BlockId::number(prev_num);

			match client.0.header(block_id) {
				Ok(maybe_prev_block) =>
					match maybe_prev_block {
						Some(signed_block) => {
							let items = signed_block.digest.logs();
							let n = items.len();
							if n > 3 {
								let parent_hash = signed_block.parent_hash;
								let mut block = signed_block.clone();
								// Remove 3 items: other, other, seal
								let d3 = block.digest.pop().unwrap();
								let d2 = block.digest.pop().unwrap();
								let d1 = block.digest.pop().unwrap();
								let pre_hash = block.hash();
								block.digest.push(d1);
								block.digest.push(d2);
								block.digest.push(d3);

								match (items[n - 2].clone(), items[n - 1].clone()) {
									(DigestItem::Other(h), DigestItem::Other(obj)) => {
										let hashes: Vec<H256> = h[16..].chunks(32).map(H256::from_slice).collect();
										let algo_id: [u8; 16] = h[0..16].try_into().unwrap();
										Ok((prev_num, pre_hash, parent_hash, algo_id.to_vec(), hashes, obj.to_vec()))
									},
									_ => {
										println!("No hashes in digest items!!!");
										Err(Error::InvalidDigestItem)
									},
								}
							} else {
								println!("No digest items!!!");
								Err(Error::NoDigestItems)
							}
						},
						None => {
							println!("No signed block!!!");
							Err(Error::NoParentBlock)
						},
					},
				Err(_e) => {
					println!("NoParentBlock!!!");
					Err(Error::NoParentBlock)
				},
			}
		}
	}

	fn estimate_obj(ver: &[u8; 16], data: &[u8]) -> Option<(u128, Vec<H256>)> {
		use std::thread;
		use std::time::{Duration as TimeDuration, SystemTime};

		let v = ver.clone();
		let d = Vec::from(data);
		let handler = thread::spawn(move || {
			let bgn = SystemTime::now();
			let hashes = get_obj_hashes(&v, &d, &H256::default());
			// *res.lock() = 1;
			(SystemTime::now().duration_since(bgn).unwrap(), hashes)
		});

		// let mut bgn = SystemTime::now();
		// let mut t = TimeDuration::default();
		for _ in 0..10 {
			thread::sleep(TimeDuration::from_secs(1));
			if handler.is_finished() {
				let res = handler.join().unwrap();
				return Some((res.0.as_millis(), res.1))
			}
			// else {
			// 	_t = SystemTime::now().duration_since(bgn).unwrap();
			// }
		}
		None
	}
}

#[cfg(feature = "std")]
pub fn get_obj_hashes(ver: &[u8; 16], data: &[u8], pre: &H256) -> Vec<H256> {
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
			_ => (p3d::AlgoType::Grid2d, 66),
		};

	let rot = if *pre == H256::default() { None } else { pre.encode()[0..4].try_into().ok() };
	let res = p3d::p3d_process(&obj, alg_type, grid_size, n_sect, rot);

	match res {
		Ok(v) => {
			for item in v {
				let h = H256::from_str(item.as_str()).unwrap();
				buf.push(h);
			}
		},
		Err(_) => {
			return Vec::new()
		},
	}

	buf
}

#[cfg(feature = "std")]
pub fn compress_obj(obj: &[u8]) -> Vec<u8> {
	type MyLzss = Lzss<10, 4, 0x20, { 1 << 10 }, { 2 << 10 }>;
	let result = MyLzss::compress(
		SliceReader::new(obj),
		VecWriter::with_capacity(4096),
	);

	result.unwrap()
}

#[cfg(feature = "std")]
pub fn decompress_obj(obj: &[u8]) -> Vec<u8> {
	type MyLzss = Lzss<10, 4, 0x20, { 1 << 10 }, { 2 << 10 }>;
	let result = MyLzss::decompress(
		SliceReader::new(obj),
		VecWriter::with_capacity(4096),
	);

	result.unwrap()
}
