#![cfg_attr(not(feature = "std"), no_std)]

use frame_system::Config;
use sp_runtime::scale_info::TypeInfo;
use sp_runtime::traits::Member;
use codec::{Encode, Decode};
use sp_std::vec::Vec;

pub type AccountId<T> = <T as Config>::AccountId;
pub type BlockNumber<T> = <T as Config>::BlockNumber;

pub trait SerialNumbersApi<Account>
where
    Account: Encode + Decode + TypeInfo + Member,
{
    /// Verify if a serial number is valid for the given block
    fn verify_serial_number(sn_hash: [u8; 16], block: u32) -> bool;
    
    /// Verify a serial number without storing it (stateless verification)
    fn verify_serial_number_stateless(
        sn_hash: [u8; 16], 
        owner: &Account, 
        block: u32, 
        block_index: u32
    ) -> bool;
    
    /// Generate multiple serial numbers for a block (off-chain helper)
    fn generate_serial_numbers_for_block(
        owner: &Account, 
        block: u32, 
        count: u32
    ) -> Vec<[u8; 16]>;
    
    /// Get serial number details (all if sn_index is None)
    fn get_serial_numbers(sn_index: Option<u64>) -> Vec<SerialNumberDetails<Account, u32>>;
    
    /// Get all serial number indices owned by an account
    fn get_sn_owners(owner: Account) -> Vec<u64>;
    
    /// Check if a serial number has been used
    fn is_serial_number_used(sn_hash: [u8; 16]) -> bool;
    
    /// Get the total count of serial numbers created
    fn sn_count() -> u64;
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo)]
pub struct SerialNumberDetails<AccountId, BlockNumber> {
    pub sn_index: u64,
    pub sn_hash: [u8; 16],
    pub owner: AccountId,
    pub created: BlockNumber,
    pub block_index: u32,
    pub is_expired: bool,
    pub expired: Option<BlockNumber>,
}
