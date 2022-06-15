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
use sp_core::crypto::{Ss58Codec,UncheckedFrom, Ss58AddressFormat, set_default_ss58_version};
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
	set_default_ss58_version(Ss58AddressFormat::Custom(POSCAN_COIN_ID.into()));

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
//
// use sc_network::{
// 	NetworkService,
//
// };
//
// use sc_service::{
// 	BuildNetworkParams,
// 	NetworkStatusSinks,
// 	NetworkStarter,
// 	TransactionPoolAdapter,
// };
// use sp_api::ProvideRuntimeApi;
// use sp_blockchain::{HeaderBackend, HeaderMetadata};
// use sp_consensus::block_validation::Chain;
//
// use sc_client_api::proof_provider::ProofProvider;
// use sp_transaction_pool::MaintainedTransactionPool;
// use sp_consensus::block_validation::DefaultBlockAnnounceValidator;
// use sp_runtime::traits::{Block as BlockT, BlockIdTo};
// use sp_consensus::import_queue::ImportQueue;
// use sc_client_api::{BlockBackend, BlockchainEvents};
// use sc_network::block_request_handler::{self, BlockRequestHandler};
// use sc_network::light_client_requests::{self, handler::LightClientRequestHandler};
// use sc_service::error::Error;
// use sc_network::config::Role;
// use futures::channel::oneshot;
// use std::task::Poll;
// use futures::{Future, FutureExt, Stream, StreamExt, stream, compat::*};
// use sp_utils::{mpsc::{tracing_unbounded, TracingUnboundedReceiver, TracingUnboundedSender}};
// use std::{pin::Pin};
// use sc_network::{NetworkStatus, network_state::NetworkState, PeerId};
//
//
// async fn build_network_future<
// 	B: BlockT,
// 	C: BlockchainEvents<B> + HeaderBackend<B>,
// 	H: sc_network::ExHashT
// > (
// 	role: Role,
// 	mut network: sc_network::NetworkWorker<B, H>,
// 	client: Arc<C>,
// 	status_sinks: NetworkStatusSinks<B>,
// 	mut rpc_rx: TracingUnboundedReceiver<sc_rpc::system::Request<B>>,
// 	should_have_peers: bool,
// 	announce_imported_blocks: bool,
// ) {
// 	let mut imported_blocks_stream = client.import_notification_stream().fuse();
//
// 	// Current best block at initialization, to report to the RPC layer.
// 	let starting_block = client.info().best_number;
//
// 	// Stream of finalized blocks reported by the client.
// 	let mut finality_notification_stream = {
// 		let mut finality_notification_stream = client.finality_notification_stream().fuse();
//
// 		// We tweak the `Stream` in order to merge together multiple items if they happen to be
// 		// ready. This way, we only get the latest finalized block.
// 		stream::poll_fn(move |cx| {
// 			let mut last = None;
// 			while let Poll::Ready(Some(item)) = Pin::new(&mut finality_notification_stream).poll_next(cx) {
// 				last = Some(item);
// 			}
// 			if let Some(last) = last {
// 				Poll::Ready(Some(last))
// 			} else {
// 				Poll::Pending
// 			}
// 		}).fuse()
// 	};
//
// 	loop {
// 		futures::select!{
// 			// List of blocks that the client has imported.
// 			notification = imported_blocks_stream.next() => {
// 				let notification = match notification {
// 					Some(n) => n,
// 					// If this stream is shut down, that means the client has shut down, and the
// 					// most appropriate thing to do for the network future is to shut down too.
// 					None => return,
// 				};
//
// 				if announce_imported_blocks {
// 					network.service().announce_block(notification.hash, None);
// 				}
//
// 				if notification.is_new_best {
// 					network.service().new_best_block_imported(
// 						notification.hash,
// 						notification.header.number().clone(),
// 					);
// 				}
// 			}
//
// 			// List of blocks that the client has finalized.
// 			notification = finality_notification_stream.select_next_some() => {
// 				network.on_block_finalized(notification.hash, notification.header);
// 			}
//
// 			// Answer incoming RPC requests.
// 			request = rpc_rx.select_next_some() => {
// 				match request {
// 					sc_rpc::system::Request::Health(sender) => {
// 						let _ = sender.send(sc_rpc::system::Health {
// 							peers: network.peers_debug_info().len(),
// 							is_syncing: network.service().is_major_syncing(),
// 							should_have_peers,
// 						});
// 					},
// 					sc_rpc::system::Request::LocalPeerId(sender) => {
// 						let _ = sender.send(network.local_peer_id().to_base58());
// 					},
// 					sc_rpc::system::Request::LocalListenAddresses(sender) => {
// 						let peer_id = network.local_peer_id().clone().into();
// 						let p2p_proto_suffix = sc_network::multiaddr::Protocol::P2p(peer_id);
// 						let addresses = network.listen_addresses()
// 							.map(|addr| addr.clone().with(p2p_proto_suffix.clone()).to_string())
// 							.collect();
// 						let _ = sender.send(addresses);
// 					},
// 					sc_rpc::system::Request::Peers(sender) => {
// 						let _ = sender.send(network.peers_debug_info().into_iter().map(|(peer_id, p)|
// 							sc_rpc::system::PeerInfo {
// 								peer_id: peer_id.to_base58(),
// 								roles: format!("{:?}", p.roles),
// 								best_hash: p.best_hash,
// 								best_number: p.best_number,
// 							}
// 						).collect());
// 					}
// 					sc_rpc::system::Request::NetworkState(sender) => {
// 						if let Some(network_state) = serde_json::to_value(&network.network_state()).ok() {
// 							let _ = sender.send(network_state);
// 						}
// 					}
// 					sc_rpc::system::Request::NetworkAddReservedPeer(peer_addr, sender) => {
// 						let x = network.add_reserved_peer(peer_addr)
// 							.map_err(sc_rpc::system::error::Error::MalformattedPeerArg);
// 						let _ = sender.send(x);
// 					}
// 					sc_rpc::system::Request::NetworkRemoveReservedPeer(peer_id, sender) => {
// 						let _ = match peer_id.parse::<PeerId>() {
// 							Ok(peer_id) => {
// 								network.remove_reserved_peer(peer_id);
// 								sender.send(Ok(()))
// 							}
// 							Err(e) => sender.send(Err(sc_rpc::system::error::Error::MalformattedPeerArg(
// 								e.to_string(),
// 							))),
// 						};
// 					}
// 					sc_rpc::system::Request::NodeRoles(sender) => {
// 						use sc_rpc::system::NodeRole;
//
// 						let node_role = match role {
// 							Role::Authority { .. } => NodeRole::Authority,
// 							Role::Light => NodeRole::LightClient,
// 							Role::Full => NodeRole::Full,
// 							Role::Sentry { .. } => NodeRole::Sentry,
// 						};
//
// 						let _ = sender.send(vec![node_role]);
// 					}
// 					sc_rpc::system::Request::SyncState(sender) => {
// 						use sc_rpc::system::SyncState;
//
// 						let _ = sender.send(SyncState {
// 							starting_block: starting_block,
// 							current_block: client.info().best_number,
// 							highest_block: network.best_seen_block(),
// 						});
// 					}
// 				}
// 			}
//
// 			// The network worker has done something. Nothing special to do, but could be
// 			// used in the future to perform actions in response of things that happened on
// 			// the network.
// 			_ = (&mut network).fuse() => {}
//
// 			// At a regular interval, we send high-level status as well as
// 			// detailed state information of the network on what are called
// 			// "status sinks".
//
// 			status_sink = status_sinks.status.next().fuse() => {
// 				status_sink.send(network.status());
// 			}
//
// 			state_sink = status_sinks.state.next().fuse() => {
// 				state_sink.send(network.network_state());
// 			}
// 		}
// 	}
// }
//
// pub fn build_network<TBl, TExPool, TImpQu, TCl>(
// 	params: BuildNetworkParams<TBl, TExPool, TImpQu, TCl>
// ) -> Result<
// 	(
// 		Arc<NetworkService<TBl, <TBl as BlockT>::Hash>>,
// 		NetworkStatusSinks<TBl>,
// 		TracingUnboundedSender<sc_rpc::system::Request<TBl>>,
// 		NetworkStarter,
// 	),
// 	Error
// >
// 	where
// 		TBl: BlockT,
// 		TCl: ProvideRuntimeApi<TBl> + HeaderMetadata<TBl, Error=sp_blockchain::Error> + Chain<TBl> +
// 		BlockBackend<TBl> + BlockIdTo<TBl, Error=sp_blockchain::Error> + ProofProvider<TBl> +
// 		HeaderBackend<TBl> + BlockchainEvents<TBl> + 'static,
// 		TExPool: MaintainedTransactionPool<Block=TBl, Hash = <TBl as BlockT>::Hash> + 'static,
// 		TImpQu: ImportQueue<TBl> + 'static,
// {
// 	let BuildNetworkParams {
// 		config, client, transaction_pool, spawn_handle, import_queue, on_demand,
// 		block_announce_validator_builder,
// 	} = params;
//
// 	let transaction_pool_adapter = Arc::new(TransactionPoolAdapter {
// 		imports_external_transactions: !matches!(config.role, Role::Light),
// 		pool: transaction_pool,
// 		client: client.clone(),
// 	});
//
// 	let protocol_id = config.protocol_id();
//
// 	let block_announce_validator = if let Some(f) = block_announce_validator_builder {
// 		f(client.clone())
// 	} else {
// 		Box::new(DefaultBlockAnnounceValidator)
// 	};
//
// 	let block_request_protocol_config = {
// 		if matches!(config.role, Role::Light) {
// 			// Allow outgoing requests but deny incoming requests.
// 			block_request_handler::generate_protocol_config(&protocol_id)
// 		} else {
// 			// Allow both outgoing and incoming requests.
// 			let (handler, protocol_config) = BlockRequestHandler::new(
// 				&protocol_id,
// 				client.clone(),
// 			);
// 			spawn_handle.spawn("block_request_handler", handler.run());
// 			protocol_config
// 		}
// 	};
//
// 	let light_client_request_protocol_config = {
// 		if matches!(config.role, Role::Light) {
// 			// Allow outgoing requests but deny incoming requests.
// 			light_client_requests::generate_protocol_config(&protocol_id)
// 		} else {
// 			// Allow both outgoing and incoming requests.
// 			let (handler, protocol_config) = LightClientRequestHandler::new(
// 				&protocol_id,
// 				client.clone(),
// 			);
// 			spawn_handle.spawn("light_client_request_handler", handler.run());
// 			protocol_config
// 		}
// 	};
//
// 	let network_params = sc_network::config::Params {
// 		role: config.role.clone(),
// 		executor: {
// 			let spawn_handle = Clone::clone(&spawn_handle);
// 			Some(Box::new(move |fut| {
// 				spawn_handle.spawn("libp2p-node", fut);
// 			}))
// 		},
// 		network_config: config.network.clone(),
// 		chain: client.clone(),
// 		on_demand: on_demand,
// 		transaction_pool: transaction_pool_adapter as _,
// 		import_queue: Box::new(import_queue),
// 		protocol_id,
// 		block_announce_validator,
// 		metrics_registry: config.prometheus_config.as_ref().map(|config| config.registry.clone()),
// 		block_request_protocol_config,
// 		light_client_request_protocol_config,
// 	};
//
// 	let has_bootnodes = !network_params.network_config.boot_nodes.is_empty();
// 	let network_mut = sc_network::NetworkWorker::new(network_params)?;
// 	let network = network_mut.service().clone();
// 	let network_status_sinks = NetworkStatusSinks::new();
//
// 	let (system_rpc_tx, system_rpc_rx) = tracing_unbounded("mpsc_system_rpc");
//
// 	let future = build_network_future(
// 		config.role.clone(),
// 		network_mut,
// 		client,
// 		network_status_sinks.clone(),
// 		system_rpc_rx,
// 		has_bootnodes,
// 		config.announce_block,
// 	);
//
// 	// TODO: Normally, one is supposed to pass a list of notifications protocols supported by the
// 	// node through the `NetworkConfiguration` struct. But because this function doesn't know in
// 	// advance which components, such as GrandPa or Polkadot, will be plugged on top of the
// 	// service, it is unfortunately not possible to do so without some deep refactoring. To bypass
// 	// this problem, the `NetworkService` provides a `register_notifications_protocol` method that
// 	// can be called even after the network has been initialized. However, we want to avoid the
// 	// situation where `register_notifications_protocol` is called *after* the network actually
// 	// connects to other peers. For this reason, we delay the process of the network future until
// 	// the user calls `NetworkStarter::start_network`.
// 	//
// 	// This entire hack should eventually be removed in favour of passing the list of protocols
// 	// through the configuration.
// 	//
// 	// See also https://github.com/paritytech/substrate/issues/6827
// 	let (network_start_tx, network_start_rx) = oneshot::channel();
//
// 	// The network worker is responsible for gathering all network messages and processing
// 	// them. This is quite a heavy task, and at the time of the writing of this comment it
// 	// frequently happens that this future takes several seconds or in some situations
// 	// even more than a minute until it has processed its entire queue. This is clearly an
// 	// issue, and ideally we would like to fix the network future to take as little time as
// 	// possible, but we also take the extra harm-prevention measure to execute the networking
// 	// future using `spawn_blocking`.
// 	spawn_handle.spawn_blocking("network-worker", async move {
// 		if network_start_rx.await.is_err() {
// 			debug_assert!(false);
// 			log::warn!(
// 				"The NetworkStart returned as part of `build_network` has been silently dropped"
// 			);
// 			// This `return` might seem unnecessary, but we don't want to make it look like
// 			// everything is working as normal even though the user is clearly misusing the API.
// 			return;
// 		}
//
// 		future.await
// 	});
//
// 	Ok((network, network_status_sinks, system_rpc_tx, NetworkStarter(network_start_tx)))
// }

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

	// let mut proto = sc_finality_grandpa_warp_sync::request_response_config_for_chain(
	// 	&config, task_manager.spawn_handle(), backend.clone(),
	// );
	//
	// proto.max_response_size = 64 * 1024 * 1024;
	// config.network.request_response_protocols.push(proto);

	// let mut cfg = sc_network::block_request_handler::generate_protocol_config(&config.protocol_id());
	// cfg.max_response_size = 64 * 1024 * 1024;

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

	// let _ = network.request_response_protocols;

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
		justification_period: 2,
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
