use anyhow::anyhow;
use bitcoin::Transaction;
use bitcoincore_rpc::RpcApi;
use clap::Parser;
use inscription::Inscription;

use crate::args::Args;

mod args;
mod inscription;

fn main() -> anyhow::Result<()> {
    dotenv::dotenv()?;

    let args = Args::parse();

    let tx = load_tx(&args)?;
    let input = args.input.unwrap_or_default();
    let txin = tx
        .input
        .get(input)
        .ok_or_else(|| anyhow!("Invalid input"))?;
    if let Some(inscription) = Inscription::extract_witness(txin)? {
        inscription.print()?;
    }
    Ok(())
}

fn load_tx(args: &Args) -> anyhow::Result<Transaction> {
    let rpc = bitcoincore_rpc::Client::new(&args.rpc_host(), args.rpc_auth())?;
    let txid = args.tx.clone().ok_or_else(|| anyhow!("Missing txid"))?;
    let tx = rpc.get_raw_transaction(&txid, args.block.as_ref())?;
    Ok(tx)
}
