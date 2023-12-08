use sp_runtime::RuntimeDebug;
use sp_std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};
use codec::{Decode, Encode, EncodeLike};
use scale_info::TypeInfo;
use frame_support::{
    dispatch::DispatchResult,
    pallet_prelude::MaxEncodedLen,
    weights::Weight,
};
use frame_support::traits::tokens::Balance;

use pallet_atomic_swap::{Config, SwapAction};

/// A swap action that only allows transferring balances.
#[derive(Clone, RuntimeDebug, Eq, PartialEq, Encode, Decode, TypeInfo, MaxEncodedLen)]
// #[scale_info(skip_type_params(C))]
// #[codec(mel_bound())]
pub struct BalanceSwapAction<AccountId, AssetId: Encode, Balance: Encode> { //: ReservableCurrency<AccountId>> {
    asset_id: AssetId,
    value: Balance,
    _marker: PhantomData<AccountId>,
}

impl<AccountId, AssetId: Encode, Balance: Encode> BalanceSwapAction<AccountId, AssetId, Balance> {
    /// Create a new swap action value of balance.
    //pub fn new(value: <C as Currency<AccountId>>::Balance) -> Self {
    pub fn new(asset_id: AssetId, value: Balance) -> Self {
        Self { asset_id, value, _marker: PhantomData }
    }
}

impl<AccountId, AssetId: Encode, Balance: Encode> Deref for BalanceSwapAction<AccountId, AssetId, Balance> {
    type Target = Balance;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<AccountId, AssetId: Encode, Balance: Encode> DerefMut for BalanceSwapAction<AccountId, AssetId, Balance> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<T: super::pallet::Config<I> + pallet_atomic_swap::Config<I>, I: 'static, AccountId, AssetId: Encode + Clone + EncodeLike<<T as super::pallet::Config<I>>::AssetId>, Balance: Encode> SwapAction<AccountId, T, I> for BalanceSwapAction<AccountId, AssetId, Balance>
    // where
    //     C: ReservableCurrency<AccountId>,
{
    fn reserve(&self, source: &AccountId) -> DispatchResult {
        // C::reserve(source, self.value)
        let _ = super::pallet::Asset::<T, I>::get(self.asset_id.clone());
        Ok(())
    }

    fn claim(&self, source: &AccountId, target: &AccountId) -> bool {
        // C::repatriate_reserved(source, target, self.value, BalanceStatus::Free).is_ok()
        true
    }

    fn weight(&self) -> Weight {
        1_000
    }

    fn cancel(&self, source: &AccountId) {
        // C::unreserve(source, self.value);
    }
}