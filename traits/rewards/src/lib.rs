#![cfg_attr(not(feature = "std"), no_std)]

// use sp_std::collections::btree_set::BTreeSet;
//
// pub trait AccountSet {
// 	type AccountId;
//
// 	fn accounts() -> BTreeSet<Self::AccountId>;
// }

pub trait RewardLocksApi<AccountId: Clone + Ord, Balance: Clone + Ord> {
	fn locks(account_id: &AccountId) -> Balance;
}
