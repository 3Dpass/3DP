//! A collection of node-specific RPC methods.
//! Substrate provides the `sc-rpc` crate, which defines the core RPC layer
//! used by Substrate nodes. This file extends those RPC definitions with
//! capabilities that are specific to this project's runtime configuration.

#![warn(missing_docs)]

use std::{collections::BTreeMap, sync::Arc};
use parking_lot::Mutex;

use runtime::{opaque::Block, AccountId, Balance, BlockNumber, Hash, Index};
use jsonrpsee::RpcModule;
use sc_client_api::{
	backend::{AuxStore, Backend, StateBackend, StorageProvider},
	client::BlockchainEvents,
};


// #[cfg(feature = "manual-seal")]
// use sc_consensus_manual_seal::rpc::{ManualSeal, ManualSealApiServer};
// use sc_network::NetworkService;
// use sc_rpc::SubscriptionTaskExecutor;
// use sc_rpc_api::DenyUnsafe;
// use sc_service::TransactionPool;
// use sc_transaction_pool::{ChainApi, Pool};
// use sp_api::ProvideRuntimeApi;
// use sp_block_builder::BlockBuilder;
// use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
// use sp_runtime::traits::BlakeTwo256;
// // Frontier
// use fc_rpc::{
// 	EthBlockDataCacheTask, OverrideHandle, RuntimeApiStorageOverride, SchemaV1Override,
// 	SchemaV2Override, SchemaV3Override, StorageOverride,
// };
// use fc_rpc_core::types::{FeeHistoryCache, FeeHistoryCacheLimit, FilterPool};
// use fp_storage::EthereumStorageSchema;
// // Runtime
// use frontier_template_runtime::{opaque::Block, AccountId, Balance, Hash, Index};



use sc_client_api::BlockBackend;
use sc_rpc::dev::{Dev, DevApiServer};
use sc_finality_grandpa::FinalityProofProvider;
use sc_transaction_pool_api::TransactionPool;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
pub use sc_rpc_api::DenyUnsafe;
use sc_finality_grandpa_rpc::{Grandpa, GrandpaApiServer};
use pallet_contracts_rpc::{Contracts, ContractsApiServer};
use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApiServer};
use substrate_frame_rpc_system::{System, SystemApiServer};
use crate::mining_rpc::{MiningRpc, PoscanMiningRpcApiServer};
use sp_consensus_poscan::{MiningPoolApi, PoscanApi};
use crate::pool::MiningPool;
use crate::pool_rpc::{MiningPoolRpc, PoscanPoolRpcApiServer};
use crate::poscan_rpc::{PoscanRpc, PoscanRpcApiServer};

use sc_network::NetworkService;
use sp_runtime::traits::BlakeTwo256;
use sc_transaction_pool::{ChainApi, Pool};

// Frontier
use fc_rpc::{
	EthBlockDataCacheTask, OverrideHandle, RuntimeApiStorageOverride, SchemaV1Override,
	SchemaV2Override, SchemaV3Override, StorageOverride,
};
use fc_rpc_core::types::{FeeHistoryCache, FeeHistoryCacheLimit, FilterPool};
use fp_storage::EthereumStorageSchema;


/// Dependencies for GRANDPA
pub struct GrandpaDeps<B> {
	/// Voting round info.
	pub shared_voter_state: sc_finality_grandpa::SharedVoterState,
	/// Authority set info.
	pub shared_authority_set: sc_finality_grandpa::SharedAuthoritySet<Hash, BlockNumber>,
	/// Receives notifications about justification events from Grandpa.
	pub justification_stream: sc_finality_grandpa::GrandpaJustificationStream<Block>,
	/// Executor to drive the subscription manager in the Grandpa RPC handler.
	pub subscription_executor: sc_rpc::SubscriptionTaskExecutor,
	/// Finality proof provider.
	pub finality_provider: Arc<FinalityProofProvider<B, Block>>,
}

/// Full client dependencies.
pub struct FullDeps<C, P, B, A: ChainApi> {
	/// The client instance to use.
	pub client: Arc<C>,
	/// Transaction pool instance.
	pub pool: Arc<P>,
	/// Graph pool instance.
	pub graph: Arc<Pool<A>>,
	/// Whether to deny unsafe calls
	pub deny_unsafe: DenyUnsafe,
	// /// A command stream to send authoring commands to manual seal consensus engine
	// pub command_sink:Sender<EngineComman>,
	/// GRANDPA specific dependencies.
	pub grandpa: GrandpaDeps<B>,

	pub mining_pool: Option<Arc<Mutex<MiningPool>>>,

	/// The Node authority flag
	pub is_authority: bool,
	/// Whether to enable dev signer
	pub enable_dev_signer: bool,
	/// Network service
	pub network: Arc<NetworkService<Block, Hash>>,
	/// EthFilterApi pool.
	pub filter_pool: Option<FilterPool>,
	/// Backend.
	pub backend: Arc<B>,
	/// Frontier Backend.
	pub frontier_backend: Arc<fc_db::Backend<Block>>,
	/// Maximum number of logs in a query.
	pub max_past_logs: u32,
	/// Fee history cache.
	pub fee_history_cache: FeeHistoryCache,
	/// Maximum fee history cache size.
	pub fee_history_cache_limit: FeeHistoryCacheLimit,
	/// Ethereum data access overrides.
	pub overrides: Arc<OverrideHandle<Block>>,
	/// Cache for Ethereum block data.
	pub block_data_cache: Arc<EthBlockDataCacheTask<Block>>,
}

pub fn overrides_handle<C, BE>(client: Arc<C>) -> Arc<OverrideHandle<Block>>
	where
		C: ProvideRuntimeApi<Block> + StorageProvider<Block, BE> + AuxStore,
		C: HeaderBackend<Block> + HeaderMetadata<Block, Error = BlockChainError>,
		C: Send + Sync + 'static,
		C::Api: sp_api::ApiExt<Block>
		+ fp_rpc::EthereumRuntimeRPCApi<Block>
		+ fp_rpc::ConvertTransactionRuntimeApi<Block>,
		BE: Backend<Block> + 'static,
		BE::State: StateBackend<BlakeTwo256>,
{
	let mut overrides_map = BTreeMap::new();
	overrides_map.insert(
		EthereumStorageSchema::V1,
		Box::new(SchemaV1Override::new(client.clone()))
			as Box<dyn StorageOverride<_> + Send + Sync>,
	);
	overrides_map.insert(
		EthereumStorageSchema::V2,
		Box::new(SchemaV2Override::new(client.clone()))
			as Box<dyn StorageOverride<_> + Send + Sync>,
	);
	overrides_map.insert(
		EthereumStorageSchema::V3,
		Box::new(SchemaV3Override::new(client.clone()))
			as Box<dyn StorageOverride<_> + Send + Sync>,
	);

	Arc::new(OverrideHandle {
		schemas: overrides_map,
		fallback: Box::new(RuntimeApiStorageOverride::new(client)),
	})
}

/// Instantiate all full RPC extensions.
pub fn create_full<C, P, B, A>(
	deps: FullDeps<C, P, B, A>,
) -> Result<RpcModule<()>, Box<dyn std::error::Error + Send + Sync>>
where
	C: ProvideRuntimeApi<Block> + StorageProvider<Block, B> + AuxStore,
	C: BlockchainEvents<Block>,
	C: BlockBackend<Block>,
	C: HeaderBackend<Block> + HeaderMetadata<Block, Error = BlockChainError> + 'static,
	C: Send + Sync + 'static,
	C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Index>,
	C::Api: pallet_contracts_rpc::ContractsRuntimeApi<Block, AccountId, Balance, BlockNumber, Hash>,
	C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
	C::Api: BlockBuilder<Block>,
	C::Api: MiningPoolApi<Block, AccountId>,
	C::Api: PoscanApi<Block, AccountId, BlockNumber>,

	C::Api: BlockBuilder<Block>,
	C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
	C::Api: fp_rpc::ConvertTransactionRuntimeApi<Block>,
	C::Api: fp_rpc::EthereumRuntimeRPCApi<Block>,

	P: TransactionPool<Block = Block> + 'static,
	B: sc_client_api::Backend<Block> + Send + Sync + 'static,
	B::State: StateBackend<BlakeTwo256>,
	A: ChainApi<Block = Block> + 'static,
{
	use fc_rpc::{
		Eth, EthApiServer, EthDevSigner, EthFilter, EthFilterApiServer,
		EthPubSub,
		EthPubSubApiServer,
		EthSigner, Net, NetApiServer,
		Web3,
		Web3ApiServer,
	};

	let mut module = RpcModule::new(());
	let FullDeps {
		client,
		pool,
		graph,
		deny_unsafe,
		grandpa,
		mining_pool,
		network,
		filter_pool,
		backend,
		frontier_backend,
		is_authority,
		enable_dev_signer,
		max_past_logs,
		fee_history_cache,
		fee_history_cache_limit,
		overrides,
		block_data_cache,
	} = deps;

	let GrandpaDeps {
		shared_voter_state,
		shared_authority_set,
		justification_stream,
		subscription_executor,
		finality_provider,
	} = grandpa;

	module.merge(System::new(client.clone(), pool.clone(), deny_unsafe).into_rpc())?;
	module.merge(TransactionPayment::new(client.clone()).into_rpc())?;
	module.merge(
		Grandpa::new(
			subscription_executor.clone(),
			shared_authority_set.clone(),
			shared_voter_state,
			justification_stream,
			finality_provider,
		)
		.into_rpc(),
	)?;

	module.merge(MiningRpc::new(client.clone()).into_rpc())?;

	if let Some(mining_pool) = mining_pool {
	 	module.merge(MiningPoolRpc::<C, Block, B>::new(client.clone(), backend.clone(), mining_pool.clone()).into_rpc())?;
	}

	module.merge(PoscanRpc::<C, Block>::new(client.clone()).into_rpc())?;

	// Add a silly RPC that returns constant values
	// io.extend_with(crate::mining_rpc::PoscanMiningRpc::to_delegate(
	// 	crate::mining_rpc::MiningRpc::<C, Block>::new(deps.client.clone()),
	// ));

	// Extend this RPC with a custom API by using the following syntax.
	// `YourRpcStruct` should have a reference to a client, which is needed
	// to call into the runtime.
	// `module.merge(YourRpcTrait::into_rpc(YourRpcStruct::new(ReferenceToClient, ...)))?;`

	// Contracts RPC API extension
	module.merge(Contracts::new(client.clone()).into_rpc())?;

	// Dev RPC API extension
	module.merge(Dev::new(client.clone(), deny_unsafe).into_rpc())?;

	let mut signers = Vec::new();
	if enable_dev_signer {
		signers.push(Box::new(EthDevSigner::new()) as Box<dyn EthSigner>);
	}

	module.merge(
		Eth::new(
			client.clone(),
			pool.clone(),
			graph,
			Some(runtime::TransactionConverter),
			network.clone(),
			signers,
			overrides.clone(),
			frontier_backend.clone(),
			// Is authority.
			is_authority,
			block_data_cache.clone(),
			fee_history_cache,
			fee_history_cache_limit,
			10,
		)
			.into_rpc(),
	)?;

	if let Some(filter_pool) = filter_pool {
		module.merge(
			EthFilter::new(
				client.clone(),
				frontier_backend,
				filter_pool,
				500_usize, // max stored filters
				max_past_logs,
				block_data_cache,
			)
				.into_rpc(),
		)?;
	}

	module.merge(
		EthPubSub::new(
			pool,
			client.clone(),
			network.clone(),
			subscription_executor,
			overrides,
		)
			.into_rpc(),
	)?;

	module.merge(
		Net::new(
			client.clone(),
			network,
			// Whether to format the `peer_count` response as Hex (default) or not.
			true,
		)
			.into_rpc(),
	)?;

	module.merge(Web3::new(client.clone()).into_rpc())?;


	Ok(module)
}
