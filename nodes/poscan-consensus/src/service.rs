//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.
#![allow(clippy::needless_borrow)]
use runtime::{self, opaque::Block, RuntimeApi, AccountId, BlockNumber};
use sc_client_api::{ExecutorProvider, BlockBackend};
use sc_executor::NativeElseWasmExecutor;
use sc_consensus::DefaultImportQueue;
use sc_finality_grandpa::{GrandpaBlockImport, grandpa_peers_set_config};
use sc_service::{error::Error as ServiceError, Configuration, PartialComponents, TaskManager};
use sc_telemetry::{Telemetry, TelemetryWorker};
use poscan_grid2d::*;
use sp_core::{Encode, Decode, H256};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use std::path::PathBuf;
use sc_consensus_poscan::{PoscanData, PoscanDataV1, PoscanDataV2};
use log::*;
use sp_std::collections::vec_deque::VecDeque;
use parking_lot::Mutex;
use std::str::FromStr;
use sp_keystore::{SyncCryptoStore, SyncCryptoStorePtr};
use sp_runtime::traits::Block as BlockT;
use sp_core::crypto::{Ss58Codec,UncheckedFrom, Ss58AddressFormat, set_default_ss58_version};
use sp_core::Pair;
use sp_consensus_poscan::{
	POSCAN_COIN_ID,
	POSCAN_ALGO_GRID2D_V3_1,
	POSCAN_ALGO_GRID2D_V3A,
	REJECT_OLD_ALGO_SINCE,
	CONS_V2_SPEC_VER,
};
use poscan_algo::get_obj_hashes;
use poscan_grid2d::randomx;
use async_trait::async_trait;
use sc_rpc::SubscriptionTaskExecutor;
use sc_finality_grandpa::{FinalityProofProvider as GrandpaFinalityProofProvider};

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

pub struct ExecutorDispatch;

impl sc_executor::NativeExecutionDispatch for ExecutorDispatch {
    type ExtendHostFunctions = (
		frame_benchmarking::benchmarking::HostFunctions,
		poscan_algo::hashable_object::HostFunctions,
	);

    fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
		runtime::api::dispatch(method, data)
    }

   fn native_version() -> sc_executor::NativeVersion {
			   runtime::native_version()
	}
}

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
			if version != Ss58AddressFormat::from(POSCAN_COIN_ID) {
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

		info!("Generated a mining key with address: {}", pair.public().to_ss58check_with_version(Ss58AddressFormat::from(POSCAN_COIN_ID)));

		match keystore_path {
			Some(path) => info!("You can go to {:?} to find the seed phrase of the mining key.", path),
			None => warn!("Keystore is not local. This means that your mining key will be lost when exiting the program. This should only happen if you are in dev mode."),
		}

		Ok(pair.public())
	}
}

type FullClient =
       sc_service::TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<ExecutorDispatch>>;

type FullBackend = sc_service::TFullBackend<Block>;
type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;

pub struct CreateInherentDataProviders {
	author: Option<Vec<u8>>,
}

#[async_trait]
impl sp_inherents::CreateInherentDataProviders<Block, ()> for CreateInherentDataProviders {
	type InherentDataProviders = (
		sp_timestamp::InherentDataProvider,
		pallet_poscan::inherents::InherentDataProvider,
	);

	async fn create_inherent_data_providers(
		&self,
		_parent: <Block as BlockT>::Hash,
		_extra_args: (),
	) -> Result<Self::InherentDataProviders, Box<dyn std::error::Error + Send + Sync>> {
		// Ok(sp_timestamp::InherentDataProvider::from_system_time())
		Ok((
			sp_timestamp::InherentDataProvider::from_system_time(),
			pallet_poscan::inherents::InherentDataProvider::with_author(self.author.clone()),
		))
	}
}

// use sc_network::{
//
/// Returns most parts of a service. Not enough to run a full chain,
/// But enough to perform chain operations like purge-chain
#[allow(clippy::type_complexity)]
pub fn new_partial(
	config: &Configuration,
	// check_inherents_after: u32,
	// enable_weak_subjectivity: bool,
	author: Option<&str>,
) -> Result<
	PartialComponents<
		FullClient,
		FullBackend,
		FullSelectChain,
		// BasicQueue<Block, TransactionFor<FullClient, Block>>,
		DefaultImportQueue<Block, FullClient>,
		sc_transaction_pool::FullPool<Block, FullClient>,
		(
			sc_consensus_poscan::PowBlockImport<
				Block,
				GrandpaBlockImport<FullBackend, Block, FullClient, FullSelectChain>,
				FullClient,
				FullSelectChain,
				PoscanAlgorithm<Block, FullClient, AccountId, BlockNumber>,
				impl sp_consensus::CanAuthorWith<Block>,
				CreateInherentDataProviders,
				AccountId,
				BlockNumber,
			>,
			sc_finality_grandpa::LinkHalf<Block, FullClient, FullSelectChain>,
			sc_finality_grandpa::SharedVoterState,
			Option<Telemetry>,
		),
	>,
	ServiceError,
> {
	let telemetry = config
		.telemetry_endpoints
		.clone()
		.filter(|x| !x.is_empty())
		.map(|endpoints| -> Result<_, sc_telemetry::Error> {
			let worker = TelemetryWorker::new(16)?;
			let telemetry = worker.handle().new_telemetry(endpoints);
			Ok((worker, telemetry))
		})
		.transpose()?;

	set_default_ss58_version(Ss58AddressFormat::from(POSCAN_COIN_ID));

	// let inherent_data_providers = build_inherent_data_providers()?;
	let executor = NativeElseWasmExecutor::<ExecutorDispatch>::new(
		config.wasm_method,
		config.default_heap_pages,
		config.max_runtime_instances,
		config.runtime_cache_size
	);

	let (client, backend, keystore_container, task_manager) =
		// sc_service::new_full_parts::<Block, RuntimeApi, Executor>(&config)?;
		sc_service::new_full_parts(
			&config,
			telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
			executor,
		)?;
	let client = Arc::new(client);

	let keystore_path = config.keystore.path().map(|p| p.to_owned());
	let auth = decode_author(author, keystore_container.sync_keystore(), keystore_path)?.encode();

	poscan_algo::CLIENT.lock().replace((Box::new(client.clone()), config.role.is_light()));

	let telemetry = telemetry.map(|(worker, telemetry)| {
		task_manager.spawn_handle().spawn("telemetry", None, worker.run());
		telemetry
	});

	let select_chain = sc_consensus::LongestChain::new(backend.clone());

	let transaction_pool = sc_transaction_pool::BasicPool::new_full(
		config.transaction_pool.clone(),
		config.role.is_authority().into(),
		config.prometheus_registry(),
		task_manager.spawn_essential_handle(),
		client.clone(),
	);

	let (grandpa_block_import, grandpa_link) = sc_finality_grandpa::block_import(
		client.clone(),
		&(client.clone() as Arc<_>),
		select_chain.clone(),
		telemetry.as_ref().map(|x| x.handle()),
	)?;

	let can_author_with = sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone());

	let pow_block_import = sc_consensus_poscan::PowBlockImport::new(
		grandpa_block_import.clone(),
		client.clone(),
		poscan_grid2d::PoscanAlgorithm::new(client.clone()),
		0, // check inherents starting at block 0
		select_chain.clone(),
		CreateInherentDataProviders { author: Some(auth.encode()) },
		can_author_with,
	);

	let import_queue = sc_consensus_poscan::import_queue(
		Box::new(pow_block_import.clone()),
		Some(Box::new(grandpa_block_import)),
		poscan_grid2d::PoscanAlgorithm::new(client.clone()),
		&task_manager.spawn_essential_handle(),
		config.prometheus_registry(),
	)?;
	let shared_voter_state = sc_finality_grandpa::SharedVoterState::empty();
	let rpc_setup = shared_voter_state.clone();

	Ok(PartialComponents {
		client,
		backend,
		import_queue,
		keystore_container,
		task_manager,
		transaction_pool,
		select_chain,
		other: (pow_block_import, grandpa_link, rpc_setup, telemetry),
	})
}

/// Builds a new service for a full client.
pub fn new_full(
	mut config: Configuration,
	author: Option<&str>,
	threads: usize,
) -> Result<TaskManager, ServiceError> {

	let sc_service::PartialComponents {
		client,
		backend,
		mut task_manager,
		import_queue,
		keystore_container,
		select_chain,
		transaction_pool,
		other: (pow_block_import, grandpa_link, shared_voter_state, mut telemetry),
	} = new_partial(&config, author)?;

	let grandpa_protocol_name = sc_finality_grandpa::protocol_standard_name(
		&client.block_hash(0).ok().flatten().expect("Genesis block exists; qed"),
		&config.chain_spec,
	);
	config

		.network
		.extra_sets
		.push(grandpa_peers_set_config(grandpa_protocol_name.clone()));

	let (network, system_rpc_tx, network_starter) =
		sc_service::build_network(sc_service::BuildNetworkParams {
			config: &config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue,
			block_announce_validator_builder: None,
			warp_sync: None,
		})?;

	if config.offchain_worker.enabled {
		sc_service::build_offchain_workers(
			&config,
			// backend.clone(),
			task_manager.spawn_handle(),
			client.clone(),
			network.clone(),
		);
	}

	let role = config.role.clone();
	let is_authority = config.role.is_authority();
	let prometheus_registry = config.prometheus_registry().cloned();
	let enable_grandpa = !config.disable_grandpa;

	let justification_stream = grandpa_link.justification_stream();
	let shared_authority_set = grandpa_link.shared_authority_set().clone();
	let finality_proof_provider = GrandpaFinalityProofProvider::new_for_service(
		backend.clone(),
		Some(shared_authority_set.clone()),
	);

	use crate::pool::MiningPool;
	let mining_pool = Some(Arc::new(Mutex::new(MiningPool::new())));

	let rpc_extensions_builder = {
		let client = client.clone();
		let pool = transaction_pool.clone();
		let shared_voter_state = shared_voter_state.clone();

		let mining_pool = mining_pool.as_ref().map(|a| a.clone());
		let backend = backend.clone();
		Box::new(move |deny_unsafe, subscription_executor: SubscriptionTaskExecutor| {
			let deps =
				crate::rpc::FullDeps { client: client.clone(), pool: pool.clone(), deny_unsafe,
					grandpa:
						crate::rpc::GrandpaDeps {
						shared_voter_state: shared_voter_state.clone(),
						shared_authority_set: shared_authority_set.clone(),
						justification_stream: justification_stream.clone(),
						subscription_executor: subscription_executor.clone(),
						finality_provider: finality_proof_provider.clone(),
					},
					mining_pool: mining_pool.clone(),
					backend: backend.clone(),
				};
			crate::rpc::create_full(deps).map_err(Into::into)
		})
	};

	let keystore_path = config.keystore.path().map(|p| p.to_owned());

	let _rpc_handlers =
		sc_service::spawn_tasks(sc_service::SpawnTasksParams {
			network: network.clone(),
			client: client.clone(),
			keystore: keystore_container.sync_keystore(),
			task_manager: &mut task_manager,
			transaction_pool: transaction_pool.clone(),
			rpc_builder: rpc_extensions_builder,
			backend,
			// network_status_sinks,
			system_rpc_tx,
			config,
			telemetry: telemetry.as_mut(),
		})?;

	if is_authority {
		let author = decode_author(author, keystore_container.sync_keystore(), keystore_path)?;

		let proposer = sc_basic_authorship::ProposerFactory::new(
			task_manager.spawn_handle(),
			client.clone(),
			transaction_pool,
			prometheus_registry.as_ref(),
			telemetry.as_ref().map(|x| x.handle()),
		);

		let can_author_with =
			sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone());

		let (worker, worker_task) = sc_consensus_poscan::start_mining_worker(
			Box::new(pow_block_import),
			client.clone(),
			select_chain,
			PoscanAlgorithm::new(client.clone()),
			proposer,
			network.clone(),
			network.clone(),
			Some(author.encode()),
			// CreateInherentDataProviders,
			CreateInherentDataProviders { author: Some(author.encode()) },
			// time to wait for a new block before starting to mine a new one
			Duration::from_secs(10),
			// how long to take to actually build the block (i.e. executing extrinsics)
			Duration::from_secs(10),
			can_author_with,
		);

		task_manager
			.spawn_essential_handle()
			.spawn_blocking("poscan", None,  worker_task);

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
		))?
		.ok_or(ServiceError::Other(
		 	"Unable to mine: key not found in keystore".to_string(),
		))?;

		// Start Mining
		info!(">>> Spawn mining loop(s)");

		for _i in 0..threads {
			let worker = worker.clone();
			let mining_pool = mining_pool.as_ref().map(|a| a.clone());
			let pair = pair.clone();
			let client = client.clone();

			thread::spawn(move || loop {
				let metadata = worker.metadata();
				if let Some(metadata) = metadata {
					let maybe_mining_prop =
						mining_pool.clone()
							.map(|mining_pool| {
								let mut mining_pool = mining_pool.lock();
								mining_pool.update_metadata(metadata.pre_hash, metadata.best_hash, metadata.difficulty);
								mining_pool.take()
									.map(|sp| MiningProposal { id: 0, pre_obj: sp.pre_obj })
							})
							.flatten()
							.or_else(|| {
								let mut lock = DEQUE.lock();
								(*lock).pop_front()
							});

					if let Some(mp) = maybe_mining_prop {
						use sp_api::BlockId;
						use sp_blockchain::HeaderBackend;
						let parent_id = BlockId::<Block>::hash(metadata.best_hash);
						let parent_num = client.block_number_from_id(&parent_id).unwrap().unwrap();
						let patch_rot = parent_num >= REJECT_OLD_ALGO_SINCE.into();
						let mining_algo = if patch_rot { &POSCAN_ALGO_GRID2D_V3A } else { &POSCAN_ALGO_GRID2D_V3_1 };

						let ver = client.runtime_version_at(&parent_id).unwrap();
						if ver.spec_version < CONS_V2_SPEC_VER {
							let hashes = get_obj_hashes(mining_algo, &mp.pre_obj, &metadata.pre_hash, patch_rot);
							if hashes.len() > 0 {
								let obj_hash = hashes[0];
								let dh = DoubleHash { pre_hash: metadata.pre_hash, obj_hash };
								let poscan_hash = dh.calc_hash();
								let psdata = PoscanData::V1(PoscanDataV1 {
									alg_id: mining_algo.clone(),
									hashes: hashes.clone(),
									obj: mp.pre_obj.clone(),
								});

								let compute = ComputeV1 {
									difficulty: metadata.difficulty,
									pre_hash: metadata.pre_hash,
									poscan_hash,
								};

								let signature = compute.sign(&pair);
								let seal = compute.seal(signature.clone());
								if hash_meets_difficulty(&seal.work, seal.difficulty) {
									info!(">>> hash_meets_difficulty: submit it: {}, {}, {}",  &seal.work, &seal.poscan_hash, &seal.difficulty);
									let _ = futures::executor::block_on(worker.submit(seal.encode(), &psdata));
								}
							}
						}
						else {
							let orig_hashes = get_obj_hashes(mining_algo, &mp.pre_obj, &H256::default(), false);
							if orig_hashes.len() == 0 {
								continue
							}

							let v = orig_hashes[0].encode();
							let mut buf = metadata.pre_hash.encode();
							buf.append(v.clone().as_mut());

							let rndx = randomx(&buf);
							if let Err(e) = rndx {
								error!(">>> {}", e);
								break
							}
							let rotation_hash = rndx.unwrap();
							let hashes = get_obj_hashes(mining_algo, &mp.pre_obj, &rotation_hash, patch_rot);
							if hashes.len() > 0 {
								use std::collections::HashSet;

								let ha = hashes.iter().collect::<HashSet<_>>();
								let hb = orig_hashes.iter().collect::<HashSet<_>>();
								if ha.intersection(&hb).collect::<Vec<_>>().len() > 0 {
									info!(">>> Some hashes after rotation are in original hashes");
									continue;
								}

								let hist_hash = calc_hist(client.clone(), &rotation_hash, &parent_id);

								let dh = DoubleHash { pre_hash: metadata.pre_hash, obj_hash: hashes[0] };
								let rndx = dh.calc_hash_randomx();
								if let Err(e) = rndx {
									error!(">>> {}", e);
									break
								}
								let poscan_hash = rndx.unwrap();
								let psdata = PoscanData::V2(PoscanDataV2 {
									alg_id: mining_algo.clone(),
									orig_hashes: orig_hashes.clone(),
									hashes: hashes.clone(),
									obj: mp.pre_obj.clone(),
								});

								let rndx = randomx(orig_hashes[0].0.as_slice());
								if let Err(e) = rndx {
									error!(">>> {}", e);
									break
								}
								let compute = ComputeV2 {
									difficulty: metadata.difficulty,
									pre_hash: metadata.pre_hash,
									poscan_hash,
									orig_hash: rndx.unwrap(),
									hist_hash,
								};

								let signature = compute.sign(&pair);
								let seal = compute.seal(signature.clone());
								if hash_meets_difficulty(&seal.work, seal.difficulty) {
									info!(">>> hash_meets_difficulty: submit it: {}, {}, {}",  &seal.work, &seal.poscan_hash, &seal.difficulty);
									let _ = futures::executor::block_on(worker.submit(seal.encode(), &psdata));
								}
							}
						}
					}
					else {
						thread::sleep(Duration::new(1, 0));
					}
				} else {
					thread::sleep(Duration::from_millis(10));
				}
			});
		}
	}

	let grandpa_config = sc_finality_grandpa::Config {
		gossip_duration: Duration::from_millis(333),
		justification_period: 2,
		name: None,
		observer_enabled: false,
		keystore: Some(keystore_container.sync_keystore()),
		local_role: role,
		telemetry: telemetry.as_ref().map(|x| x.handle()),
		protocol_name: grandpa_protocol_name,
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
			voting_rule: sc_finality_grandpa::VotingRulesBuilder::default().build(),
			prometheus_registry,
			shared_voter_state: shared_voter_state.clone(),
			telemetry: telemetry.as_ref().map(|x| x.handle()),
		};

		// the GRANDPA voter task is considered infallible, i.e.
		// if it fails we take down the service with it.
		task_manager.spawn_essential_handle().spawn_blocking(
			"grandpa-voter",
			None,
			sc_finality_grandpa::run_grandpa_voter(grandpa_config)?,
		);
	}

	network_starter.start_network();
	Ok(task_manager)
}
