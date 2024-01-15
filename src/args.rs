use std::path::PathBuf;

use anyhow::anyhow;
use bitcoin::{BlockHash, Txid};
use bitcoincore_rpc::Auth;
use directories::BaseDirs;

use crate::filter::Filter;

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

    /// Path to RPC cookie file (if applicable). Searches known folders by default
    #[arg(long, env = "BITCOIN_COOKIE")]
    pub cookie: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

impl Args {
    pub fn find_cookie(&self) -> Option<PathBuf> {
        if let Some(bd) = BaseDirs::new() {
            let paths = [
                bd.home_dir().join(".bitcoin").join("cookie"),
                bd.config_dir().join("bitcoin").join("cookie"),
                bd.config_local_dir().join("bitcoin").join("cookie"),
                bd.data_dir().join("bitcoin").join("cookie"),
            ];
            return paths.into_iter().find(|p| p.exists());
        }
        None
    }

    pub fn rpc_host(&self) -> String {
        match &self.host {
            Some(host) => host.clone(),
            None => "localhost".into(),
        }
    }

    pub fn rpc_auth(&self) -> anyhow::Result<Auth> {
        // Auth order:
        // 1. If cookie is specified, use it
        // 2. If username AND password are specified, use them
        // 3. Search for cookies in default folders
        // 4. Raise authentication error for nothing found
        let auth = if let Some(cookie) = &self.cookie {
            Auth::CookieFile(cookie.clone())
        } else if self.user.is_some() && self.password.is_some() {
            Auth::UserPass(
                self.user.clone().unwrap_or_default(),
                self.password.clone().unwrap_or_default(),
            )
        } else if let Some(cookie) = self.find_cookie() {
            Auth::CookieFile(cookie)
        } else {
            return Err(anyhow!("Missing RPC auth info"));
        };

        Ok(auth)
    }

    pub fn scan_mode(&self) -> anyhow::Result<ScanMode> {
        let mode = match &self.command {
            Commands::ScanBlock {
                block,
                filter,
                extract: _extract,
            } => ScanMode::Block(*block, filter.clone()),
            Commands::ScanTx {
                tx,
                block,
                input: Some(input),
                filter: _filter,
                extract: _extract,
            } => ScanMode::Input(*input, *tx, *block),
            Commands::ScanTx {
                tx,
                block,
                input: _input,
                filter,
                extract: _extract,
            } => ScanMode::Transaction(*tx, *block, filter.clone()),
        };
        Ok(mode)
    }

    pub fn extract(&self) -> Option<&PathBuf> {
        match &self.command {
            Commands::ScanBlock {
                block: _,
                filter: _,
                extract,
            } => extract.as_ref(),
            Commands::ScanTx {
                tx: _,
                block: _,
                input: _,
                filter: _,
                extract,
            } => extract.as_ref(),
        }
    }
}

#[derive(clap::Subcommand, Debug)]
pub enum Commands {
    /// Scan a full block and print/extract every recognizable Inscription
    ScanBlock {
        /// Blockhash to scan
        block: BlockHash,

        /// Filter inscriptions by type [text, json, brc20, image]
        #[arg(long)]
        filter: Vec<Filter>,

        /// Extract inscriptions to this folder
        #[arg(long)]
        extract: Option<PathBuf>,
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

        /// Filter inscriptions by type [text, json, brc20, image]
        #[arg(long)]
        filter: Vec<Filter>,

        /// Extract inscriptions to this folder
        #[arg(long)]
        extract: Option<PathBuf>,
    },
}

pub enum ScanMode {
    Block(BlockHash, Vec<Filter>),
    Transaction(Txid, Option<BlockHash>, Vec<Filter>),
    Input(usize, Txid, Option<BlockHash>),
}
