// This file is part of 3DPass.
//
// Copyright (c) 2020-2022 Parity Technologies (UK) Ltd.
// Copyright (c) 2025 3DPass
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use std::{collections::HashMap, marker::PhantomData, sync::Arc};

// Substrate
use sc_client_api::{backend::AuxStore, BlockOf};
use sc_consensus::{BlockCheckParams, BlockImport, BlockImportParams, ImportResult};
use sp_api::{ProvideRuntimeApi, BlockId, Core};
use sp_block_builder::BlockBuilder as BlockBuilderApi;
use sp_blockchain::{well_known_cache_keys::Id as CacheKeyId, HeaderBackend};
use sp_consensus::Error as ConsensusError;
use sp_runtime::traits::{Block as BlockT, Header as HeaderT};
// Frontier
use fp_consensus::{ensure_log, FindLogError};
use fp_rpc::EthereumRuntimeRPCApi;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("Multiple runtime Ethereum blocks, rejecting!")]
	MultipleRuntimeLogs,
	#[error("Runtime Ethereum block not found, rejecting!")]
	NoRuntimeLog,
	#[error("Cannot access the runtime at genesis, rejecting!")]
	RuntimeApiCallFailed,
}

impl From<Error> for String {
	fn from(error: Error) -> String {
		error.to_string()
	}
}

impl From<FindLogError> for Error {
	fn from(error: FindLogError) -> Error {
		match error {
			FindLogError::NotFound => Error::NoRuntimeLog,
			FindLogError::MultipleLogs => Error::MultipleRuntimeLogs,
		}
	}
}

impl From<Error> for ConsensusError {
	fn from(error: Error) -> ConsensusError {
		ConsensusError::ClientImport(error.to_string())
	}
}

pub struct FrontierBlockImport<B: BlockT, I, C> {
	inner: I,
	client: Arc<C>,
	backend: Arc<fc_db::Backend<B>>,
	_marker: PhantomData<B>,
}

impl<Block: BlockT, I: Clone + BlockImport<Block>, C> Clone for FrontierBlockImport<Block, I, C> {
	fn clone(&self) -> Self {
		FrontierBlockImport {
			inner: self.inner.clone(),
			client: self.client.clone(),
			backend: self.backend.clone(),
			_marker: PhantomData,
		}
	}
}

impl<B, I, C> FrontierBlockImport<B, I, C>
where
	B: BlockT,
	I: BlockImport<B, Transaction = sp_api::TransactionFor<C, B>> + Send + Sync,
	I::Error: Into<ConsensusError>,
	C: ProvideRuntimeApi<B> + Send + Sync + HeaderBackend<B> + AuxStore + BlockOf,
	C::Api: EthereumRuntimeRPCApi<B>,
	C::Api: BlockBuilderApi<B>,
{
	pub fn new(inner: I, client: Arc<C>, backend: Arc<fc_db::Backend<B>>) -> Self {
		Self {
			inner,
			client,
			backend,
			_marker: PhantomData,
		}
	}
}

#[async_trait::async_trait]
impl<B, I, C> BlockImport<B> for FrontierBlockImport<B, I, C>
where
	B: BlockT,
	I: BlockImport<B, Transaction = sp_api::TransactionFor<C, B>> + Send + Sync,
	I::Error: Into<ConsensusError>,
	C: ProvideRuntimeApi<B> + Send + Sync + HeaderBackend<B> + AuxStore + BlockOf,
	C::Api: EthereumRuntimeRPCApi<B>,
	C::Api: BlockBuilderApi<B>,
{
	type Error = ConsensusError;
	type Transaction = sp_api::TransactionFor<C, B>;

	async fn check_block(
		&mut self,
		block: BlockCheckParams<B>,
	) -> Result<ImportResult, Self::Error> {

		let parent_id: BlockId<B> = BlockId::hash(block.parent_hash);

		let ver = self.client
			.runtime_api()
			.version(&parent_id)
			.map_err(|_err| Error::RuntimeApiCallFailed)?;

		if ver.spec_version <= 128u32 {
			return Ok(ImportResult::imported(true))
		}

		self.inner.check_block(block).await.map_err(Into::into)
	}

	async fn import_block(
		&mut self,
		block: BlockImportParams<B, Self::Transaction>,
		new_cache: HashMap<CacheKeyId, Vec<u8>>,
	) -> Result<ImportResult, Self::Error> {

		let parent_id: BlockId<B> = BlockId::hash(*block.header.parent_hash());

		let ver = self.client
			.runtime_api()
			.version(&parent_id)
			.map_err(|_err| Error::RuntimeApiCallFailed)?;

		if ver.spec_version <= 128u32 {
			return Ok(ImportResult::imported(true))
		}

		// We validate that there are only one frontier log. No other
		// actions are needed and mapping syncing is delegated to a separate
		// worker.
		ensure_log(block.header.digest()).map_err(Error::from)?;

		self.inner
			.import_block(block, new_cache)
			.await
			.map_err(Into::into)
	}
}
