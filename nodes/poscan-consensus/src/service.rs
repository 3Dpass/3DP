//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.
#![allow(clippy::needless_borrow)]
use runtime::{self, opaque::Block, RuntimeApi};
use sc_client_api::{ExecutorProvider, RemoteBackend};
use sc_executor::native_executor_instance;
pub use sc_executor::NativeExecutor;
use sc_finality_grandpa::{GrandpaBlockImport, grandpa_peers_set_config};
use sc_service::{error::Error as ServiceError, Configuration, PartialComponents, TaskManager};
use poscan_grid2d::*;
use sp_api::TransactionFor;
use sp_consensus::import_queue::BasicQueue;
use sp_core::{Encode, Decode, H256};
use sp_inherents::InherentDataProviders;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use std::path::PathBuf;
// use sp_runtime::generic::BlockId;
use sc_consensus_poscan::PoscanData;
use log::*;
use sp_std::collections::vec_deque::VecDeque;
use spin::Mutex;
use std::str::FromStr;
use sp_keystore::{SyncCryptoStore, SyncCryptoStorePtr};
use sp_core::crypto::{Ss58Codec,UncheckedFrom, Ss58AddressFormat};
use sp_core::Pair;
use sp_consensus_poscan::POSCAN_COIN_ID;

pub struct MiningProposal {
	pub id: i32,
	pub pre_obj: Vec<u8>,
}

lazy_static! {
    pub static ref DEQUE: Mutex<VecDeque<MiningProposal>> = {
        let m = VecDeque::new();
        Mutex::new(m)
    };
}

// Our native executor instance.
native_executor_instance!(
	pub Executor,
	runtime::api::dispatch,
	runtime::native_version,
);

pub fn decode_author(
	author: Option<&str>, keystore: SyncCryptoStorePtr, keystore_path: Option<PathBuf>,
) -> Result<sc_consensus_poscan::app::Public, String> {
	if let Some(author) = author {
		if author.starts_with("0x") {
			Ok(sc_consensus_poscan::app::Public::unchecked_from(
				H256::from_str(&author[2..]).map_err(|_| "Invalid author account".to_string())?
			).into())
		} else {
			let (address, version) = sc_consensus_poscan::app::Public::from_ss58check_with_version(author)
				.map_err(|_| "Invalid author address".to_string())?;
			if version != Ss58AddressFormat::Custom(POSCAN_COIN_ID.into()) {
				return Err("Invalid author version".to_string())
			}
			Ok(address)
		}
	} else {
		info!("The node is configured for mining, but no author key is provided.");

		let (pair, phrase, _) = sc_consensus_poscan::app::Pair::generate_with_phrase(None);

		SyncCryptoStore::insert_unknown(
			&*keystore.as_ref(),
			sc_consensus_poscan::app::ID,
			&phrase,
			pair.public().as_ref(),
		).map_err(|e| format!("Registering mining key failed: {:?}", e))?;

		info!("Generated a mining key with address: {}", pair.public().to_ss58check_with_version(Ss58AddressFormat::Custom(POSCAN_COIN_ID.into())));

		match keystore_path {
			Some(path) => info!("You can go to {:?} to find the seed phrase of the mining key.", path),
			None => warn!("Keystore is not local. This means that your mining key will be lost when exiting the program. This should only happen if you are in dev mode."),
		}

		Ok(pair.public())
	}
}

type FullClient = sc_service::TFullClient<Block, RuntimeApi, Executor>;
type FullBackend = sc_service::TFullBackend<Block>;
type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;

pub fn build_inherent_data_providers() -> Result<InherentDataProviders, ServiceError> {
	let providers = InherentDataProviders::new();

	providers
		.register_provider(sp_timestamp::InherentDataProvider)
		.map_err(Into::into)
		.map_err(sp_consensus::error::Error::InherentData)?;

	Ok(providers)
}

/// Returns most parts of a service. Not enough to run a full chain,
/// But enough to perform chain operations like purge-chain
#[allow(clippy::type_complexity)]
pub fn new_partial(
	config: &Configuration,
) -> Result<
	PartialComponents<
		FullClient,
		FullBackend,
		FullSelectChain,
		BasicQueue<Block, TransactionFor<FullClient, Block>>,
		sc_transaction_pool::FullPool<Block, FullClient>,
		(
			sc_consensus_poscan::PowBlockImport<
				Block,
				GrandpaBlockImport<FullBackend, Block, FullClient, FullSelectChain>,
				FullClient,
				FullSelectChain,
				PoscanAlgorithm<FullClient>,
				impl sp_consensus::CanAuthorWith<Block>,
			>,
			sc_finality_grandpa::LinkHalf<Block, FullClient, FullSelectChain>,
		),
	>,
	ServiceError,
> {
	let inherent_data_providers = build_inherent_data_providers()?;

	let (client, backend, keystore_container, task_manager) =
		sc_service::new_full_parts::<Block, RuntimeApi, Executor>(&config)?;
	let client = Arc::new(client);

	let select_chain = sc_consensus::LongestChain::new(backend.clone());

	let transaction_pool = sc_transaction_pool::BasicPool::new_full(
		config.transaction_pool.clone(),
		config.role.is_authority().into(),
		config.prometheus_registry(),
		task_manager.spawn_handle(),
		client.clone(),
	);

	let (grandpa_block_import, grandpa_link) = sc_finality_grandpa::block_import(
		client.clone(),
		&(client.clone() as std::sync::Arc<_>),
		select_chain.clone(),
	)?;

	let can_author_with = sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone());

	let pow_block_import = sc_consensus_poscan::PowBlockImport::new(
		grandpa_block_import,
		client.clone(),
		poscan_grid2d::PoscanAlgorithm::new(client.clone()),
		0, // check inherents starting at block 0
		select_chain.clone(),
		inherent_data_providers.clone(),
		can_author_with,
	);

	let import_queue = sc_consensus_poscan::import_queue(
		Box::new(pow_block_import.clone()),
		None,
		poscan_grid2d::PoscanAlgorithm::new(client.clone()),
		inherent_data_providers.clone(),
		&task_manager.spawn_handle(),
		config.prometheus_registry(),
	)?;

	Ok(PartialComponents {
		client,
		backend,
		import_queue,
		keystore_container,
		task_manager,
		transaction_pool,
		select_chain,
		inherent_data_providers,
		other: (pow_block_import, grandpa_link),
	})
}

/// Builds a new service for a full client.
pub fn new_full(
	mut config: Configuration,
	author: Option<&str>,
) -> Result<TaskManager, ServiceError> {
	let sc_service::PartialComponents {
		client,
		backend,
		mut task_manager,
		import_queue,
		keystore_container,
		select_chain,
		transaction_pool,
		inherent_data_providers,
		other: (pow_block_import, grandpa_link),
	} = new_partial(&config)?;

	config.network.extra_sets.push(grandpa_peers_set_config());

	let (network, network_status_sinks, system_rpc_tx, network_starter) =
		sc_service::build_network(sc_service::BuildNetworkParams {
			config: &config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue,
			on_demand: None,
			block_announce_validator_builder: None,
		})?;

	if config.offchain_worker.enabled {
		sc_service::build_offchain_workers(
			&config,
			backend.clone(),
			task_manager.spawn_handle(),
			client.clone(),
			network.clone(),
		);
	}

	let is_authority = config.role.is_authority();
	let prometheus_registry = config.prometheus_registry().cloned();
	let enable_grandpa = !config.disable_grandpa;

	let rpc_extensions_builder = {
		let client = client.clone();
		let pool = transaction_pool.clone();
		Box::new(move |deny_unsafe, _| {
			let deps = crate::rpc::FullDeps {
				client: client.clone(),
				pool: pool.clone(),
				deny_unsafe,
				// command_sink: command_sink.clone(),
			};

			crate::rpc::create_full(deps)
		})
	};

	let keystore_path = config.keystore.path().map(|p| p.to_owned());

	let (_rpc_handlers, telemetry_connection_notifier) =
		sc_service::spawn_tasks(sc_service::SpawnTasksParams {
			network: network.clone(),
			client: client.clone(),
			keystore: keystore_container.sync_keystore(),
			task_manager: &mut task_manager,
			transaction_pool: transaction_pool.clone(),
			rpc_extensions_builder, // Box::new(|_, _| ()),
			on_demand: None,
			remote_blockchain: None,
			backend,
			network_status_sinks,
			system_rpc_tx,
			config,
		})?;

	if is_authority {
		let author = decode_author(author, keystore_container.sync_keystore(), keystore_path)?;

		let proposer = sc_basic_authorship::ProposerFactory::new(
			task_manager.spawn_handle(),
			client.clone(),
			transaction_pool,
			prometheus_registry.as_ref(),
		);

		let can_author_with =
			sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone());

		let (_worker, worker_task) = sc_consensus_poscan::start_mining_worker(
			Box::new(pow_block_import),
			client.clone(),
			select_chain,
			PoscanAlgorithm::new(client.clone()),
			proposer,
			network.clone(),
			Some(author.encode()),
			inherent_data_providers,
			// time to wait for a new block before starting to mine a new one
			Duration::from_secs(10),
			// how long to take to actually build the block (i.e. executing extrinsics)
			Duration::from_secs(10),
			can_author_with,
		);

		task_manager
			.spawn_essential_handle()
			.spawn_blocking("pow", worker_task);

		// Start Mining
		let	mut poscan_data: Option<PoscanData> = None;
		let mut poscan_hash: H256 = H256::random();

		let pre_digest = author.encode();
		let author = sc_consensus_poscan::app::Public::decode(&mut &pre_digest[..]).map_err(|_| {
			ServiceError::Other(
				"Unable to mine: author pre-digest decoding failed".to_string(),
			)
		})?;

		let keystore = keystore_container.local_keystore()
		.ok_or(ServiceError::Other(
			"Unable to mine: no local keystore".to_string(),
		))?;

		info!(">>> author: {:x?}", &author);

		let pair = keystore.key_pair::<sc_consensus_poscan::app::Pair>(
			&author,
		).map_err(|_| ServiceError::Other(
			"Unable to mine: fetch pair from author failed".to_string(),
		))?;
		// .ok_or(ServiceError::Other(
		// 	"Unable to mine: key not found in keystore".to_string(),
		// ))?;

		debug!(target:"poscan", ">>> Spawn mining loop");

		thread::spawn(move || loop {
			let worker = _worker.clone();
			let metadata = worker.lock().metadata();
			if let Some(metadata) = metadata {

				// info!(">>> poscan_hash: compute: {:x?}", &poscan_hash);
				let compute = Compute {
					difficulty: metadata.difficulty,
					pre_hash:  metadata.pre_hash,
					poscan_hash,
				};

				let signature = compute.sign(&pair);
				let seal = compute.seal(signature.clone());
				if hash_meets_difficulty(&seal.work, seal.difficulty) {
					let mut worker = worker.lock();

					if let Some(ref psdata) = poscan_data {
						// let _ = psdata.encode();
						info!(">>> hash_meets_difficulty: submit it: {}, {}, {}",  &seal.work, &seal.poscan_hash, &seal.difficulty);
						// TODO: pass signature to submit
						info!(">>> signature: {:x?}", &signature.to_vec());
						info!(">>> pre_hsash: {:x?}", compute.pre_hash);
						info!(">>> check verify: {}", compute.verify(&signature.clone(), &author));
						worker.submit(seal.encode(), &psdata);
					}
				} else {
					let mut lock = DEQUE.lock();
					let maybe_mining_prop = (*lock).pop_front();
					if let Some(mp) = maybe_mining_prop {
						let hashes = get_obj_hashes(&mp.pre_obj);
						if hashes.len() > 0 {
							let obj_hash = hashes[0];
							let dh = DoubleHash { pre_hash: metadata.pre_hash, obj_hash };
							poscan_hash = dh.calc_hash();
							info!(">>>double hash is: {:#}", poscan_hash.to_string());
							poscan_data = Some(PoscanData { hashes, obj: mp.pre_obj });
						}
						else {
							warn!(">>> Empty hash set for obj {}", mp.id);
						}
						// thread::sleep(Duration::new(1, 0));
					}
					else {
						thread::sleep(Duration::new(1, 0));
					}
				}
			} else {
				thread::sleep(Duration::new(1, 0));
			}
		});
	}

	let grandpa_config = sc_finality_grandpa::Config {
		gossip_duration: Duration::from_millis(333),
		justification_period: 512,
		name: None,
		observer_enabled: false,
		keystore: Some(keystore_container.sync_keystore()),
		is_authority,
	};

	if enable_grandpa {
		// start the full GRANDPA voter
		// NOTE: non-authorities could run the GRANDPA observer protocol, but at
		// this point the full voter should provide better guarantees of block
		// and vote data availability than the observer. The observer has not
		// been tested extensively yet and having most nodes in a network run it
		// could lead to finality stalls.
		let grandpa_config = sc_finality_grandpa::GrandpaParams {
			config: grandpa_config,
			link: grandpa_link,
			network,
			telemetry_on_connect: telemetry_connection_notifier.map(|x| x.on_connect_stream()),
			voting_rule: sc_finality_grandpa::VotingRulesBuilder::default().build(),
			prometheus_registry,
			shared_voter_state: sc_finality_grandpa::SharedVoterState::empty(),
		};

		// the GRANDPA voter task is considered infallible, i.e.
		// if it fails we take down the service with it.
		task_manager.spawn_essential_handle().spawn_blocking(
			"grandpa-voter",
			sc_finality_grandpa::run_grandpa_voter(grandpa_config)?,
		);
	}

	network_starter.start_network();
	Ok(task_manager)
}

/// Builds a new service for a light client.
pub fn new_light(config: Configuration) -> Result<TaskManager, ServiceError> {
	let inherent_data_providers = build_inherent_data_providers()?;

	let (client, backend, keystore_container, mut task_manager, on_demand) =
		sc_service::new_light_parts::<Block, RuntimeApi, Executor>(&config)?;

	let transaction_pool = Arc::new(sc_transaction_pool::BasicPool::new_light(
		config.transaction_pool.clone(),
		config.prometheus_registry(),
		task_manager.spawn_handle(),
		client.clone(),
		on_demand.clone(),
	));

	let select_chain = sc_consensus::LongestChain::new(backend.clone());

	let (grandpa_block_import, _) = sc_finality_grandpa::block_import(
		client.clone(),
		&(client.clone() as Arc<_>),
		select_chain.clone(),
	)?;

	// FixMe #375
	let _can_author_with = sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone());

	let pow_block_import = sc_consensus_poscan::PowBlockImport::new(
		grandpa_block_import,
		client.clone(),
		PoscanAlgorithm::new(client.clone()),
		0, // check inherents starting at block 0
		select_chain,
		inherent_data_providers.clone(),
		sp_consensus::AlwaysCanAuthor,
	);

	let import_queue = sc_consensus_poscan::import_queue(
		Box::new(pow_block_import),
		None,
		PoscanAlgorithm::new(client.clone()),
		inherent_data_providers,
		&task_manager.spawn_handle(),
		config.prometheus_registry(),
	)?;

	let (network, network_status_sinks, system_rpc_tx, network_starter) =
		sc_service::build_network(sc_service::BuildNetworkParams {
			config: &config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue,
			on_demand: Some(on_demand.clone()),
			block_announce_validator_builder: None,
		})?;

	if config.offchain_worker.enabled {
		sc_service::build_offchain_workers(
			&config,
			backend.clone(),
			task_manager.spawn_handle(),
			client.clone(),
			network.clone(),
		);
	}

	sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		remote_blockchain: Some(backend.remote_blockchain()),
		transaction_pool,
		task_manager: &mut task_manager,
		on_demand: Some(on_demand),
		rpc_extensions_builder: Box::new(|_, _| ()),
		config,
		client,
		keystore: keystore_container.sync_keystore(),
		backend,
		network,
		network_status_sinks,
		system_rpc_tx,
	})?;

	network_starter.start_network();

	Ok(task_manager)
}
