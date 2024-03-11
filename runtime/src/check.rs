use sp_core::H256;
use sp_std::vec::Vec;

pub(crate) fn check_obj(_alg_id: &[u8;16], _obj: &Vec<u8>, _hashes: &Vec<H256>) -> bool {
	// TODO: additional validation
	true
}
