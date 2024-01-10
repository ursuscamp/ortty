use std::path::PathBuf;

use bitcoin::{BlockHash, Txid};
use bitcoincore_rpc::Auth;

#[derive(clap::Parser, Debug)]
pub struct Args {
    #[arg(long, env = "BITCOIN_HOST")]
    pub host: Option<String>,

    #[arg(long, env = "BITCOIN_USER")]
    pub user: Option<String>,

    #[arg(long, env = "BITCOIN_PASS")]
    pub password: Option<String>,

    #[arg(long, env = "BITCOIN_COOKIE")]
    pub cookie: Option<PathBuf>,

    #[arg(long)]
    pub block: Option<BlockHash>,

    #[arg(long)]
    pub tx: Option<Txid>,

    #[arg(long)]
    pub input: Option<usize>,
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
}
