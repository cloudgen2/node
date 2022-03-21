
#[macro_export]
macro_rules! catch_default(($a:ident,$b:expr,$c:expr) => { match $b {
  Err(error) => if error == Error!($a).into() { Ok($c) } else { Err(error) }, x => x,
}});

#[macro_export]
macro_rules! issue_and_resolve(($a:ty,$b:expr,$c:expr) => {
  <$a>::resolve_creating($b, <$a>::issue($c));
});

#[macro_export]
macro_rules! burn_and_settle(($a:ty,$b:expr,$c:expr) => {
  use frame_support::traits::{WithdrawReasons,ExistenceRequirement};
  <$a>::settle($b,<$a>::burn($c),
    WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE,
    ExistenceRequirement::KeepAlive
  ).map_err(|_| Error!(PositiveImbalance))?;
});

#[macro_export]
macro_rules! store_get(                  ($a:ident)                 => { GEN_PATH!($a,get)() });

#[macro_export]
macro_rules! store_set(                  ($a:ident,$b:expr)         => { GEN_PATH!($a,set)($b) });

#[macro_export]
macro_rules! emit(                 ($($a:expr);* $(;)?)                  => { use Event::*; $(Self::deposit_event($a);)* });

#[macro_export]
macro_rules! fail(                 ($a:ident)                 => { return Err(Error!($a).into()); });

#[macro_export]
macro_rules! ok_or(                ($a:expr,$b:ident)         => { $a.ok_or(Error!($b).into()) });

#[macro_export]
macro_rules! store_delete(

  ($a:ident) => { GEN_PATH!($a,kill)() };
  ($a:ident[$b:expr]) => { GEN_PATH!($a,remove)($b) }

);

#[macro_export]
macro_rules! store(
  ($a:ident = $b:expr; $($t:tt)*) => { GEN_PATH!($a,set)($b); store! { $($t)* } };
  ($a:ident[$b:expr] = $c:expr; $($t:tt)*) => { GEN_PATH!($a,insert)($b,$c); store! { $($t)* } };
  ($a:ident += $b:expr; $($t:tt)*) => { GEN_PATH!($a,mutate)(|x| *x += $b); store! { $($t)* } };
  ($a:ident -= $b:expr; $($t:tt)*) => { GEN_PATH!($a,mutate)(|x| *x -= $b); store! { $($t)* } };
  ($($t:tt)*) => { $($t)* }
);

#[macro_export]
macro_rules! require(
  ($a:expr,$b:ident) => { ensure!($a, Error!($b)); };
  (parser,($($a:tt)*),^ | | $b:ident; $($t:tt)*) => { ensure!($($a)*, Error!($b)); require! { $($t)* } };
  (parser,($($a:tt)*),^ | | $b:ident) => { ensure!($($a)*, Error!($b)); };
  (parser,($($a:tt)*),$b:tt $($c:tt)*) => { require!(parser,($($a)* $b),$($c)*) };
  (parser,($($a:tt)*),$($b:tt)*) => { $($a)* $($b)* };
  ($($t:tt)*) => { require!(parser,(),$($t)*) }
);

#[macro_export]
macro_rules! only_positive_amount( ($a:expr)                  => { require!($a > 0, AmountIsNotPositive); });


