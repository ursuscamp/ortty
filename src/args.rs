use std::path::PathBuf;

use bitcoin::{BlockHash, Txid};
use bitcoincore_rpc::Auth;

#[derive(clap::Parser, Debug)]
pub struct Args {
    /// Host name/IP address of Bitcoin full node
    #[arg(long, env = "BITCOIN_HOST")]
    pub host: Option<String>,

    /// Username for RPC user (if applicable)
    #[arg(long, env = "BITCOIN_USER")]
    pub user: Option<String>,

    /// Password for RPC user (if applicable)
    #[arg(long, env = "BITCOIN_PASS")]
    pub password: Option<String>,

    /// Path to RPC cookie file (if applicable)
    #[arg(long, env = "BITCOIN_COOKIE")]
    pub cookie: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

impl Args {
    pub fn rpc_host(&self) -> String {
        match &self.host {
            Some(host) => host.clone(),
            None => "localhost".into(),
        }
    }

    pub fn rpc_auth(&self) -> Auth {
        if let Some(cookie) = &self.cookie {
            Auth::CookieFile(cookie.clone())
        } else {
            Auth::UserPass(
                self.user.clone().unwrap_or_default(),
                self.password.clone().unwrap_or_default(),
            )
        }
    }

    pub fn scan_mode(&self) -> anyhow::Result<ScanMode> {
        let mode = match self.command {
            Commands::ScanBlock { block } => ScanMode::Block(block),
            Commands::ScanTx {
                tx,
                block,
                input: Some(input),
            } => ScanMode::Input(input, tx, block),
            Commands::ScanTx {
                tx,
                block,
                input: _input,
            } => ScanMode::Transaction(tx, block),
        };
        Ok(mode)
    }
}

#[derive(clap::Subcommand, Debug)]
pub enum Commands {
    /// Scan a full block and print/extract every recognizable Inscription
    ScanBlock {
        /// Blockhash to scan
        block: BlockHash,
    },
    /// Scan a transaction and print/extract every recognizable Inscription
    ScanTx {
        /// Txid to scan
        tx: Txid,

        /// Blockhash of transaction (not required if node is using txindex=1)
        block: Option<BlockHash>,

        /// Optional input to scan (default to all inputs)
        #[arg(long)]
        input: Option<usize>,
    },
}

pub enum ScanMode {
    Block(BlockHash),
    Transaction(Txid, Option<BlockHash>),
    Input(usize, Txid, Option<BlockHash>),
}
