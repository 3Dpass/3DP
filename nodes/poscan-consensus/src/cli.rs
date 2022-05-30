use sc_cli::RunCmd;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct Cli {
	#[structopt(subcommand)]
	pub subcommand: Option<Subcommand>,

	#[structopt(flatten)]
	pub run: RunCmd,

	#[structopt(long)]
	pub author: Option<String>,
}

#[derive(Debug, StructOpt)]
pub enum Subcommand {
	/// Key management cli utilities
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

	#[structopt(name = "import-mining-key")]
	ImportMiningKey(ImportMiningKeyCommand),

	#[structopt(name = "generate-mining-key")]
	GenerateMiningKey(GenerateMiningKeyCommand),
}

#[derive(Debug, StructOpt)]
pub struct ImportMiningKeyCommand {
	#[structopt()]
	pub suri: String,

	#[allow(missing_docs)]
	#[structopt(flatten)]
	pub shared_params: sc_cli::SharedParams,

	#[allow(missing_docs)]
	#[structopt(flatten)]
	pub keystore_params: sc_cli::KeystoreParams,
}

impl sc_cli::CliConfiguration for ImportMiningKeyCommand {
	fn shared_params(&self) -> &sc_cli::SharedParams { &self.shared_params }
	fn keystore_params(&self) -> Option<&sc_cli::KeystoreParams> { Some(&self.keystore_params) }
}

#[derive(Debug, StructOpt)]
pub struct GenerateMiningKeyCommand {
	#[allow(missing_docs)]
	#[structopt(flatten)]
	pub shared_params: sc_cli::SharedParams,

	#[allow(missing_docs)]
	#[structopt(flatten)]
	pub keystore_params: sc_cli::KeystoreParams,
}

impl sc_cli::CliConfiguration for GenerateMiningKeyCommand {
	fn shared_params(&self) -> &sc_cli::SharedParams { &self.shared_params }
	fn keystore_params(&self) -> Option<&sc_cli::KeystoreParams> { Some(&self.keystore_params) }
}
