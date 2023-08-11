#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::vec::Vec;


pub trait PoscanApi {
	fn uncompleted_objects() -> Option<Vec<(u32, Vec<u8>)>>;
}

// impl<Difficulty: Clone + Ord + Default, AccountId: Clone + Ord> MiningPoolStatApi<Difficulty, AccountId> for () {
// 	fn difficulty(_pool_id: &AccountId) -> Difficulty { Difficulty::default() }
// 	fn member_status(_pool_id: &AccountId, _member_id: &AccountId) -> Result<(), CheckMemberError> { Ok(()) }
// 	fn get_stat(_pool_id: &AccountId) -> Option<(Percent, Percent, Vec<(AccountId, u32)>)> { None }
// }
