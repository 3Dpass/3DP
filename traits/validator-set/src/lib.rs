#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::vec::Vec;

pub trait ValidatorSetApi<AccountId: Clone + Ord> {
	fn validators() -> Vec<AccountId>;
}
