// Copyright 2017-2020 Parity Technologies (UK) Ltd.
// This file is part of Substrate.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

use crate::chain_spec;
use crate::cli::{Cli, Subcommand};
use crate::service;

use log::*;
use sp_core::{hexdisplay::HexDisplay, crypto::{Pair, Ss58Codec, Ss58AddressFormat}};
use sp_core::crypto::set_default_ss58_version;
use sp_keystore::SyncCryptoStore;
use sc_cli::{SubstrateCli, ChainSpec, RuntimeVersion};
use sc_service::{PartialComponents, config::KeystoreConfig};
use sc_keystore::LocalKeystore;
use sp_consensus_poscan::POSCAN_COIN_ID;

impl SubstrateCli for Cli {
	fn impl_name() -> String {
		"Proof Of Scan Consensus Node (PoScan and Grandpa)".into()
	}

	fn impl_version() -> String {
		env!("SUBSTRATE_CLI_IMPL_VERSION").into()
	}

	fn description() -> String {
		env!("CARGO_PKG_DESCRIPTION").into()
	}

	fn author() -> String {
		env!("CARGO_PKG_AUTHORS").into()
	}

	fn support_url() -> String {
		"https://github.com/3Dpass/3DP/issues".into()
	}

	fn copyright_start_year() -> i32 {
		2019
	}

	fn load_spec(&self, id: &str) -> Result<Box<dyn sc_service::ChainSpec>, String> {
		Ok(match id {
			"dev" => Box::new(chain_spec::dev_config()?),
			"" | "local" => Box::new(chain_spec::local_testnet_config()?),
			path => Box::new(chain_spec::ChainSpec::from_json_file(
				std::path::PathBuf::from(path),
			)?),
		})
	}

	fn native_runtime_version(_: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
		&runtime::VERSION
	}
}

/// Parse and run command line arguments
pub fn run() -> sc_cli::Result<()> {
	let cli = Cli::from_args();

	set_default_ss58_version(Ss58AddressFormat::from(POSCAN_COIN_ID));

	match &cli.subcommand {
		Some(Subcommand::Key(cmd)) => cmd.run(&cli),
		Some(Subcommand::BuildSpec(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run(config.chain_spec, config.network))
		}
		Some(Subcommand::CheckBlock(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents {
					client,
					task_manager,
					import_queue,
					..
				} = service::new_partial(&config, None)?;
				Ok((cmd.run(client, import_queue), task_manager))
			})
		}
		Some(Subcommand::ExportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents {
					client,
					task_manager,
					..
				} = service::new_partial(&config, None)?;
				Ok((cmd.run(client, config.database), task_manager))
			})
		}
		Some(Subcommand::ExportState(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents {
					client,
					task_manager,
					..
				} = service::new_partial(&config, None)?;
				Ok((cmd.run(client, config.chain_spec), task_manager))
			})
		}
		Some(Subcommand::ImportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents {
					client,
					task_manager,
					import_queue,
					..
				} = service::new_partial(&config, None)?;
				Ok((cmd.run(client, import_queue), task_manager))
			})
		}
		Some(Subcommand::PurgeChain(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run(config.database))
		}
		Some(Subcommand::Revert(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents {
					client,
					task_manager,
					backend,
					..
				} = service::new_partial(&config, None)?;
				// TODO:!!! None ?`
				Ok((cmd.run(client, backend, None), task_manager))
			})
		}
		Some(Subcommand::ImportMiningKey(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| {
				let keystore = match &config.keystore {
					KeystoreConfig::Path { path, password } => LocalKeystore::open(
						path.clone(),
						password.clone()
					).map_err(|e| format!("Open keystore failed: {:?}", e))?,
					KeystoreConfig::InMemory => LocalKeystore::in_memory(),
				};

				let pair = sc_consensus_poscan::app::Pair::from_string(
					&cmd.suri,
					None,
				).map_err(|e| format!("Invalid seed: {:?}", e))?;

				SyncCryptoStore::insert_unknown(
					&keystore,
					sc_consensus_poscan::app::ID,
					&cmd.suri,
					pair.public().as_ref(),
				).map_err(|e| format!("Registering mining key failed: {:?}", e))?;

				println!(
					"Public key: 0x{}\nSecret seed: {}\nAddress: {}",
					HexDisplay::from(&pair.public().as_ref()),
					cmd.suri,
					// TODO: kulupu -> 3dp
					pair.public().to_ss58check_with_version(Ss58AddressFormat::from(POSCAN_COIN_ID)),
				);

				// for i in 0..16383 {
				// 	println!("{} - Address: 0x{}",
				// 		i, pair.public().to_ss58check_with_version(Ss58AddressFormat::Custom(i))
				// 	);
				// }

				info!("Registered one mining key (public key 0x{}).",
					  HexDisplay::from(&pair.public().as_ref()));

				Ok(())
			})
		},
		Some(Subcommand::GenerateMiningKey(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| {
				let keystore = match &config.keystore {
					KeystoreConfig::Path { path, password } => LocalKeystore::open(
						path.clone(),
						password.clone()
					).map_err(|e| format!("Open keystore failed: {:?}", e))?,
					KeystoreConfig::InMemory => {
						info!(">>> in memory keystore");
						LocalKeystore::in_memory()
					},
				};

				let (pair, phrase, _) = sc_consensus_poscan::app::Pair::generate_with_phrase(None);

				SyncCryptoStore::insert_unknown(
					&keystore,
					sc_consensus_poscan::app::ID,
					&phrase,
					pair.public().as_ref(),
				).map_err(|e| format!("Registering mining key failed: {:?}", e))?;

				info!("Generated one mining key.");

				println!(
					"Public key: 0x{}\nSecret seed: {}\nAddress: {}",
					HexDisplay::from(&pair.public().as_ref()),
					phrase,
					// TODO: kulupu -> 3dp
					pair.public().to_ss58check_with_version(Ss58AddressFormat::from(POSCAN_COIN_ID)),
				);

				let _pair = keystore.key_pair::<sc_consensus_poscan::app::Pair>(
					&pair.public(),
				).map_err(|_e|
					format!("Unable to mine: fetch pair from author failed")
				)?;

				Ok(())
			})
		},
		None => {
			let runner = cli.create_runner(&cli.run)?;
			runner.run_node_until_exit(|config| async move {
				match config.role {
					// Role::Light => service::new_light(config),
					_ => service::new_full(
						config,
						cli.author.as_ref().map(|s| s.as_str()),
						cli.threads.unwrap_or(1),
					),
				}
				.map_err(sc_cli::Error::Service)
			})
		}
	}
}
