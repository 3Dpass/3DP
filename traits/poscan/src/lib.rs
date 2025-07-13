#![cfg_attr(not(feature = "std"), no_std)]

use frame_system::Config;
use sp_runtime::scale_info::TypeInfo;
use sp_runtime::traits::Member;
use sp_runtime::DispatchResult;
use codec::{Encode, Decode};
use sp_std::vec::Vec;

pub type AccountId<T> = <T as Config>::AccountId;
pub type Block<T> = <T as Config>::BlockNumber;

pub trait PoscanApi<Account, Block>
	where
		Account: Encode + Decode + TypeInfo + Member,
		Block: Encode + Decode + TypeInfo + Member,
{
	fn get_poscan_object(i: u32) -> Option<sp_consensus_poscan::ObjData<Account, Block>>;
	fn is_owner_of(account_id: &Account, obj_idx: u32) -> bool;
	fn property(obj_idx: u32, prop_idx: u32) -> Option<sp_consensus_poscan::PropValue>;
	fn replicas_of(original_idx: u32) -> Vec<u32>;
    fn object_has_asset(obj_idx: u32) -> bool;
	fn get_dynamic_rewards_growth_rate() -> Option<u32>;
	fn get_author_part() -> Option<u8>;
	fn get_unspent_rewards(obj_idx: u32) -> Option<u128>;
	fn get_fee_payer(obj_idx: u32) -> Option<Account>;
	fn get_pending_storage_fees() -> Option<u128>;
	fn get_rewards() -> Option<u128>;
	fn get_qc_timeout(obj_idx: u32) -> Option<u32>;
}


/// Trait for creating an asset account with a deposit taken from a designated depositor specified
/// by the client.
pub trait AccountTouch<AssetId, AccountId> {
	/// The type for currency units of the deposit.
	type Balance;

	/// The deposit amount of a native currency required for creating an account of the `asset`.
	fn deposit_required(asset: AssetId) -> Self::Balance;

	/// Create an account for `who` of the `asset` with a deposit taken from the `depositor`.
	fn touch(asset: AssetId, who: AccountId, depositor: AccountId) -> DispatchResult;
}

/// Converts an asset balance value into balance.
pub trait ConversionFromAssetBalance<AssetBalance, AssetId, OutBalance> {
	type Error;
	fn from_asset_balance(
		balance: AssetBalance,
		asset_id: AssetId,
	) -> Result<OutBalance, Self::Error>;
}

