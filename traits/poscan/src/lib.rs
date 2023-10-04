#![cfg_attr(not(feature = "std"), no_std)]

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
	fn get_poscan_object(i: u32) -> Option<sp_consensus_poscan::ObjData<Account, Block>>;
}

