#![feature(stmt_expr_attributes)]

use rio_proc_macro::*;

pub struct Test;

impl Test {
  fn to_fxp(&self) -> u128 { 12345u128 }

  fn floor(&self) -> &Self { self }
}

macro_rules! rio_syntax_rewrite(
  (lwa,"fxp",
    let $a:ident = $b:expr;
  ) => {
    let $a = $b;
    let $a = $a.to_fxp();
  }
);

#[rio_syntax]
fn test01() -> u128 {
  let test = |"fxp"| Test;
  test
}

#[test]
fn test_macro() {
  let test = test01();
  assert_eq!(test, 12345u128);
}
