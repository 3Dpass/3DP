use sc_cli::RunCmd;
// use std::str::FromStr;
// use structopt::StructOpt;


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
