use codec::{Decode, Encode};

use sp_runtime::{
    traits::{
        AtLeast32BitUnsigned, CheckedAdd, CheckedSub, MaybeSerializeDeserialize, Member,
        Saturating, StaticLookup, Zero,
    },
    DispatchError, DispatchResult, RuntimeDebug,
};
use sp_std::{
    collections::btree_map::BTreeMap,
    convert::{TryFrom, TryInto},
    prelude::*,
    result,
};

use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, ensure, IterableStorageMap, Parameter,
    StorageMap,
};
use frame_system::{ensure_root, ensure_signed};

use rio_support::info;

use orml_traits::{
    arithmetic::{self, Signed},
    BalanceStatus, LockIdentifier, MultiCurrency, MultiCurrencyExtended, MultiLockableCurrency,
    MultiReservableCurrency,
    // MultiReservableCurrency, OnReceived,
};

use crate::types::TotalAssetInfo;

use frame_support::traits::{Currency as FrameCurrency, Get, Instance};
use sp_std::marker::PhantomData;

use pallet_balances::{NegativeImbalance,PositiveImbalance};

use frame_support::traits::{WithdrawReasons,ExistenceRequirement,SignedImbalance};

use rio_proc_macro::rio_typelevel;

use super::*;

pub struct Compat<T,I>(PhantomData<(T,I)>);

#[rio_typelevel]
impl<T, I> FrameCurrency<T::AccountId> for Compat<T,I>
  where
    T: pallet_balances::Config + MakeQSelf<AsBalances>,
    T: Config<Balance = AsBalances::Balance>,
    I: Get<T::CurrencyId>,
    T::AccountId: MakeAlias<AccountId>,
{
  type Balance = AsBalances::Balance;
  type PositiveImbalance = PositiveImbalance<T>;
  type NegativeImbalance = NegativeImbalance<T>;

  fn total_balance(who: &AccountId) -> Self::Balance {
    unimplemented!()
  }

  fn can_slash(who: &AccountId, value: Self::Balance) -> bool {
    unimplemented!()
  }

  fn total_issuance() -> Self::Balance {
    unimplemented!()
  }

  fn minimum_balance() -> Self::Balance
  { unimplemented!() }

  fn burn(amount: Self::Balance) -> Self::PositiveImbalance
  {
    <TotalIssuance<T>>::mutate(I::get(), |v| *v -= amount);
    Self::PositiveImbalance::new(amount)
  }

  fn issue(amount: Self::Balance) -> Self::NegativeImbalance
  {
    <TotalIssuance<T>>::mutate(I::get(), |v| *v += amount);
    Self::NegativeImbalance::new(amount)
  }

  fn free_balance(who: &AccountId) -> Self::Balance
  {
    Module::<T>::accounts(who, I::get()).free
  }

  fn ensure_can_withdraw(
        who: &AccountId,
        _amount: Self::Balance,
        reasons: WithdrawReasons,
        new_balance: Self::Balance
    ) -> DispatchResult
  { unimplemented!() }

  fn transfer(
        source: &AccountId,
        dest: &AccountId,
        value: Self::Balance,
        existence_requirement: ExistenceRequirement
    ) -> DispatchResult
  { unimplemented!() }

  fn slash(
        who: &AccountId,
        value: Self::Balance
    ) -> (Self::NegativeImbalance, Self::Balance)
  { unimplemented!() }

  fn deposit_into_existing(
        who: &AccountId,
        value: Self::Balance
    ) -> Result<Self::PositiveImbalance, DispatchError>
  { unimplemented!() }

  fn deposit_creating(
        who: &AccountId,
        value: Self::Balance
    ) -> Self::PositiveImbalance
  {
    // TODO: How to solve problem with this checks ?
    //let _ = Module::<T>::get_asset(&I::get())?;
    //Module::<T>::can_do(&I::get(), Restriction::Depositable)?;
    if value.is_zero() {
      return Self::PositiveImbalance::new(value);
    }
    // TODO: Maybe checked add is needed ?
    Module::<T>::set_free_balance(I::get(), who, Module::<T>::free_balance(I::get(), who) + value);
    Self::PositiveImbalance::new(value)
  }

  fn withdraw(
        who: &AccountId,
        value: Self::Balance,
        reasons: WithdrawReasons,
        liveness: ExistenceRequirement
    ) -> Result<Self::NegativeImbalance, DispatchError>
  {
    let _ = Module::<T>::get_asset(&I::get())?;
    Module::<T>::can_do(&I::get(), Restriction::Withdrawable)?;
    if value.is_zero() {
      return Ok(Self::NegativeImbalance::new(value));
    }
    Module::<T>::ensure_can_withdraw(I::get(), who, value)?;
    // TODO: Maybe checked sub is needed ?
    Module::<T>::set_free_balance(I::get(), who, Module::<T>::free_balance(I::get(), who) - value);
    Ok(Self::NegativeImbalance::new(value))
  }

  fn make_free_balance_be(
        who: &AccountId,
        balance: Self::Balance
    ) -> SignedImbalance<Self::Balance, Self::PositiveImbalance>
  { unimplemented!() }

}





