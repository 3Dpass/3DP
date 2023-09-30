#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::vec::Vec;
use frame_system::Config;
use sp_runtime::scale_info::TypeInfo;
use sp_runtime::traits::Member;
use codec::{Encode, Decode};

pub type AccountId<T> = <T as Config>::AccountId;
pub type Block<T> = <T as Config>::BlockNumber;

pub trait PoscanApi<Account, Block>
	where
		Account: Encode + Decode + TypeInfo + Member,
		Block: Encode + Decode + TypeInfo + Member,
{
	fn uncompleted_objects() -> Option<Vec<(u32, Vec<u8>)>>;
	fn get_poscan_object(i: u32) -> Option<sp_consensus_poscan::ObjData<Account, Block>>;
}

// impl<Difficulty: Clone + Ord + Default, AccountId: Clone + Ord> MiningPoolStatApi<Difficulty, AccountId> for () {
// 	fn difficulty(_pool_id: &AccountId) -> Difficulty { Difficulty::default() }
// 	fn member_status(_pool_id: &AccountId, _member_id: &AccountId) -> Result<(), CheckMemberError> { Ok(()) }
// 	fn get_stat(_pool_id: &AccountId) -> Option<(Percent, Percent, Vec<(AccountId, u32)>)> { None }
// }
