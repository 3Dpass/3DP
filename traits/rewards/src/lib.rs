#![cfg_attr(not(feature = "std"), no_std)]

pub trait RewardLocksApi<AccountId: Clone + Ord, Balance: Clone + Ord> {
	fn locks(account_id: &AccountId) -> Balance;
	fn unlock_upto(author: &AccountId, amount: Balance);
}
