use sp_runtime::RuntimeDebug;
use sp_std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};
use codec::{Decode, Encode, EncodeLike};
use scale_info::TypeInfo;
use frame_support::{dispatch::DispatchResult, ensure, pallet_prelude::MaxEncodedLen, weights::Weight};
use sp_runtime::traits::Saturating;

use pallet_atomic_swap::SwapAction;
use super::{
    pallet,
    pallet::{
        Error,
    },
    types::AccountStatus,
};

/// A swap action that only allows transferring balances.
#[derive(Clone, RuntimeDebug, Eq, PartialEq, Encode, Decode, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(I))]
// #[codec(mel_bound())]
pub struct TokenSwapAction<T: pallet::Config<I>, I: 'static, AccountId, AssetId: Encode + EncodeLike> {
    asset_id: AssetId,
    value: <T as pallet::Config<I>>::Balance,
    _marker: PhantomData<(AccountId, T, I)>,
}

impl<T: pallet::Config<I>, I: 'static, AccountId, AssetId: Encode + EncodeLike> TokenSwapAction<T, I, AccountId, AssetId> {
    /// Create a new swap action value of balance.
    pub fn new(asset_id: AssetId, value: <T as pallet::Config<I>>::Balance) -> Self {
        Self { asset_id, value, _marker: PhantomData }
    }
}

impl<T: pallet::Config<I>, I: 'static, AccountId, AssetId: Encode + EncodeLike> Deref for TokenSwapAction<T, I, AccountId, AssetId> {
    type Target = <T as pallet::Config<I>>::Balance;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T: pallet::Config<I>, I: 'static, AccountId, AssetId: Encode + EncodeLike> DerefMut
for TokenSwapAction<T, I, AccountId, AssetId> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<T: pallet::Config<I>, I: 'static> SwapAction<T::AccountId, T> for TokenSwapAction<T, I, T::AccountId, T::AssetId>
{
    fn reserve(&self, source: &T::AccountId) -> DispatchResult {
        // let _ = pallet::Asset::<T, I>::get(self.asset_id.clone());

        if let Some(_) = pallet::AccountReserved::<T, I>::get(self.asset_id.clone(), (*source).clone()) {
            return Err(Error::<T, I>::AlreadyExists.into())
        }

        pallet::Account::<T, I>::try_mutate(self.asset_id.clone(), (*source).clone(), |maybe_account| -> DispatchResult {
            let asset_acc = maybe_account.as_mut().ok_or(Error::<T, I>::NoAccount)?;
            if asset_acc.status != AccountStatus::Liquid {
                return Err(Error::<T, I>::IncorrectStatus.into())
            }

            if asset_acc.balance < self.value {
                return Err(Error::<T, I>::BalanceLow.into())
            }

            asset_acc.balance.saturating_sub(self.value);
            pallet::AccountReserved::<T, I>::insert(self.asset_id.clone(), (*source).clone(), self.value);

            Ok(())
        })?;

        Ok(())
    }

    fn claim(&self, source: &T::AccountId, target: &T::AccountId) -> bool {
        if let Some(_reserved) = pallet::AccountReserved::<T, I>::get(self.asset_id.clone(), (*source).clone()) {
            pallet::Account::<T, I>::try_mutate(self.asset_id.clone(), (*source).clone(), |maybe_account| -> DispatchResult {
                ensure!(maybe_account.is_some(), Error::<T, I>::NoAccount);
                self.cancel(source);

                pallet::Account::<T, I>::try_mutate(self.asset_id.clone(), (*target).clone(), |_maybe_account| -> DispatchResult {
                    let f = super::types::TransferFlags { keep_alive: false, best_effort: false, burn_dust: false };
                    pallet::Pallet::<T, I>::do_transfer(self.asset_id.clone(), source, target, self.value, Some((*target).clone()), f).map(|_| ())
                })
            }).map_or_else(|_| self.reserve(source).is_ok(), |_| true)
        }
        else {
            false
        }
    }

    fn weight(&self) -> Weight {
        1_000
    }

    fn cancel(&self, source: &T::AccountId) {
        if let Some(_reserved) = pallet::AccountReserved::<T, I>::get(self.asset_id.clone(), (*source).clone()) {
            let _ = pallet::Account::<T, I>::try_mutate(self.asset_id.clone(), (*source).clone(), |maybe_account| -> DispatchResult {
                let asset_acc = maybe_account.as_mut().ok_or(Error::<T, I>::NoAccount)?;
                asset_acc.balance.saturating_add(asset_acc.balance);
                pallet::AccountReserved::<T, I>::remove(self.asset_id.clone(), (*source).clone());

                Ok(())
            });
        }
    }
}