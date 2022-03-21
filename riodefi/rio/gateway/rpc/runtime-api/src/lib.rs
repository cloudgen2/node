#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use sp_std::prelude::Vec;

use sp_std::collections::btree_map::BTreeMap;

pub use rio_gateway::{WithdrawItem, WithdrawState};

sp_api::decl_runtime_apis! {
    pub trait GatewayApi<CurrencyId, AccountId, Balance> where
        CurrencyId: Codec,
        AccountId: Codec,
        Balance: Codec,
    {
        fn withdraw_list() -> BTreeMap<u64, (WithdrawItem<CurrencyId, AccountId, Balance>, Balance)>;
        fn pending_withdraw_list() -> BTreeMap<u64, (WithdrawItem<CurrencyId, AccountId, Balance>, Balance)>;
    }

  	pub trait OracleApi<ProviderId, Key, Value> where
		ProviderId: Codec,
		Key: Codec,
		Value: Codec,
	{
		fn get_value(provider_id: ProviderId, key: Key) -> Option<Value>;
		fn get_all_values(provider_id: ProviderId) -> Vec<(Key, Option<Value>)>;
	}
}
