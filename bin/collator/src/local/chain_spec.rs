//! Chain specifications.

use local_runtime::{
    wasm_binary_unwrap, AccountId, AuraConfig, AuraId, BalancesConfig, EVMConfig, GenesisConfig,
    GrandpaConfig, GrandpaId, LocalNetworkPrecompiles, Signature, SudoConfig, SystemConfig,
    VestingConfig,
    RioAssetsConfig,
    RioGatewayConfig,
    RioRootConfig,
    RioPaymentFeeConfig,
    SessionKeys,
    ValidatorSetConfig,
    SessionConfig,
};
use sc_service::ChainType;
use sp_core::{sr25519, Pair, Public};

use sp_runtime::traits::{IdentifyAccount, Verify};

use rio_primitives::{Balance,CurrencyId};

type AccountPublic = <Signature as Verify>::Signer;

/// Specialized `ChainSpec` for Shiden Network.
pub type ChainSpec = sc_service::GenericChainSpec<local_runtime::GenesisConfig>;


fn session_keys(
    aura: AuraId,
    grandpa: GrandpaId,
) -> SessionKeys {
    SessionKeys { aura, grandpa }
}

/// Helper function to generate a crypto pair from seed
fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

/// Helper function to generate an account ID from seed
fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}
/*
/// Generate an Aura authority key.
pub fn authority_keys_from_seed(s: &str) -> (AuraId, GrandpaId) {
    (get_from_seed::<AuraId>(s), get_from_seed::<GrandpaId>(s))
}
*/

pub fn authority_keys_from_seed(s: &str) -> (
    AccountId,
    AuraId,
    GrandpaId
) {
    (
        get_account_id_from_seed::<sr25519::Public>(s),
        get_from_seed::<AuraId>(s),
        get_from_seed::<GrandpaId>(s)
    )
}

/// Development config (single validator Alice)
pub fn development_config() -> ChainSpec {
    let mut properties = serde_json::map::Map::new();
    properties.insert("tokenSymbol".into(), "RFUEL".into());
    properties.insert("tokenDecimals".into(), 18.into());
    ChainSpec::from_genesis(
        "Development",
        "dev",
        ChainType::Development,
        move || {
            testnet_genesis(
                vec![authority_keys_from_seed("Alice")],
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                vec![
                    get_account_id_from_seed::<sr25519::Public>("Alice"),
                    get_account_id_from_seed::<sr25519::Public>("Bob"),
                    get_account_id_from_seed::<sr25519::Public>("Dave"),
                    get_account_id_from_seed::<sr25519::Public>("Charlie"),
                    get_account_id_from_seed::<sr25519::Public>("Eve"),
                    get_account_id_from_seed::<sr25519::Public>("Ferdie"),
                ],
            )
        },
        vec![],
        None,
        None,
        Some(properties),
        None,
    )
}

fn assets_init() -> Vec<(
    CurrencyId,
    rio_assets::AssetInfo,
    rio_assets::Restrictions,
    Vec<(AccountId, Balance)>,
)> {
    use rio_protocol as rp;
    vec![
        // asset id defined in protocol
        (
            CurrencyId::from(rp::LOCKED_RFUEL),
            rio_assets::AssetInfo {
                symbol: b"LOCKED_RFUEL".to_vec(),
                name: b"Locked Rio Fuel Token".to_vec(),
                decimals: 12,
                desc: b"Locked Rio Fuel Token".to_vec(),
                chain: rio_assets::Chain::Rio,
            },
            rio_assets::Restriction::Transferable.into(),
            vec![],
        ),
        (
            CurrencyId::from(rp::OM),
            rio_assets::AssetInfo {
                symbol: b"OM".to_vec(),
                name: b"MANTRA DAO Token".to_vec(),
                decimals: 12,
                desc: b"MANTRA DAO Token".to_vec(),
                chain: rio_assets::Chain::Rio,
            },
            rio_assets::Restrictions::none(),
            vec![],
        ),
        (
            CurrencyId::from(rp::RBTC),
            rio_assets::AssetInfo {
                symbol: b"RBTC".to_vec(),
                name: b"RBTC token".to_vec(),
                decimals: 8,
                desc: b"Bitcoin in RioChain".to_vec(),
                chain: rio_assets::Chain::Bitcoin,
            },
            rio_assets::Restrictions::none(),
            vec![],
        ),
        (
            CurrencyId::from(rp::RLTC),
            rio_assets::AssetInfo {
                symbol: b"RLTC".to_vec(),
                name: b"RLTC token".to_vec(),
                decimals: 8,
                desc: b"Litecoin in RioChain".to_vec(),
                chain: rio_assets::Chain::Litecoin,
            },
            rio_assets::Restrictions::none(),
            vec![],
        ),
        (
            CurrencyId::from(rp::RETH),
            rio_assets::AssetInfo {
                symbol: b"RETH".to_vec(),
                name: b"RETH token".to_vec(),
                decimals: 18,
                desc: b"Ether in RioChain".to_vec(),
                chain: rio_assets::Chain::Ethereum,
            },
            rio_assets::Restrictions::none(),
            vec![],
        ),
        (
            CurrencyId::from(rp::RUSDT),
            rio_assets::AssetInfo {
                symbol: b"RUSDT".to_vec(),
                name: b"RUSDT token".to_vec(),
                decimals: 6,
                desc: b"USDT in RioChain".to_vec(),
                chain: rio_assets::Chain::Ethereum,
            },
            rio_assets::Restrictions::none(),
            vec![],
        ),
    ]
}


fn testnet_genesis(
    initial_authorities: Vec<(AccountId, AuraId, GrandpaId)>,
    root_key: AccountId,
    endowed_accounts: Vec<AccountId>,
) -> GenesisConfig {
    use rio_protocol as rp;
    // This is supposed the be the simplest bytecode to revert without returning any data.
    // We will pre-deploy it under all of our precompiles to ensure they can be called from
    // within contracts.
    // (PUSH1 0x00 PUSH1 0x00 REVERT)
    let revert_bytecode = vec![0x60, 0x00, 0x60, 0x00, 0xFD];
    GenesisConfig {
        system: SystemConfig {
            code: wasm_binary_unwrap().to_vec(),
            changes_trie_config: Default::default(),
        },
        balances: BalancesConfig {
            balances: endowed_accounts
                .iter()
                .cloned()
                .map(|k| (k, 1_000_000_000_000_000_000_000))
                .collect(),
        },
        vesting: VestingConfig { vesting: vec![] },
        validator_set: ValidatorSetConfig {
            validators: initial_authorities.iter().map(|x| x.0.clone()).collect::<Vec<_>>(),
        },
        session: SessionConfig {
            keys: initial_authorities.iter().map(|x| {
                (x.0.clone(), x.0.clone(), session_keys(x.1.clone(), x.2.clone()))
            }).collect::<Vec<_>>(),
        },
        aura: AuraConfig {
            authorities: vec![],
            // authorities: initial_authorities.iter().map(|x| (x.0.clone())).collect(),
        },
        grandpa: GrandpaConfig {
            authorities: vec![],
            /*
            authorities: initial_authorities
                .iter()
                .map(|x| (x.1.clone(), 1))
                .collect(),
                */
        },
        evm: EVMConfig {
            // We need _some_ code inserted at the precompile address so that
            // the evm will actually call the address.
            accounts: LocalNetworkPrecompiles::<()>::used_addresses()
                .map(|addr| {
                    (
                        addr,
                        pallet_evm::GenesisAccount {
                            nonce: Default::default(),
                            balance: Default::default(),
                            storage: Default::default(),
                            code: revert_bytecode.clone(),
                        },
                    )
                })
                .collect(),
        },
        rio_root: RioRootConfig {
            managers: vec![root_key.clone()]
        },
        ethereum: Default::default(),
        sudo: SudoConfig { key: root_key },
        rio_assets: RioAssetsConfig {
            init: assets_init(),
        },
        rio_gateway: RioGatewayConfig {
            max_deposit_index: 10000,
            initial_supported_currencies: vec![
                (CurrencyId::from(rp::RFUEL), 10 * 1_000_000_000_000_000_000), // 10 RFUEL
                (CurrencyId::from(rp::OM), 10 * 1_000_000_000_000_000_000),
                (CurrencyId::from(rp::RBTC), 5 * 100_000),  // 0.005 BTC
                (CurrencyId::from(rp::RETH), 5 * 1_000_000_000_000_000_000),  // 0.05 ETH
                (CurrencyId::from(rp::RUSDT), 5 * 1_000_000), // 5 USDT
            ],
            deposit_addr_info: vec![(
                CurrencyId::from(rp::RBTC),
                rio_gateway::DepositAddrInfo::Bip32(
                    rio_gateway::Bip32 {
                        x_pub: b"upub5DRdTWfz3NeZwd25HeQ2xMNjnYtYRfZzC6fEDjmPH2AwnxjvTrySjVApEiDufv68gqsZ7TCUcNfb1P4KLjNvZCTsPCaVb68SLedQwPKMLKR".to_vec(),
                        path: b"m/49'/1'/0".to_vec()
                    }
                )
            )],
            admins: vec![(
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                rio_gateway::Auths::all(),
            )],
        },
        rio_payment_fee: RioPaymentFeeConfig {
            account_id: get_account_id_from_seed::<sr25519::Public>("Eve"),
        },
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use sp_runtime::BuildStorage;

    #[test]
    fn test_create_development_chain_spec() {
        development_config().build_storage().unwrap();
    }
}
