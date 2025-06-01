//! Helper module to build a genesis configuration for the weight-fee-runtime

use super::{
	opaque::SessionKeys, AccountId, BalancesConfig, CouncilConfig, DifficultyConfig, GenesisConfig,
	IndicesConfig, RewardsConfig, SessionConfig, Signature, SudoConfig, SystemConfig,
	ValidatorSetConfig,
	EVMChainIdConfig, EVMConfig,
};
use sp_consensus_poscan::DOLLARS;
use sp_core::{sr25519, Pair, Public, U256, H160, ByteArray};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::traits::{IdentifyAccount, Verify};

use fp_evm::GenesisAccount;

use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use super::Runtime;

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
pub fn authority_keys_from_seed(seed: &str) -> (AccountId, GrandpaId, ImOnlineId) {
	(
		account_id_from_seed::<sr25519::Public>(seed),
		get_from_seed::<GrandpaId>(seed),
		get_from_seed::<ImOnlineId>(seed),
	)
}

fn session_keys(grandpa: GrandpaId, imonline: ImOnlineId) -> SessionKeys {
	SessionKeys { grandpa, imonline }
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

use crate::FrontierPrecompiles;

/// Helper function to build a genesis configuration
pub fn testnet_genesis(
	wasm_binary: &[u8],
	initial_authorities: Vec<(AccountId, GrandpaId, ImOnlineId)>,
	root_key: AccountId,
	endowed_accounts: Vec<AccountId>,
	initial_difficulty: U256,
) -> GenesisConfig {
	let revert_bytecode = vec![0x60, 0x00, 0x60, 0x00, 0xFD];

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
		indices: IndicesConfig { indices: vec![] },
		sudo: SudoConfig {
			key: Some(root_key),
		},
		grandpa: Default::default(),
		im_online: Default::default(),
		difficulty: DifficultyConfig { initial_difficulty },
		rewards: RewardsConfig {
			reward: 500 * DOLLARS,
			mints: Default::default(),
		},
		democracy: Default::default(),
		membership: Default::default(),
		phragmen_election: Default::default(),
		council: CouncilConfig {
			members: initial_authorities
				.iter()
				.map(|x| x.0.clone())
				.collect::<Vec<_>>(),
			phantom: Default::default(),
		},
		technical_committee: Default::default(),
		treasury: Default::default(),
		vesting: Default::default(),
		transaction_payment: Default::default(),
		transaction_storage: Default::default(),
		poscan_assets: Default::default(),
		poscan_pool_assets: Default::default(),
		scored_pool: Default::default(),
		validator_set: ValidatorSetConfig {
			initial_validators: initial_authorities
				.iter()
				.map(|x| x.0.clone())
				.collect::<Vec<_>>(),
		},
		session: SessionConfig {
			keys: initial_authorities
				.iter()
				.map(|x| {
					(
						x.0.clone(),
						x.0.clone(),
						session_keys(x.1.clone(), x.2.clone()),
					)
				})
				.collect::<Vec<_>>(),
		},
		// EVM compatibility
		evm_chain_id: EVMChainIdConfig { chain_id: 1333  as u64},
		evm: EVMConfig {
			// We need _some_ code inserted at the precompile address so that
			// the evm will actually call the address.
			accounts: FrontierPrecompiles::<Runtime>::used_addresses()
				.map(|addr| {
					(
						//addr.into(),
						H160::from_slice(&addr.as_slice()[0..20]),

						GenesisAccount {
							nonce: Default::default(),
							balance: Default::default(),
							storage: Default::default(),
							code: revert_bytecode.clone(),
						},
					)
				})
				.collect(),
		},

			// accounts: {
			// 	let mut map = BTreeMap::new();
			// 	//let mut buf: &[u8; 20] = AccountId::from_str("d7b6PWkynfechps4xKCkGTB5cmXPxuVxUbJzdf2eKZseBqt4K").unwrap().as_slice().try_into().unwrap();
			// 	let mut acc: H160 = H160::from_slice(&AccountId::from_str("d7b6PWkynfechps4xKCkGTB5cmXPxuVxUbJzdf2eKZseBqt4K").unwrap().as_slice()[0..20]);
			//
			// 	map.insert(
			// 		// H160 address of Alice dev account
			// 		// Derived from SS58 (42 prefix) address
			// 		// SS58: 5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY
			// 		// hex: 0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d
			// 		// Using the full hex key, truncating to the first 20 bytes (the first 40 hex chars)
			// 		//H160::from_("d43593c715fdd31c61141abd04a99fd6822c8558")
			// 		//H160::from_str("142d480d26ef1281acabcd10a508aaedb3f4a43e")
			// 		//	.expect("internal H160 is valid; qed"),
			// 		acc,
			// 		fp_evm::GenesisAccount {
			// 			balance: U256::from(1_000_000_000_000_000_000u128),
			// 			//	.expect("internal U256 is valid; qed"),
			// 			code: Default::default(),
			// 			nonce: Default::default(),
			// 			storage: Default::default(),
			// 		},
			// 	);
			// 	map.insert(
			// 		// H160 address of CI test runner account
			// 		H160::from_str("6be02d1d3665660d22ff9624b7be0551ee1ac91b")
			// 			.expect("internal H160 is valid; qed"),
			// 		fp_evm::GenesisAccount {
			// 			balance: U256::from_str("0xffffffffffffffffffffffffffffffff")
			// 				.expect("internal U256 is valid; qed"),
			// 			code: Default::default(),
			// 			nonce: Default::default(),
			// 			storage: Default::default(),
			// 		},
			// 	);
			// 	map.insert(
			// 		// H160 address for benchmark usage
			// 		H160::from_str("1000000000000000000000000000000000000001")
			// 			.expect("internal H160 is valid; qed"),
			// 		fp_evm::GenesisAccount {
			// 			nonce: U256::from(1),
			// 			balance: U256::from(1_000_000_000_000_000_000_000_000u128),
			// 			storage: Default::default(),
			// 			code: vec![0x00],
			// 		},
			// 	);
			// 	map
		// 	},
		// },
		ethereum: Default::default(),
		dynamic_fee: Default::default(),
		base_fee: Default::default(),
	}
}
