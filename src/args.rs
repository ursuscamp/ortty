use std::{io::stdout, path::PathBuf};

use anyhow::{anyhow, bail};
use bitcoin::{BlockHash, Txid};
use bitcoincore_rpc::Auth;
use crossterm::tty::IsTty;
use directories::BaseDirs;

use crate::{filter::Filter, inscription::InscriptionId};

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
    pub command: Commands,
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
            Commands::Scan {
                block: Some(block),
                tx: None,
                filter,
                ..
            } => ScanMode::Block(*block, filter.clone()),
            Commands::Scan {
                block,
                tx: Some(txid),
                filter,
                ..
            } => ScanMode::Transaction(*txid, *block, filter.clone()),
            _ => bail!("Cannot determine scan mode"),
        };
        Ok(mode)
    }

    pub fn extract(&self) -> Option<&PathBuf> {
        match &self.command {
            Commands::Scan { extract, .. } => extract.as_ref(),
            _ => None,
        }
    }

    pub fn web(&self) -> Option<bool> {
        match &self.command {
            Commands::Scan { web, .. } => Some(*web),
            _ => None,
        }
    }

    pub fn inscription_id(&self) -> Option<bool> {
        match &self.command {
            Commands::Scan { inscription_id, .. } => Some(*inscription_id),
            _ => None,
        }
    }

    pub fn raw(&self) -> bool {
        // If it's not a TTY, then never print colored text
        if !stdout().is_tty() {
            return true;
        }

        match &self.command {
            Commands::Scan { raw, .. } => *raw,
            Commands::Inscription { raw, .. } => *raw,
            _ => false,
        }
    }
}

#[derive(clap::Subcommand, Debug)]
pub enum Commands {
    /// Scan a block and/or tx in order to view the embedded inscriptions. Specifying only a
    /// blockhash will scan the entire block. Specifying a blockhash and a txid will scan that tx.
    /// Optionally, an input can be specified to extract only that input.
    ///
    /// When connected to a node with `txindex=1` specified, blockhash is not required.
    Scan {
        /// Blockhash of transaction
        #[arg(long)]
        block: Option<BlockHash>,

        /// Txid to scan
        #[arg(long)]
        tx: Option<Txid>,

        /// Filter inscriptions by type [text, json, brc20, image]
        #[arg(long)]
        filter: Vec<Filter>,

        /// Extract inscriptions to this folder
        #[arg(long)]
        extract: Option<PathBuf>,

        /// View the inscription on the web
        #[arg(long)]
        web: bool,

        /// Print inscription ID along with the output
        #[arg(long)]
        inscription_id: bool,

        /// Prints JSON as unformatted plain text
        #[arg(long)]
        raw: bool,
    },

    /// Explore the blockchain interactively
    Explore,

    /// View a single inscription by inscription id. Requires node with txindex=1
    Inscription {
        inscription_id: InscriptionId,

        /// Prints JSON as unformatted plain text
        #[arg(long)]
        raw: bool,
    },
}

pub enum ScanMode {
    Block(BlockHash, Vec<Filter>),
    Transaction(Txid, Option<BlockHash>, Vec<Filter>),
}
