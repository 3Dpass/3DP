//use sc_cli::RunCmd;
// use std::str::FromStr;
// use structopt::StructOpt;

#[allow(missing_docs)]
#[derive(Debug, clap::Parser)]
pub struct RunCmd {
	#[allow(missing_docs)]
	#[clap(flatten)]
	pub base: sc_cli::RunCmd,

	/// Choose sealing method.
	#[cfg(feature = "manual-seal")]
	#[clap(long, arg_enum, ignore_case = true)]
	pub sealing: Sealing,

	#[clap(long)]
	pub enable_dev_signer: bool,

	/// Maximum number of logs in a query.
	#[clap(long, default_value = "10000")]
	pub max_past_logs: u32,

	/// Maximum fee history cache size.
	#[clap(long, default_value = "2048")]
	pub fee_history_limit: u64,

	/// The dynamic-fee pallet target gas price set by block author
	#[clap(long, default_value = "1")]
	pub target_gas_price: u64,
}

#[derive(Debug, clap::Parser)]
pub struct Cli {
	#[clap(subcommand)]
	pub subcommand: Option<Subcommand>,

	#[clap(flatten)]
	pub run: RunCmd,

	#[clap(long)]
	pub author: Option<String>,

	#[clap(long)]
	pub threads: Option<usize>,
}

#[derive(Debug, clap::Parser)]
pub enum Subcommand {
	/// Key management cli utilities
	#[clap(subcommand)]
	Key(sc_cli::KeySubcommand),

	/// Build a chain specification.
	BuildSpec(sc_cli::BuildSpecCmd),

	/// Validate blocks.
	CheckBlock(sc_cli::CheckBlockCmd),

	/// Export blocks.
	ExportBlocks(sc_cli::ExportBlocksCmd),

	/// Export the state of a given block into a chain spec.
	ExportState(sc_cli::ExportStateCmd),

	/// Import blocks.
	ImportBlocks(sc_cli::ImportBlocksCmd),

	/// Remove the whole chain.
	PurgeChain(sc_cli::PurgeChainCmd),

	/// Revert the chain to a previous state.
	Revert(sc_cli::RevertCmd),

	#[clap(name = "import-mining-key")]
	ImportMiningKey(ImportMiningKeyCommand),

	#[clap(name = "generate-mining-key")]
	GenerateMiningKey(GenerateMiningKeyCommand),

	/// Db meta columns information.
	FrontierDb(fc_cli::FrontierDbCmd),
}

#[derive(Debug, clap::Parser)]
pub struct ImportMiningKeyCommand {
	#[clap()]
	pub suri: String,

	#[allow(missing_docs)]
	#[clap(flatten)]
	pub shared_params: sc_cli::SharedParams,

	#[allow(missing_docs)]
	#[clap(flatten)]
	pub keystore_params: sc_cli::KeystoreParams,
}

impl sc_cli::CliConfiguration for ImportMiningKeyCommand {
	fn shared_params(&self) -> &sc_cli::SharedParams { &self.shared_params }
	fn keystore_params(&self) -> Option<&sc_cli::KeystoreParams> { Some(&self.keystore_params) }
}

#[derive(Debug, clap::Parser)]
pub struct GenerateMiningKeyCommand {
	#[allow(missing_docs)]
	#[clap(flatten)]
	pub shared_params: sc_cli::SharedParams,

	#[allow(missing_docs)]
	#[clap(flatten)]
	pub keystore_params: sc_cli::KeystoreParams,
}

impl sc_cli::CliConfiguration for GenerateMiningKeyCommand {
	fn shared_params(&self) -> &sc_cli::SharedParams { &self.shared_params }
	fn keystore_params(&self) -> Option<&sc_cli::KeystoreParams> { Some(&self.keystore_params) }
}
