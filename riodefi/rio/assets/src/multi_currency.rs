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
    // MultiReservableCurrency, OnReceived,
    MultiReservableCurrency
};

use crate::types::TotalAssetInfo;

use frame_support::traits::{Currency as FrameCurrency, Get};

use super::*;

impl<T: Config> MultiCurrency<T::AccountId> for Module<T> {
    type CurrencyId = T::CurrencyId;
    type Balance = T::Balance;

    fn minimum_balance(currency_id: Self::CurrencyId) -> Self::Balance {
        //TODO: write this code.
        todo!()
    }

    fn total_issuance(currency_id: Self::CurrencyId) -> Self::Balance {
        <TotalIssuance<T>>::get(currency_id)
    }

    fn total_balance(currency_id: Self::CurrencyId, who: &T::AccountId) -> Self::Balance {
        Self::accounts(who, currency_id).total()
    }

    fn free_balance(currency_id: Self::CurrencyId, who: &T::AccountId) -> Self::Balance {
        Self::accounts(who, currency_id).free
    }

    // Ensure that an account can withdraw from their free balance given any
    // existing withdrawal restrictions like locks and vesting balance.
    // Is a no-op if amount to be withdrawn is zero.
    fn ensure_can_withdraw(
        currency_id: Self::CurrencyId,
        who: &T::AccountId,
        amount: Self::Balance,
    ) -> DispatchResult {
        let _ = Self::get_asset(&currency_id)?;
        Self::can_do(&currency_id, Restriction::Withdrawable)?;

        if amount.is_zero() {
            return Ok(());
        }

        let new_balance = Self::free_balance(currency_id, who)
            .checked_sub(&amount)
            .ok_or(Error::<T>::BalanceTooLow)?;
        ensure!(
            new_balance >= Self::accounts(who, currency_id).frozen(),
            Error::<T>::LiquidityRestrictions
        );
        Ok(())
    }

    /// Transfer some free balance from `from` to `to`.
    /// Is a no-op if value to be transferred is zero or the `from` is the same
    /// as `to`.
    fn transfer(
        currency_id: Self::CurrencyId,
        from: &T::AccountId,
        to: &T::AccountId,
        amount: Self::Balance,
    ) -> DispatchResult {
        let _ = Self::get_asset(&currency_id)?;
        Self::can_do(&currency_id, Restriction::Transferable)?;

        if amount.is_zero() || from == to {
            return Ok(());
        }

        Self::ensure_can_withdraw(currency_id, from, amount)?;

        let from_balance = Self::free_balance(currency_id, from);
        let to_balance = Self::free_balance(currency_id, to);
        Self::set_free_balance(currency_id, from, from_balance - amount);
        Self::set_free_balance(currency_id, to, to_balance + amount);

        Ok(())
    }

    /// Deposit some `amount` into the free balance of account `who`.
    ///
    /// Is a no-op if the `amount` to be deposited is zero.
    fn deposit(
        currency_id: Self::CurrencyId,
        who: &T::AccountId,
        amount: Self::Balance,
    ) -> DispatchResult {
        let _ = Self::get_asset(&currency_id)?;
        Self::can_do(&currency_id, Restriction::Depositable)?;

        if amount.is_zero() {
            return Ok(());
        }

        let new_total = Self::total_issuance(currency_id)
            .checked_add(&amount)
            .ok_or(Error::<T>::TotalIssuanceOverflow)?;
        <TotalIssuance<T>>::insert(currency_id, new_total);
        Self::set_free_balance(
            currency_id,
            who,
            Self::free_balance(currency_id, who) + amount,
        );

        Ok(())
    }

    fn withdraw(
        currency_id: Self::CurrencyId,
        who: &T::AccountId,
        amount: Self::Balance,
    ) -> DispatchResult {
        let _ = Self::get_asset(&currency_id)?;
        Self::can_do(&currency_id, Restriction::Withdrawable)?;

        if amount.is_zero() {
            return Ok(());
        }
        Self::ensure_can_withdraw(currency_id, who, amount)?;

        <TotalIssuance<T>>::mutate(currency_id, |v| *v -= amount);
        Self::set_free_balance(
            currency_id,
            who,
            Self::free_balance(currency_id, who) - amount,
        );

        Ok(())
    }

    // Check if `value` amount of free balance can be slashed from `who`.
    fn can_slash(currency_id: Self::CurrencyId, who: &T::AccountId, value: Self::Balance) -> bool {
        if Self::get_asset(&currency_id).is_err()
            || Self::can_do(&currency_id, Restriction::Slashable).is_err()
        {
            return false;
        }

        if value.is_zero() {
            return true;
        }
        Self::free_balance(currency_id, who) >= value
    }

    /// Is a no-op if `value` to be slashed is zero.
    ///
    /// NOTE: `slash()` prefers free balance, but assumes that reserve balance
    /// can be drawn from in extreme circumstances. `can_slash()` should be used
    /// prior to `slash()` to avoid having to draw from reserved funds, however
    /// we err on the side of punishment if things are inconsistent
    /// or `can_slash` wasn't used appropriately.
    fn slash(
        currency_id: Self::CurrencyId,
        who: &T::AccountId,
        amount: Self::Balance,
    ) -> Self::Balance {
        if amount.is_zero() {
            return amount;
        }

        let account = Self::accounts(who, currency_id);
        let free_slashed_amount = account.free.min(amount);
        let mut remaining_slash = amount - free_slashed_amount;

        // slash free balance
        if !free_slashed_amount.is_zero() {
            Self::set_free_balance(currency_id, who, account.free - free_slashed_amount);
        }

        // slash reserved balance
        if !remaining_slash.is_zero() {
            let reserved_slashed_amount = account.reserved.min(remaining_slash);
            remaining_slash -= reserved_slashed_amount;
            Self::set_reserved_balance(
                currency_id,
                who,
                account.reserved - reserved_slashed_amount,
            );
        }

        <TotalIssuance<T>>::mutate(currency_id, |v| *v -= amount - remaining_slash);
        remaining_slash
    }
}

impl<T: Config> MultiCurrencyExtended<T::AccountId> for Module<T> {
    type Amount = T::Amount;

    fn update_balance(
        currency_id: Self::CurrencyId,
        who: &T::AccountId,
        by_amount: Self::Amount,
    ) -> DispatchResult {
        if by_amount.is_zero() {
            return Ok(());
        }

        let by_balance = TryInto::<Self::Balance>::try_into(by_amount.abs())
            .map_err(|_| Error::<T>::AmountIntoBalanceFailed)?;
        if by_amount.is_positive() {
            Self::deposit(currency_id, who, by_balance)
        } else {
            Self::withdraw(currency_id, who, by_balance)
        }
    }
}

impl<T: Config> MultiLockableCurrency<T::AccountId> for Module<T> {
    type Moment = T::BlockNumber;

    // Set a lock on the balance of `who` under `currency_id`.
    // Is a no-op if lock amount is zero.
    fn set_lock(
        lock_id: LockIdentifier,
        currency_id: Self::CurrencyId,
        who: &T::AccountId,
        amount: Self::Balance,
    ) -> DispatchResult {
        if amount.is_zero() {
            return Ok(());
        }
        let mut new_lock = Some(BalanceLock {
            id: lock_id,
            amount,
        });
        let mut locks = Self::locks(who, currency_id)
            .into_iter()
            .filter_map(|lock| {
                if lock.id == lock_id {
                    new_lock.take()
                } else {
                    Some(lock)
                }
            })
            .collect::<Vec<_>>();
        if let Some(lock) = new_lock {
            locks.push(lock)
        }
        Self::update_locks(currency_id, who, &locks[..]);
        Ok(())
    }

    // Extend a lock on the balance of `who` under `currency_id`.
    // Is a no-op if lock amount is zero
    fn extend_lock(
        lock_id: LockIdentifier,
        currency_id: Self::CurrencyId,
        who: &T::AccountId,
        amount: Self::Balance,
    ) -> DispatchResult {
        if amount.is_zero() {
            return Ok(());
        }
        let mut new_lock = Some(BalanceLock {
            id: lock_id,
            amount,
        });
        let mut locks = Self::locks(who, currency_id)
            .into_iter()
            .filter_map(|lock| {
                if lock.id == lock_id {
                    new_lock.take().map(|nl| BalanceLock {
                        id: lock.id,
                        amount: lock.amount.max(nl.amount),
                    })
                } else {
                    Some(lock)
                }
            })
            .collect::<Vec<_>>();
        if let Some(lock) = new_lock {
            locks.push(lock)
        }
        Self::update_locks(currency_id, who, &locks[..]);
        Ok(())
    }

    fn remove_lock(lock_id: LockIdentifier, currency_id: Self::CurrencyId, who: &T::AccountId)  -> DispatchResult {
        let mut locks = Self::locks(who, currency_id);
        locks.retain(|lock| lock.id != lock_id);
        Self::update_locks(currency_id, who, &locks[..]);
        Ok(())
    }
}

impl<T: Config> MultiReservableCurrency<T::AccountId> for Module<T> {
    /// Check if `who` can reserve `value` from their free balance.
    ///
    /// Always `true` if value to be reserved is zero.
    fn can_reserve(
        currency_id: Self::CurrencyId,
        who: &T::AccountId,
        value: Self::Balance,
    ) -> bool {
        if Self::get_asset(&currency_id).is_err()
            || Self::can_do(&currency_id, Restriction::Reservable).is_err()
        {
            return false;
        }

        if value.is_zero() {
            return true;
        }
        Self::ensure_can_withdraw(currency_id, who, value).is_ok()
    }

    /// Slash from reserved balance, returning any amount that was unable to be
    /// slashed.
    ///
    /// Is a no-op if the value to be slashed is zero.
    fn slash_reserved(
        currency_id: Self::CurrencyId,
        who: &T::AccountId,
        value: Self::Balance,
    ) -> Self::Balance {
        if value.is_zero() {
            return Zero::zero();
        }

        let reserved_balance = Self::reserved_balance(currency_id, who);
        let actual = reserved_balance.min(value);
        Self::set_reserved_balance(currency_id, who, reserved_balance - actual);
        <TotalIssuance<T>>::mutate(currency_id, |v| *v -= actual);
        value - actual
    }

    fn reserved_balance(currency_id: Self::CurrencyId, who: &T::AccountId) -> Self::Balance {
        Self::accounts(who, currency_id).reserved
    }

    /// Move `value` from the free balance from `who` to their reserved balance.
    ///
    /// Is a no-op if value to be reserved is zero.
    fn reserve(
        currency_id: Self::CurrencyId,
        who: &T::AccountId,
        value: Self::Balance,
    ) -> DispatchResult {
        let _ = Self::get_asset(&currency_id)?;
        Self::can_do(&currency_id, Restriction::Reservable)?;

        if value.is_zero() {
            return Ok(());
        }
        Self::ensure_can_withdraw(currency_id, who, value)?;

        let account = Self::accounts(who, currency_id);
        Self::set_free_balance(currency_id, who, account.free - value);
        Self::set_reserved_balance(currency_id, who, account.reserved + value);
        Ok(())
    }

    /// Unreserve some funds, returning any amount that was unable to be
    /// unreserved.
    ///
    /// Is a no-op if the value to be unreserved is zero.
    fn unreserve(
        currency_id: Self::CurrencyId,
        who: &T::AccountId,
        value: Self::Balance,
    ) -> Self::Balance {
        if Self::get_asset(&currency_id).is_err()
            || Self::can_do(&currency_id, Restriction::Unreservable).is_err()
        {
            return Zero::zero();
        }

        if value.is_zero() {
            return Zero::zero();
        }

        let account = Self::accounts(who, currency_id);
        let actual = account.reserved.min(value);
        Self::set_reserved_balance(currency_id, who, account.reserved - actual);
        Self::set_free_balance(currency_id, who, account.free + actual);
        value - actual
    }

    /// Move the reserved balance of one account into the balance of another,
    /// according to `status`.
    ///
    /// Is a no-op if:
    /// - the value to be moved is zero; or
    /// - the `slashed` id equal to `beneficiary` and the `status` is
    ///   `Reserved`.
    fn repatriate_reserved(
        currency_id: Self::CurrencyId,
        slashed: &T::AccountId,
        beneficiary: &T::AccountId,
        value: Self::Balance,
        status: BalanceStatus,
    ) -> result::Result<Self::Balance, DispatchError> {
        if value.is_zero() {
            return Ok(Zero::zero());
        }

        if slashed == beneficiary {
            return match status {
                BalanceStatus::Free => Ok(Self::unreserve(currency_id, slashed, value)),
                BalanceStatus::Reserved => {
                    Ok(value.saturating_sub(Self::reserved_balance(currency_id, slashed)))
                }
            };
        }

        let from_account = Self::accounts(slashed, currency_id);
        let to_account = Self::accounts(beneficiary, currency_id);
        let actual = from_account.reserved.min(value);
        match status {
            BalanceStatus::Free => {
                Self::set_free_balance(currency_id, beneficiary, to_account.free + actual);
            }
            BalanceStatus::Reserved => {
                Self::set_reserved_balance(currency_id, beneficiary, to_account.reserved + actual);
            }
        }
        Self::set_reserved_balance(currency_id, slashed, from_account.reserved - actual);
        Ok(value - actual)
    }
}

impl<T: Config> TransferAll<T::AccountId> for Module<T> {
    fn transfer_all(source: &T::AccountId, dest: &T::AccountId) -> DispatchResult {
        //TODO: FIXME: write this code.
        todo!()
    }
}
