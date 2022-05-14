//! Helper module to build a genesis configuration for the weight-fee-runtime

use super::{
	AccountId, BalancesConfig, GenesisConfig, GrandpaConfig, Signature, SudoConfig, SystemConfig,
	DifficultyConfig, RewardsConfig, WASM_BINARY,
};
use sp_core::{sr25519, Pair, Public, U256};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::traits::{IdentifyAccount, Verify};
use kulupu_primitives::DOLLARS;

/// Helper function to generate a crypto pair from seed
fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

type AccountPublic = <Signature as Verify>::Signer;

/// Helper function to generate an account ID from seed
pub fn account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Helper function to generate session key from seed
pub fn authority_keys_from_seed(seed: &str) -> GrandpaId {
	get_from_seed::<GrandpaId>(seed)
}

pub fn dev_genesis(wasm_binary: &[u8]) -> GenesisConfig {
	testnet_genesis(
		wasm_binary,
		// Initial Authorities
		vec![authority_keys_from_seed("Alice")],
		// Root Key
		account_id_from_seed::<sr25519::Public>("Alice"),
		// Endowed Accounts
		vec![
			account_id_from_seed::<sr25519::Public>("Alice"),
			account_id_from_seed::<sr25519::Public>("Bob"),
			account_id_from_seed::<sr25519::Public>("Alice//stash"),
			account_id_from_seed::<sr25519::Public>("Bob//stash"),
		],
		U256::from(10),
	)
}

/// Helper function to build a genesis configuration
pub fn testnet_genesis(
	wasm_binary: &[u8],
	initial_authorities: Vec<GrandpaId>,
	root_key: AccountId,
	endowed_accounts: Vec<AccountId>,
	initial_difficulty: U256,
) -> GenesisConfig {
	GenesisConfig {
		frame_system: Some(SystemConfig {
			code: wasm_binary.to_vec(),
			changes_trie_config: Default::default(),
		}),
		pallet_balances: Some(BalancesConfig {
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, 1 << 60))
				.collect(),
		}),
		pallet_sudo: Some(SudoConfig { key: root_key }),
		pallet_grandpa: Some(GrandpaConfig {
			authorities: initial_authorities.iter().map(|x| (x.clone(), 1)).collect(),
		}),
		difficulty: Some(DifficultyConfig {
			initial_difficulty,
		}),
		rewards: Some(RewardsConfig {
			reward: 60 * DOLLARS,
			mints: Default::default(),
		}),
		collective_Instance1: Default::default(),
		collective_Instance2: Default::default(),
		treasury: Default::default(),


		/*
			GenesisConfig {
		system: SystemConfig {
			code: wasm_binary.to_vec(),
			changes_trie_config: Default::default(),
		},
		balances: BalancesConfig {
			balances: vec![
				(
					get_account_id_from_seed::<sr25519::Public>("Alice"),
					10_000_000 * DOLLARS
				),
				(
					get_account_id_from_seed::<sr25519::Public>("Bob"),
					10_000_000 * DOLLARS
				),
			],
		},
		indices: IndicesConfig {
			indices: vec![],
		},
		difficulty: DifficultyConfig {
			initial_difficulty,
		},
		collective_Instance1: Default::default(),
		collective_Instance2: Default::default(),
		democracy: Default::default(),
		treasury: Default::default(),
		elections_phragmen: Default::default(),
		eras: Default::default(),
		membership_Instance1: Default::default(),
		vesting: Default::default(),
		rewards: RewardsConfig {
			reward: 60 * DOLLARS,
			mints: Default::default(),
		},
		contracts: Default::default(),
	}
		*/


	}
}
