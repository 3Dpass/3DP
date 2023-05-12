#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::vec::Vec;

pub trait ValidatorSetApi<AccountId: Clone + Ord, BlockNumber, Balance> {
	fn validators() -> Vec<AccountId>;
	fn author(block_num: BlockNumber) -> Option<AccountId>;
}
