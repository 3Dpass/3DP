// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0
// This file is part of Frontier.
//
// Copyright (c) 2021-2022 Parity Technologies (UK) Ltd.
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

mod mapping_db;
mod meta_db;
mod tests;
pub(crate) mod utils;

use std::{path::PathBuf, str::FromStr, sync::Arc};

use clap::ArgEnum;
use ethereum_types::H256;
use serde::Deserialize;
// Substrate
use sc_cli::{PruningParams, SharedParams};
use sp_runtime::traits::Block as BlockT;

use self::{
	mapping_db::{MappingDb, MappingKey, MappingValue},
	meta_db::{MetaDb, MetaKey, MetaValue},
};

/// Cli tool to interact with the Frontier backend db
#[derive(Debug, Clone, clap::Parser)]
pub struct FrontierDbCmd {
	/// Specify the operation to perform.
	///
	/// Can be one of `create | read | update | delete`.
	#[clap(arg_enum, ignore_case = true, required = true)]
	pub operation: Operation,

	/// Specify the column to query.
	///
	/// Can be one of `meta | block | transaction`.
	#[clap(arg_enum, ignore_case = true, required = true)]
	pub column: Column,

	/// Specify the key to either read or write.
	#[clap(short('k'), long, required = true)]
	pub key: String,

	/// Specify the value to write.
	///
	/// - When `Some`, path to file.
	/// - When `None`, read from stdin.
	///
	/// In any case, payload must be serializable to a known type.
	#[clap(long, parse(from_os_str))]
	pub value: Option<PathBuf>,

	/// Shared parameters
	#[clap(flatten)]
	pub shared_params: SharedParams,

	#[allow(missing_docs)]
	#[clap(flatten)]
	pub pruning_params: PruningParams,
}

#[derive(ArgEnum, Debug, Clone)]
pub enum Operation {
	Create,
	Read,
	Update,
	Delete,
}

#[derive(ArgEnum, Debug, Clone)]
pub enum Column {
	Meta,
	Block,
	Transaction,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum DbValue<H> {
	Meta(MetaValue<H>),
	Mapping(MappingValue<H>),
}

impl FrontierDbCmd {
	pub fn run<C, B: BlockT>(
		&self,
		client: Arc<C>,
		backend: Arc<fc_db::Backend<B>>,
	) -> sc_cli::Result<()>
	where
		C: sp_api::ProvideRuntimeApi<B>,
		C::Api: fp_rpc::EthereumRuntimeRPCApi<B>,
		C: sp_blockchain::HeaderBackend<B>,
	{
		match self.column {
			Column::Meta => {
				// New meta db handler
				let meta_db = MetaDb::new(self, backend);
				// Maybe get a MetaKey
				let key = MetaKey::from_str(&self.key)?;
				// Maybe get a MetaValue
				let value = match utils::maybe_deserialize_value::<B>(
					&self.operation,
					self.value.as_ref(),
				)? {
					Some(DbValue::Meta(value)) => Some(value),
					None => None,
					_ => return Err(format!("Unexpected `{:?}` value", self.value).into()),
				};
				// Run the query
				meta_db.query(&key, &value)?
			}
			Column::Block | Column::Transaction => {
				// New mapping db handler
				let mapping_db = MappingDb::new(self, client, backend);
				// Maybe get a MappingKey
				let key = MappingKey::EthBlockOrTransactionHash(
					H256::from_str(&self.key).expect("H256 provided key"),
				);
				// Maybe get a MappingValue
				let value = match utils::maybe_deserialize_value::<B>(
					&self.operation,
					self.value.as_ref(),
				)? {
					Some(DbValue::Mapping(value)) => Some(value),
					None => None,
					_ => return Err(format!("Unexpected `{:?}` value", self.value).into()),
				};
				// Run the query
				mapping_db.query(&self.column, &key, &value)?
			}
		}
		Ok(())
	}
}

impl sc_cli::CliConfiguration for FrontierDbCmd {
	fn shared_params(&self) -> &SharedParams {
		&self.shared_params
	}

	fn pruning_params(&self) -> Option<&PruningParams> {
		Some(&self.pruning_params)
	}
}
