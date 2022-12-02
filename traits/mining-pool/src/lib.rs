#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::vec::Vec;
pub use sp_runtime::Percent;
use frame_system::Config;
pub type PoolId<T> = <T as Config>::AccountId;

pub trait MiningPoolStatApi<Difficulty: Clone + Ord + Default, AccountId: Clone + Ord> {
	/// Return the target pow difficulty of the next block.
	fn difficulty(pool_id: &AccountId) -> Difficulty;
	fn get_stat(pool_id: &AccountId) -> Option<(Percent, Vec<(AccountId, u32)>)>;
}

impl<Difficulty: Clone + Ord + Default, AccountId: Clone + Ord> MiningPoolStatApi<Difficulty, AccountId> for () {
	fn difficulty(_pool_id: &AccountId) -> Difficulty { Difficulty::default() }
	fn get_stat(_pool_id: &AccountId) -> Option<(Percent, Vec<(AccountId, u32)>)> { None }
}
