//! Helper module to build a genesis configuration for the weight-fee-runtime

use super::{
	AccountId, Signature, GenesisConfig,
	BalancesConfig, SudoConfig, SystemConfig, IndicesConfig,
	DifficultyConfig,
	RewardsConfig,
	CouncilConfig,
	SessionConfig,
	ValidatorSetConfig,
	opaque::SessionKeys,
};
use sp_core::{sr25519, Pair, Public, U256};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::traits::{IdentifyAccount, Verify};
use sp_consensus_poscan::DOLLARS;

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
pub fn authority_keys_from_seed(seed: &str) -> (AccountId, GrandpaId) {
	(
		account_id_from_seed::<sr25519::Public>(seed),
		get_from_seed::<GrandpaId>(seed),
	)
}

fn session_keys(grandpa: GrandpaId) -> SessionKeys {
	SessionKeys { grandpa }
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
	initial_authorities: Vec<(AccountId, GrandpaId)>,
	root_key: AccountId,
	endowed_accounts: Vec<AccountId>,
	initial_difficulty: U256,
) -> GenesisConfig {
	GenesisConfig {
		system: SystemConfig {
			code: wasm_binary.to_vec(),
			// changes_trie_config: Default::default(),
		},
		balances: BalancesConfig {
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, 1 << 60))
				.collect(),
		},
		indices: IndicesConfig {
			indices: vec![],
		},
		sudo: SudoConfig { key: Some(root_key) },
		grandpa: Default::default(),
		im_online: Default::default(),
		difficulty: DifficultyConfig {
			initial_difficulty,
		},
		rewards: RewardsConfig {
			reward: 500 * DOLLARS,
			mints: Default::default(),
		},
		democracy: Default::default(),
		council: CouncilConfig {
			members: initial_authorities.iter().map(|x| x.0.clone()).collect::<Vec<_>>(),
			phantom: Default::default(),
		},
		technical_committee: Default::default(),
		treasury: Default::default(),
		vesting: Default::default(),
		transaction_payment: Default::default(),
		transaction_storage: Default::default(),
		assets: Default::default(),
		scored_pool: Default::default(),
		validator_set: ValidatorSetConfig {
			initial_validators: initial_authorities.iter().map(|x| x.0.clone()).collect::<Vec<_>>(),
		},
		session: SessionConfig {
			keys: initial_authorities.iter().map(|x| {
				// (x.0.clone(), x.0.clone(), session_keys(x.1.clone(), x.2.clone()))
				(x.0.clone(), x.0.clone(), session_keys(x.1.clone()))
			}).collect::<Vec<_>>(),
		},
	}
}
