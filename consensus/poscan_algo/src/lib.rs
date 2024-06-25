#![cfg_attr(not(feature = "std"), no_std)]

use sp_core::{H256, Encode, Decode};
use sp_std::vec::Vec;
use sp_runtime_interface::runtime_interface;
use sp_core::offchain::Duration;

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
use sp_consensus_poscan::{
	POSCAN_ALGO_GRID2D_V2,
	POSCAN_ALGO_GRID2D_V3,
	POSCAN_ALGO_GRID2D_V3_1,
	POSCAN_ALGO_GRID2D_V3A,
};

#[cfg(feature = "std")]
use lzss::{Lzss, SliceReader, VecWriter};

#[cfg(feature = "std")]
use parking_lot::Mutex;
#[cfg(feature = "std")]
use sp_blockchain::HeaderBackend;
#[cfg(feature = "std")]
use std::thread;
#[cfg(feature = "std")]
use std::time::{Duration as TimeDuration, SystemTime};
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

#[derive(Encode, Decode, Clone, Debug)]
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
		get_obj_hashes(ver, data, pre, false)
	}

	fn calc_obj_hashes_n(ver: &[u8; 16], data: &[u8], pre: &H256, n: u32) -> Vec<H256> {
		get_obj_hashes_n(ver, data, pre, n as usize, false)
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

	fn fake_test() -> bool {
		return true
	}

	fn try_call() -> bool {
		return true
	}

	fn try_call_128() -> bool { return true }

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

	fn estimate_obj(ver: &[u8; 16], data: &[u8], timeout: Duration) -> Option<(u128, Vec<H256>)> {
		let v = ver.clone();
		let d = Vec::from(data);
		let handler = thread::spawn(move || {
			let bgn = SystemTime::now();
			let hashes = get_obj_hashes(&v, &d, &H256::default(), false);
			// *res.lock() = 1;
			(SystemTime::now().duration_since(bgn).unwrap(), hashes)
		});

		let n = timeout.millis() / 100;
		for _ in 0..n {
			thread::sleep(TimeDuration::from_millis(100));
			if handler.is_finished() {
				let res = handler.join().unwrap();
				return Some((res.0.as_millis(), res.1))
			}
		}
		None
	}

	fn check_object(alg_id: &[u8;16], obj: &Vec<u8>, hashes: &Vec<H256>) -> bool {
		self::check_obj(alg_id, obj, hashes)
	}
}

#[cfg(feature = "std")]
pub fn get_obj_hashes(ver: &[u8; 16], data: &[u8], pre: &H256, patch_rot: bool) -> Vec<H256> {
	get_obj_hashes_n(ver, data, pre, 10, patch_rot)
}

#[cfg(feature = "std")]
pub fn get_obj_hashes_n(ver: &[u8; 16], data: &[u8], pre: &H256, depth: usize, patch_rot: bool) -> Vec<H256> {
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


#[cfg(feature = "std")]
use ndarray::{Array1, Array2, Array3, ArrayView1, ArrayView2, Axis, arr1, arr2};
#[cfg(feature = "std")]
use tri_mesh::prelude::*;
#[cfg(feature = "std")]
use num_traits::abs;
#[cfg(feature = "std")]
type VectorTriangles = Array3<f64>;
#[cfg(feature = "std")]
use obj::{load_obj, Obj, Vertex};


#[cfg(feature = "std")]
fn cross(triangles: &VectorTriangles) -> Array2<f64> {
	let dims = triangles.dim();
	let mut d = Array3::zeros((dims.0, 2, dims.1));

	for (i, m) in triangles.axis_iter(Axis(0)).enumerate() {
		for (j, p) in m.axis_iter(Axis(1)).enumerate() {
			d[[i, 0, j]] = p[1] - p[0];
			d[[i, 1, j]] = p[2] - p[1];
		}
	}

	let mut cr = Array2::zeros((dims.0, dims.1));
	for (i, m) in d.axis_iter(Axis(0)).enumerate() {
		let a: ArrayView1<f64> = m.slice(s![0, ..]);
		let b: ArrayView1<f64> = m.slice(s![1, ..]);
		cr[[i, 0]] = a[1] * b[2] - a[2] * b[1];
		cr[[i, 1]] = a[2] * b[0] - a[0] * b[2];
		cr[[i, 2]] = a[0] * b[1] - a[1] * b[0];
	}
	cr
}

#[cfg(feature = "std")]
fn volume(triangles: &Array3<f64>) -> f64 {
	let p0: ArrayView2<f64> = triangles.slice(s![0.., 0, 0..]);
	let p1: ArrayView2<f64> = triangles.slice(s![0.., 1, 0..]);
	let p2: ArrayView2<f64> = triangles.slice(s![0.., 2, 0..]);

	let f1 = &p0 + &p1 + &p2;
	let f2 = &p0 * &p0 + &p1 * &p1 + &p0 * &p1 + &p1 * &f1;
	let f3 = &p0 * &p0 * &p0 + &p0 * &p0 * &p1 + &p0 * &p1 * &p1 + &p1 * &p1 * &p1 + &p2 * &f2;

	let g0 = &(&f2 + &(&(&p0 + &f1) * &p0));
	let g1 = &(&f2 + &(&(&p1 + &f1) * &p1));
	let g2 = &(&f2 + &(&(&p2 + &f1) * &p2));

	let d = f1.nrows();
	let mut integral: Array2<f64> = Array2::zeros((10, d));

	let crosses = cross(&triangles);

	integral.slice_mut(s![0..1, ..]).assign(&(&crosses.slice(s![.., 0]) * &f1.slice(s![.., 0])));
	integral.slice_mut(s![1..4, ..]).assign(&(&crosses * &f2).t().slice(s![.., ..]));
	integral.slice_mut(s![4..7, ..]).assign(&(&crosses * &f3).t().slice(s![.., ..]));

	for i in 0..3 {
		let triangle_i = (i + 1) % 3;
		integral.slice_mut(s![i+7, ..]).assign(
			&(&crosses.slice(s![.., i]) * &(
				&triangles.slice(s![.., 0, triangle_i]) * &g0.slice(s![.., i]) +
					&triangles.slice(s![.., 0, triangle_i]) * &g1.slice(s![.., i]) +
					&triangles.slice(s![.., 0, triangle_i]) * &g2.slice(s![.., i]))
			)
		);
	}
	let coefficients: Array1<f64> = arr1(&[1. / 6., 1. / 24., 1. / 24., 1. / 24., 1. / 60., 1. / 60., 1. / 60., 1. / 120., 1. / 120., 1. / 120.]);
	let integrated: Array1<f64> = integral.sum_axis(Axis(1)) * coefficients;
	let volume = integrated[0];
	volume
}

#[cfg(feature = "std")]
pub fn check_obj(_alg_id: &[u8;16], obj: &Vec<u8>, _hashes: &Vec<H256>) -> bool {
	use sp_consensus_poscan::decompress_obj;
	use sp_std::vec;

	let mut obj = obj.clone();

	if obj[..4] == vec![b'l', b'z', b's', b's'] {
		log::debug!("poscan: start decompress");
		obj = decompress_obj(&obj[4..]);
		log::debug!("poscan: end decompress");
	}

	let model: Obj<Vertex, u32> = load_obj(obj.as_slice()).unwrap();

	let vertices = model.vertices
		.iter()
		.flat_map(|v| v.position.iter())
		.map(|v| <f64 as NumCast>::from(*v).unwrap())
		.collect();

	let mesh = match MeshBuilder::new()
		.with_indices(model.indices)
		.with_positions(vertices)
		.build() {
		Ok(m) => m,
		Err(_) => return false,
	};

	let mut triangles: Array3<f64> = Array3::zeros((mesh.no_faces(), 3, 3));

	for (i, fid) in mesh.face_iter().enumerate() {
		let vs = mesh.face_vertices(fid);
		let v1 = mesh.vertex_position(vs.0);
		let v2 = mesh.vertex_position(vs.1);
		let v3 = mesh.vertex_position(vs.2);
		triangles.slice_mut(s![i, .., ..])
			.assign(
				&arr2(&[
					[v1.x as f64, v1.y as f64, v1.z as f64],
					[v2.x as f64, v2.y as f64, v2.z as f64],
					[v3.x as f64, v3.y as f64, v3.z as f64],
				]
				));
	}
	log::debug!("poscan: start is_valid()");
	let is_valid = is_valid(&mesh);
	log::debug!("poscan: is_valid: {:#?}", &is_valid);

	is_valid.is_ok() && {
		log::debug!("poscan: start volume");

		let volume = volume(&triangles);
		log::debug!("poscan: volume: {}", &volume);

		log::debug!("poscan: start bound_vol");
		let (p1, p2) = mesh.extreme_coordinates();
		let bound_vol = abs(p2.x - p1.x) * abs(p2.y - p1.y) * abs(p2.z - p1.z);
		log::debug!("poscan: bound_vol: {}", &bound_vol);

		volume > 0.1 * bound_vol
	}
}


#[cfg(feature = "std")]
use tri_mesh::mesh::Error as MeshError;

#[cfg(feature = "std")]
#[macro_use]
extern crate ndarray;

#[cfg(feature = "std")]
pub fn is_valid(mesh: &Mesh) -> Result<(), MeshError>
{
	log::debug!("=== validate 1");
	for vertex_id in mesh.vertex_iter() {
		if let Some(halfedge_id) = mesh.walker_from_vertex(vertex_id).halfedge_id()
		{
			if !mesh.halfedge_iter().any(|he_id| he_id == halfedge_id) {
				return Err(MeshError::MeshIsInvalid {message: format!("Vertex {} points to an invalid halfedge {}", vertex_id, halfedge_id)});
			}
			if mesh.walker_from_vertex(vertex_id).as_twin().vertex_id().unwrap() != vertex_id
			{
				return Err(MeshError::MeshIsInvalid {message: format!("Halfedge {} pointed to by vertex {} does not start in that vertex, but instead in {}", mesh.walker_from_vertex(vertex_id).halfedge_id().unwrap(), vertex_id, mesh.walker_from_vertex(vertex_id).as_twin().vertex_id().unwrap())});
			}
		}
		else {
			return Err(MeshError::MeshIsInvalid {message: format!("Vertex {} does not point to a halfedge", vertex_id)});
		}
	}
	log::debug!("=== validate 2");
	for halfedge_id in mesh.halfedge_iter() {
		let walker = mesh.walker_from_halfedge(halfedge_id);

		if let Some(twin_id) = walker.twin_id()
		{
			if !mesh.halfedge_iter().any(|he_id| he_id == twin_id) {
				return Err(MeshError::MeshIsInvalid {message: format!("Halfedge {} points to an invalid twin halfedge {}", halfedge_id, twin_id)});
			}
			if mesh.walker_from_halfedge(twin_id).twin_id().unwrap() != halfedge_id
			{
				return Err(MeshError::MeshIsInvalid {message: format!("Halfedge twin pointed to by halfedge {} does not point back to halfedge", halfedge_id)});
			}
			if mesh.walker_from_halfedge(twin_id).vertex_id() == walker.vertex_id()
			{
				return Err(MeshError::MeshIsInvalid {message: format!("Invalid orientation: The halfedge {} and its twin halfedge {} points to the same vertex {}", halfedge_id, twin_id, walker.vertex_id().unwrap())});
			}
		}
		else {
			return Err(MeshError::MeshIsInvalid {message: format!("Halfedge {} does not point to a twin halfedge", halfedge_id)});
		}

		if let Some(vertex_id) = walker.vertex_id()
		{
			if !mesh.vertex_iter().any(|vid| vid == vertex_id) {
				return Err(MeshError::MeshIsInvalid {message: format!("Halfedge {} points to an invalid vertex {}", halfedge_id, vertex_id)});
			}
		}
		else {
			return Err(MeshError::MeshIsInvalid {message: format!("Halfedge {} does not point to a vertex", halfedge_id)});
		}

		if let Some(face_id) = walker.face_id()
		{
			if !mesh.face_iter().any(|fid| fid == face_id) {
				return Err(MeshError::MeshIsInvalid {message: format!("Halfedge {} points to an invalid face {}", halfedge_id, face_id)});
			}
			if walker.next_id().is_none() {
				return Err(MeshError::MeshIsInvalid {message: format!("Halfedge {} points to a face but not a next halfedge", halfedge_id)});
			}
		}

		if let Some(next_id) = walker.next_id()
		{
			if !mesh.halfedge_iter().any(|he_id| he_id == next_id) {
				return Err(MeshError::MeshIsInvalid {message: format!("Halfedge {} points to an invalid next halfedge {}", halfedge_id, next_id)});
			}
			if walker.face_id().is_none() {
				return Err(MeshError::MeshIsInvalid {message: format!("Halfedge {} points to a next halfedge but not a face", halfedge_id)});
			}
			if mesh.walker_from_halfedge(next_id).previous_id().unwrap() != halfedge_id
			{
				return Err(MeshError::MeshIsInvalid {message: format!("Halfedge next pointed to by halfedge {} does not point back to halfedge", halfedge_id)});
			}
		}

		if mesh.edge_length(halfedge_id) < 0.00001
		{
			return Err(MeshError::MeshIsInvalid {message: format!("Length of edge {} is too small ({})", halfedge_id, mesh.edge_length(halfedge_id))})
		}
	}
	log::debug!("=== validate 3");
	for face_id in mesh.face_iter() {
		if let Some(halfedge_id) = mesh.walker_from_face(face_id).halfedge_id()
		{
			if !mesh.halfedge_iter().any(|he_id| he_id == halfedge_id) {
				return Err(MeshError::MeshIsInvalid {message: format!("Face {} points to an invalid halfedge {}", face_id, halfedge_id)});
			}
			if mesh.walker_from_face(face_id).face_id().unwrap() != face_id
			{
				return Err(MeshError::MeshIsInvalid {message: format!("Halfedge pointed to by face {} does not point to back to face", face_id)});
			}
		}
		else {
			return Err(MeshError::MeshIsInvalid {message: format!("Face {} does not point to a halfedge", face_id)});
		}

		if mesh.face_area(face_id) < 0.00001
		{
			return Err(MeshError::MeshIsInvalid {message: format!("Area of face {} is too small ({})", face_id, mesh.face_area(face_id))})
		}
	}

	log::debug!("=== validate 4");
	for vertex_id1 in mesh.vertex_iter()
	{
		for vertex_id2 in mesh.vertex_iter()
		{
			if mesh.connecting_edge(vertex_id1, vertex_id2).is_some() != mesh.connecting_edge(vertex_id2, vertex_id1).is_some()
			{
				return Err(MeshError::MeshIsInvalid {message: format!("Vertex {} and Vertex {} is connected one way, but not the other way", vertex_id1, vertex_id2)});
			}
			let mut found = false;
			for halfedge_id in mesh.vertex_halfedge_iter(vertex_id1) {
				if mesh.walker_from_halfedge(halfedge_id).vertex_id().unwrap() == vertex_id2 {
					if found {
						return Err(MeshError::MeshIsInvalid {message: format!("Vertex {} and Vertex {} is connected by multiple edges", vertex_id1, vertex_id2)})
					}
					found = true;
				}
			}
		}
	}
	log::debug!("=== validate 5 - all");

	Ok(())
}
