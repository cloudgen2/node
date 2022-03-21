#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://substrate.dev/docs/en/knowledgebase/runtime/frame>
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod types {
  use codec::{Decode, Encode};
  use rio_primitives::*;
  use sp_runtime::RuntimeDebug;

  #[derive(Encode, Decode, Clone, PartialEq, Eq, Default, RuntimeDebug, scale_info::TypeInfo)]
  pub struct Strategy {
    pub end_block_number:   BlockNumber,
    pub per_block_reward:   Balance,
    pub start_block_number: BlockNumber,
  }

  #[derive(Encode, Decode, Clone, PartialEq, Eq, Default, RuntimeDebug, scale_info::TypeInfo)]
  pub struct UnstakeType {
    pub amount:        Balance,
    pub applicable_at: Moment,
  }
}

#[frame_support::pallet]
pub mod pallet {
  use frame_support::{
    dispatch::DispatchResult,
    pallet_prelude::*,
    traits::{Currency as FrameCurrency, ExistenceRequirement},
    PalletId,
  };
  use frame_system::pallet_prelude::*;
  use rio_primitives::{
    burn_and_settle, catch_default, emit, fail, issue_and_resolve, ok_or, only_positive_amount, require,
    store, store_delete, store_get, store_set, *,
  };
  use rio_proc_macro::{rio_pallet_module_impl, rio_syntax};
  use sp_runtime::{traits::AccountIdConversion, FixedPointNumber, SaturatedConversion};

  use super::types::*;

  macro_rules! GEN_PATH(($A:ident,$b:ident) => { $A::<T>::$b });
  macro_rules! Error(($a:ident) => { GEN_PATH!(Error,$a) });

  macro_rules! marker_balance(($a:expr) => { MarkerCurrencyOf::<T>::free_balance($a) });
  macro_rules! total_issuance(() => { MarkerCurrencyOf::<T>::total_issuance() });

  macro_rules! to_fxp(($a:expr) => { Price::checked_from_integer($a).ok_or(Error!(CheckedFromIntegerFailed))? });

  type MarkerCurrencyOf<T> = <T as Config>::MarkerCurrency;

  /// Configure the pallet by specifying the parameters and types on which it depends.
  #[pallet::config]
  pub trait Config:
    frame_system::Config<AccountId = AccountId, BlockNumber = BlockNumber>
    + pallet_timestamp::Config<Moment = Moment>
  {
    /// Because this pallet emits events, it depends on the runtime's definition of an event.
    type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

    type StakeCurrency: FrameCurrency<Self::AccountId, Balance = Balance>;

    type MarkerCurrency: FrameCurrency<Self::AccountId, Balance = Balance>;

    type OwnerOrigin: EnsureOrigin<Self::Origin>;

    /// For example: 10^12.
    #[pallet::constant]
    type MinimumStakeBalance: Get<Balance>;

    /// For example: `MinimumStakeBalance` * 188.
    #[pallet::constant]
    type MaximumPerBlockReward: Get<Balance>;

    /// For example: 1.
    #[pallet::constant]
    type DefaultPrice: Get<Price>;

    #[pallet::constant]
    type PalletId: Get<PalletId>;
  }

  #[pallet::pallet]
  #[pallet::generate_store(pub(super) trait Store)]
  pub struct Pallet<T>(_);

  #[pallet::storage]
  pub type ClaimingFeePercent<T> = StorageValue<_, Percent, ValueQuery>;

  #[pallet::storage]
  pub type LastUpdateBlockNumber<T> = StorageValue<_, BlockNumber, ValueQuery>;

  #[pallet::storage]
  pub type LastStakeTime<T> = StorageMap<_, Twox64Concat, AccountId, Moment, ValueQuery>;

  #[pallet::storage]
  pub type FeePool<T> = StorageValue<_, Balance, ValueQuery>;

  #[pallet::storage]
  pub type LockedRewards<T> = StorageValue<_, Balance, ValueQuery>;

  #[pallet::storage]
  pub type TotalStaked<T> = StorageValue<_, Balance, ValueQuery>;

  #[pallet::storage]
  pub type TotalUnstaked<T> = StorageValue<_, Balance, ValueQuery>;

  #[pallet::storage]
  pub type UnstakingTime<T> = StorageValue<_, Moment, ValueQuery>;

  #[pallet::storage]
  pub type PriceStored<T> = StorageValue<_, Price, ValueQuery>;

  #[pallet::storage]
  pub type CurrentStrategy<T> = StorageValue<_, Strategy, ValueQuery>;

  #[pallet::storage]
  pub type NextStrategy<T> = StorageValue<_, Strategy, ValueQuery>;

  #[pallet::storage]
  pub type Unstakes<T> = StorageMap<_, Twox64Concat, AccountId, UnstakeType, ValueQuery>;

  // Pallets use events to inform users when important changes are made.
  // https://substrate.dev/docs/en/knowledgebase/runtime/events
  #[pallet::event]
  #[pallet::generate_deposit(pub(super) fn deposit_event)]
  pub enum Event<T: Config> {
    /// [account, requested_amount, claimed_amount, fee_amount, burned_amount].
    Claimed(AccountId, Balance, Balance, Balance, Balance),
    /// [fee_percent].
    ClaimingFeePercentUpdated(Percent),
    /// [per_block_reward, start_block_number, end_block_number].
    CurrentStrategyUpdated(Balance, BlockNumber, BlockNumber),
    /// [receiver, amount].
    FeeClaimed(AccountId, Balance),
    /// [per_block_reward, start_block_number, end_block_number].
    NextStrategyUpdated(Balance, BlockNumber, BlockNumber),
    /// [unstaking_time].
    UnstakingTimeUpdated(Moment),
    /// Next strategy removed.
    NextStrategyRemoved(),
    /// [amount].
    PoolDecreased(Balance),
    /// [payer, amount].
    PoolIncreased(AccountId, Balance),
    /// Price updated.
    PriceUpdated(Price),
    /// [amount].
    RewardsUnlocked(Balance),
    /// [account, payer, staked_amount, minted_amount].
    Staked(AccountId, Option<AccountId>, Balance, Balance),
    /// [account, requested_amount, unstaked_amount, burned_amount].
    Unstaked(AccountId, Balance, Balance, Balance),
    /// [account, amount].
    UnstakingCanceled(AccountId, Balance),
    /// [account, amount].
    Withdrawed(AccountId, Balance),
  }

  // Errors inform users that something went wrong.
  #[pallet::error]
  pub enum Error<T> {
    /// The strategy is not being applied now.
    TheStrategyIsNotBeingAppliedNow,
    /// Current strategy is not set.
    CurrentStrategyIsNotSet,
    /// Next strategy is not set.
    NextStrategyIsNotSet,
    /// Imposible to calculate price, total supply zero.
    ImposibleToCalculatePriceTotalSupplyIsZero,
    /// Too small unstaking amount.
    TooSmallUnstakingAmount,
    /// Not enougth marked units.
    NotEnoughMarkerUnits,
    /// Validation of strategy parameters is failed, duration is zero.
    VOSpDurationIsZero,
    /// Validation of strategy parameters is failed, start block number less than current.
    VOSpStartBlockNumberLessThanCurrent,
    /// Validation of strategy parameters is failed, per block reward overflow,
    VOSpPerBlockRewardOverflow,
    /// Invalid fee percent,
    InvalidFeePercent,
    /// Minimal stake balance should be more or equal to one asset marker.
    MinimalStakeBalanceShouldBeMoreOrEqualToOneAssetMarker,
    /// To small staking amount.
    ToSmallStakingAmount,
    /// Unstake amount is zero, for account.
    UnstakeAmountIsZeroForAccount,
    /// Unstake is not released yet, for account.
    UnstakeIsNotReleasedYetForAccount,
    /// Not enough funds in supply.
    NotEnoughFundsInSupply,
    /// Amount is not positive.
    AmountIsNotPositive,
    /// Not enough unstaked balance.
    NotEnoughUnstakedBalance,
    /// Stake balance is less than minimal stake.
    StakeBalanceIsLessThanMinimalStake,
    /// Positive imbalance.
    PositiveImbalance,
    /// No fees.
    NoFees,
    /// Not enough locked rewards.
    NotEnoughLockedRewards,
    /// Checked from rational failed.
    CheckedFromRationalFailed,
    /// Checked from integer failed.
    CheckedFromIntegerFailed,
  }

  type Re<T> = Result<T, DispatchError>;

  #[rio_pallet_module_impl]
  impl<T: Config> Pallet<T> {
    #[inline]
    pub fn get_pallet_account() -> AccountId { T::PalletId::get().into_account() }

    #[inline]
    pub fn get_block_number() -> BlockNumber { frame_system::Pallet::<T>::block_number() }

    #[inline]
    pub fn get_timestamp() -> Moment { pallet_timestamp::Pallet::<T>::now() }

    #[inline]
    pub fn try_get_current_strategy_unlocked_rewards() -> Re<Balance> {
      Self::try_get_strategy_unlocked_rewards(&store_get!(CurrentStrategy))
    }

    #[rio_syntax]
    pub fn try_get_unlocked_rewards() -> Re<(Balance, bool)> {
      let mut unlocked = get_current_strategy_unlocked_rewards!();
      let mut current_strategy_ended = false;
      if get_block_number!() >= store_get!(CurrentStrategy).end_block_number {
        current_strategy_ended = true;
        unlocked += get_strategy_unlocked_rewards!(&store_get!(NextStrategy));
      }
      unlocked = unlocked.min(store_get!(LockedRewards));
      |"Ok"| (unlocked, current_strategy_ended)
    }

    #[rio_syntax]
    pub fn try_calculate_price() -> Re<Price> {
      let (unlocked, _) = get_unlocked_rewards!();
      let total_staked = store_get!(TotalStaked);
      let total_supply = total_issuance!();
      if total_supply > 0 {
        let p: Option<Price> = Price::checked_from_rational(total_staked + unlocked, total_supply);
        return ok_or!(p, CheckedFromRationalFailed);
      }
      |"fail!"| ImposibleToCalculatePriceTotalSupplyIsZero;
    }

    pub fn try_calculate_unstake(account: &AccountId, amount: Balance) -> Re<(Balance, Balance)> {
      Self::try_calculate_unstake_parametric(account, amount, calculate_price!())
    }

    // Here methods implementations is started in solidity contract order.

    #[rio_syntax]
    pub fn try_cancel_unstaking(caller: AccountId, amount: Balance) -> Re<()> {
      only_positive_amount!(amount);
      update!();
      let mut unstake = Unstakes::<T>::get(caller.clone());
      |"require!"| unstake.amount >= amount ^ || NotEnoughUnstakedBalance;
      let marker_balance = marker_balance!(&caller);
      let price_stored = store_get!(PriceStored);
      let staked_amount = |".floor_to()"| price_stored * to_fxp!(marker_balance);
      |"require!"| {
        staked_amount + amount >= T::MinimumStakeBalance::get() ^ || StakeBalanceIsLessThanMinimalStake
      };
      let marker_amount = |".floor_to()"| to_fxp!(amount) / price_stored;
      issue_and_resolve!(T::MarkerCurrency, &caller, marker_amount);
      |"store!"| {
        TotalStaked += amount;
        TotalUnstaked -= amount;
      };
      unstake.amount -= amount;
      |"store!"| {
        Unstakes[caller.clone()] = unstake;
      };
      |"emit!"| {
        Staked(caller.clone(), None, amount, marker_amount);
        UnstakingCanceled(caller.clone(), amount);
      };
      Ok(())
    }

    #[rio_syntax]
    pub fn try_claim(caller: AccountId, amount: Balance) -> Re<()> {
      only_positive_amount!(amount);
      update!();
      let (mut claimed_amount, burned_amount) = calculate_unstake!(&caller, amount);
      let fee = store_get!(ClaimingFeePercent) * claimed_amount;
      before_marker_asset_transfer!(Some(&caller), None, burned_amount);
      burn_and_settle!(T::MarkerCurrency, &caller, burned_amount);
      |"store!"| {
        TotalStaked -= claimed_amount;
      };
      claimed_amount -= fee;
      |"store!"| {
        FeePool += fee;
      };
      |"emit!"| Claimed(caller.clone(), amount, claimed_amount, fee, burned_amount);
      T::StakeCurrency::transfer(
        &get_pallet_account!(),
        &caller,
        claimed_amount,
        ExistenceRequirement::KeepAlive,
      )?;
      Ok(())
    }

    #[rio_syntax]
    pub fn try_claim_fees(receiver: AccountId) -> Re<()> {
      let fees = store_get!(FeePool);
      |"require!"| fees > 0 ^ || NoFees;
      // Owner is checked in extrinsic.
      store_delete!(FeePool);
      |"emit!"| FeeClaimed(receiver.clone(), fees);
      T::StakeCurrency::transfer(&get_pallet_account!(), &receiver, fees, ExistenceRequirement::KeepAlive)?;
      Ok(())
    }

    #[rio_syntax]
    pub fn try_create_new_strategy(
      per_block_reward: Balance, start_block_number: BlockNumber, duration: BlockNumber,
    ) -> Re<()> {
      update!();
      // FIXME: Maybe create new method that just will create Strategy structure and
      // do validation ?
      validate_strategy_parameters!(per_block_reward, start_block_number, duration);
      let end_block_number = start_block_number + duration;
      let strategy = Strategy { per_block_reward, start_block_number, end_block_number };
      let mut current_strategy = store_get!(CurrentStrategy);
      if current_strategy.start_block_number > get_block_number!() {
        store_delete!(NextStrategy);
        |"emit!"| {
          NextStrategyRemoved();
          CurrentStrategyUpdated(per_block_reward, start_block_number, end_block_number);
        };
      } else {
        |"emit!"| NextStrategyUpdated(per_block_reward, start_block_number, end_block_number);
        |"store!"| {
          NextStrategy = strategy;
        };
        if current_strategy.end_block_number > start_block_number {
          current_strategy.end_block_number = start_block_number;
          |"store!"| {
            CurrentStrategy = current_strategy.clone();
          };
          |"emit!"| {
            CurrentStrategyUpdated(
              current_strategy.per_block_reward,
              current_strategy.start_block_number,
              current_strategy.end_block_number,
            )
          };
        }
      }
      Ok(())
    }

    #[rio_syntax]
    pub fn try_decrease_pool(signer: AccountId, amount: Balance) -> Re<()> {
      only_positive_amount!(amount);
      update!();
      let decreased_amount = amount.min(store_get!(LockedRewards));
      if decreased_amount == 0 {
        return Ok(());
      }
      |"require!"| store_get!(LockedRewards) < decreased_amount ^ || NotEnoughLockedRewards;
      T::StakeCurrency::transfer(
        &get_pallet_account!(),
        &signer,
        decreased_amount,
        ExistenceRequirement::KeepAlive,
      )?;
      |"store!"| {
        LockedRewards -= decreased_amount;
      };
      |"emit!"| PoolDecreased(decreased_amount);
      Ok(())
    }

    #[rio_syntax]
    pub fn try_increase_pool(caller: AccountId, amount: Balance) -> Re<()> {
      only_positive_amount!(amount);
      update!();
      |"store!"| {
        LockedRewards += amount;
      };
      |"emit!"| PoolIncreased(caller, amount);
      Ok(())
    }

    // set_claiming_fee_percent is in extrinsics.

    #[rio_syntax]
    pub fn try_deposit_to_stake(staker: AccountId, amount: Balance) -> Re<()> {
      only_positive_amount!(amount);
      stake!(&staker, &staker, amount);
      Ok(())
    }

    #[rio_syntax]
    pub fn try_stake_for_user(payer: AccountId, staker: AccountId, amount: Balance) -> Re<()> {
      only_positive_amount!(amount);
      stake!(&staker, &payer, amount);
      Ok(())
    }

    #[rio_syntax]
    pub fn try_unstake(caller: AccountId, amount: Balance) -> Re<()> {
      only_positive_amount!(amount);
      update!();
      let (unstaked_amount, burned_amount) =
        calculate_unstake_parametric!(&caller, amount, store_get!(PriceStored));
      burn_and_settle!(T::MarkerCurrency, &caller, amount);
      |"store!"| {
        TotalStaked -= unstaked_amount;
        TotalUnstaked += unstaked_amount;
      };
      let mut unstake = Unstakes::<T>::get(caller.clone());
      unstake.amount += unstaked_amount;
      unstake.applicable_at = get_timestamp!() + store_get!(UnstakingTime);
      |"store!"| {
        Unstakes[caller.clone()] = unstake;
      };
      |"emit!"| Unstaked(caller.clone(), amount, unstaked_amount, burned_amount);
      Ok(())
    }

    // update is in extrinsics directly.

    #[rio_syntax]
    pub fn try_withdraw_from_unstaked(caller: AccountId) -> Re<()> {
      let unstake = Unstakes::<T>::get(caller.clone());
      |"require!"| {
        unstake.amount > 0 ^ || UnstakeAmountIsZeroForAccount;
        unstake.applicable_at <= get_timestamp!() ^ || UnstakeIsNotReleasedYetForAccount;
      };
      T::StakeCurrency::transfer(
        &get_pallet_account!(),
        &caller,
        unstake.amount,
        ExistenceRequirement::KeepAlive,
      )?;
      store_delete! { Unstakes[caller.clone()] };
      |"store!"| {
        TotalStaked -= unstake.amount.clone();
      };
      |"emit!"| Withdrawed(caller.clone(), unstake.amount.clone());
      Ok(())
    }

    // set_unstaking_time is in extrinsics directly.

    #[rio_syntax]
    pub fn try_get_strategy_unlocked_rewards(strategy: &Strategy) -> Re<Balance> {
      let current_block_number = get_block_number!();
      if current_block_number < strategy.start_block_number
        || current_block_number == store_get!(LastUpdateBlockNumber)
      {
        |"fail!"| TheStrategyIsNotBeingAppliedNow;
      }
      let last_rewarded_block_number = store_get!(LastUpdateBlockNumber).max(strategy.start_block_number);
      let last_rewardable_block_number = current_block_number.min(strategy.end_block_number);
      if last_rewarded_block_number < last_rewardable_block_number {
        let blocks_diff = last_rewardable_block_number - last_rewarded_block_number;
        return |"Ok"| strategy.per_block_reward * blocks_diff as u128;
      }
      |"fail!"| TheStrategyIsNotBeingAppliedNow;
    }

    #[inline]
    #[rio_syntax]
    pub fn try_calculate_unstake_parametric(
      account: &AccountId, amount: Balance, price: Price,
    ) -> Re<(Balance, Balance)> {
      use rio_primitives::TruncCeilFloor;
      let mut unstaked_amount = amount;
      let mut burned_amount = |".ceil_to()"| price / to_fxp!(amount);
      let balance = marker_balance!(account);
      |"require!"| {
        burned_amount > 0 ^ || TooSmallUnstakingAmount;
        balance >= burned_amount ^ || NotEnoughMarkerUnits;
      };
      let remaining_marker_balance = balance - burned_amount;
      let remaining_stake = |".floor_to()"| price * to_fxp!(remaining_marker_balance);
      if remaining_stake < T::MinimumStakeBalance::get() {
        burned_amount = balance;
        unstaked_amount += remaining_stake;
      }
      |"Ok"| (unstaked_amount, burned_amount)
    }

    #[rio_syntax]
    pub fn try_unlock_rewards_and_stake() -> Re<()> {
      let (unlocked, current_strategy_ended) = get_unlocked_rewards!();
      if current_strategy_ended {
        let strategy = store_get!(NextStrategy);
        |"store!"| {
          CurrentStrategy = strategy.clone();
        };
        |"emit!"| {
          NextStrategyRemoved();
          CurrentStrategyUpdated(
            strategy.per_block_reward,
            strategy.start_block_number,
            strategy.end_block_number,
          );
        };
        store_delete!(NextStrategy);
        if unlocked > 0 {
          |"emit!"| RewardsUnlocked(unlocked);
          |"store!"| {
            LockedRewards -= unlocked;
            TotalStaked += unlocked;
          };
        }
      }
      |"store!"| {
        LastUpdateBlockNumber = get_block_number!();
      };
      Ok(())
    }

    #[inline]
    pub fn try_update() -> Re<()> {
      if get_block_number!() <= store_get!(LastUpdateBlockNumber) {
        return Ok(());
      }
      unlock_rewards_and_stake!();
      update_price!();
      Ok(())
    }

    pub fn try_update_price() -> Re<()> {
      let new_price = catch_default!(
        ImposibleToCalculatePriceTotalSupplyIsZero,
        Self::try_calculate_price(),
        T::DefaultPrice::get()
      )?;
      store! { PriceStored = new_price; }
      emit! { PriceUpdated(new_price); }
      Ok(())
    }

    #[rio_syntax]
    pub fn try_validate_strategy_parameters(
      per_block_reward: Balance, start_block_number: BlockNumber, duration: BlockNumber,
    ) -> Re<()> {
      |"require!"| {
        duration > 0 ^ || VOSpDurationIsZero;
        start_block_number >= get_block_number!() ^ || VOSpStartBlockNumberLessThanCurrent;
        per_block_reward <= T::MaximumPerBlockReward::get() ^ || VOSpPerBlockRewardOverflow;
      };
      Ok(())
    }

    #[rio_syntax]
    pub fn try_set_claiming_fee_percent(fee_percent: Percent) -> Re<()> {
      |"require!"| fee_percent > Percent::zero() ^ || InvalidFeePercent;
      store! { ClaimingFeePercent = fee_percent; }
      emit! { ClaimingFeePercentUpdated(fee_percent); }
      Ok(())
    }

    pub fn set_unstaking_time(unstaking_time: Moment) {
      store! { UnstakingTime = unstaking_time; }
      emit! { UnstakingTimeUpdated(unstaking_time); }
    }

    #[rio_syntax]
    pub fn try_before_marker_asset_transfer(
      from: Option<&AccountId>, to: Option<&AccountId>, amount: Balance,
    ) -> Re<()> {
      update!();
      let price_stored = store_get!(PriceStored);
      match from {
        | None => (),
        | Some(from) => {
          let marker_balance_of_from = marker_balance!(from);
          let new_balance_of_from = |".ceil_to()"| price_stored * to_fxp!(marker_balance_of_from - amount);
          |"require!"| {
            new_balance_of_from
              >= T::MinimumStakeBalance::get() ^ || MinimalStakeBalanceShouldBeMoreOrEqualToOneAssetMarker
          };
        }
      }
      match to {
        | None => (),
        | Some(to) => {
          let marker_balance_of_to = marker_balance!(to);
          let new_balance_of_to = |".ceil_to()"| price_stored * to_fxp!(marker_balance_of_to + amount);
          |"require!"| {
            new_balance_of_to
              >= T::MinimumStakeBalance::get() ^ || MinimalStakeBalanceShouldBeMoreOrEqualToOneAssetMarker
          };
        }
      }
      Ok(())
    }

    #[rio_syntax]
    pub fn set_current_strategy(
      per_block_reward: Balance, start_block_number: BlockNumber, end_block_number: BlockNumber,
    ) {
      |"store!"| {
        CurrentStrategy = Strategy { per_block_reward, start_block_number, end_block_number };
      };
      |"emit!"| CurrentStrategyUpdated(per_block_reward, start_block_number, end_block_number);
    }

    /// returns minted_amount.
    #[rio_syntax]
    pub fn try_stake(staker: &AccountId, payer: &AccountId, amount: Balance) -> Re<Balance> {
      update!();
      let minted_amount = |".floor_to()"| to_fxp!(amount) / store_get!(PriceStored);
      |"require!"| minted_amount > 0 ^ || ToSmallStakingAmount;
      before_marker_asset_transfer!(None, Some(staker), minted_amount);
      T::StakeCurrency::transfer(&payer, &get_pallet_account!(), amount, ExistenceRequirement::KeepAlive)?;
      issue_and_resolve!(T::MarkerCurrency, staker, minted_amount);
      |"store!"| {
        TotalStaked += amount;
        LastStakeTime[staker] = get_timestamp!();
      };
      |"emit!"| Staked(staker.clone(), |"Some"| payer.clone(), amount, minted_amount);
      |"Ok"| minted_amount
    }
  }

  // Dispatchable functions allows users to interact with the pallet and invoke state changes.
  // These functions materialize as "extrinsics", which are often compared to transactions.
  // Dispatchable functions must be annotated with a weight and must return a DispatchResult.
  #[pallet::call]
  impl<T: Config> Pallet<T> {
    /// Deposit assets to stake.
    #[pallet::weight(0)]
    pub fn deposit_to_stake(origin: OriginFor<T>, amount: Balance) -> DispatchResult {
      deposit_to_stake!(ensure_signed(origin)?, amount);
      Ok(())
    }

    #[pallet::weight(0)]
    pub fn claim(origin: OriginFor<T>, amount: Balance) -> DispatchResult {
      claim!(ensure_signed(origin)?, amount);
      Ok(())
    }

    #[pallet::weight(0)]
    pub fn claim_fees(origin: OriginFor<T>) -> DispatchResult {
      T::OwnerOrigin::ensure_origin(origin.clone())?;
      claim_fees!(ensure_signed(origin)?);
      Ok(())
    }

    #[pallet::weight(0)]
    pub fn decrease_pool(origin: OriginFor<T>, amount: Balance) -> DispatchResult {
      T::OwnerOrigin::ensure_origin(origin.clone())?;
      decrease_pool!(ensure_signed(origin)?, amount);
      Ok(())
    }

    #[pallet::weight(0)]
    pub fn cancel_unstaking(origin: OriginFor<T>, amount: Balance) -> DispatchResult {
      cancel_unstaking!(ensure_signed(origin)?, amount);
      Ok(())
    }

    /// Withdraw unstaked assets.
    #[pallet::weight(0)]
    pub fn withdraw_from_unstaked(origin: OriginFor<T>) -> DispatchResult {
      withdraw_from_unstaked!(ensure_signed(origin)?);
      Ok(())
    }

    #[pallet::weight(0)]
    pub fn update_unstaking_time(origin: OriginFor<T>, unstaking_time: Moment) -> DispatchResult {
      T::OwnerOrigin::ensure_origin(origin)?;
      set_unstaking_time!(unstaking_time);
      emit! { UnstakingTimeUpdated(unstaking_time); };
      Ok(())
    }

    #[pallet::weight(0)]
    pub fn increase_pool(origin: OriginFor<T>, amount: Balance) -> DispatchResult {
      increase_pool!(ensure_signed(origin)?, amount);
      Ok(())
    }

    #[pallet::weight(0)]
    pub fn stake_for_user(origin: OriginFor<T>, staker: AccountId, amount: Balance) -> DispatchResult {
      stake_for_user!(ensure_signed(origin)?, staker, amount);
      Ok(())
    }

    #[pallet::weight(0)]
    pub fn unstake(origin: OriginFor<T>, amount: Balance) -> DispatchResult {
      unstake!(ensure_signed(origin)?, amount);
      Ok(())
    }

    #[pallet::weight(0)]
    pub fn create_new_strategy(
      origin: OriginFor<T>, per_block_reward: Balance, start_block_number: BlockNumber, duration: BlockNumber,
    ) -> DispatchResult {
      T::OwnerOrigin::ensure_origin(origin)?;
      create_new_strategy!(per_block_reward, start_block_number, duration);
      Ok(())
    }

    #[pallet::weight(0)]
    pub fn set_claiming_fee_percent(origin: OriginFor<T>, fee_percent: Percent) -> DispatchResult {
      T::OwnerOrigin::ensure_origin(origin)?;
      set_claiming_fee_percent!(fee_percent);
      Ok(())
    }
  }
}
