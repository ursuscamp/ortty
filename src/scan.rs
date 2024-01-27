use std::sync::Arc;

use bitcoin::{BlockHash, Txid};
use bitcoincore_rpc::RpcApi;

use crate::{
    args::{Args, ScanMode},
    filter::Filter,
    inscription::Inscription,
};

pub fn scan(args: &Args) -> anyhow::Result<Vec<Arc<Inscription>>> {
    match args.scan_mode()? {
        ScanMode::Block(block, filter) => scan_block(args, &block, &filter),
        ScanMode::Transaction(txid, block, filter) => {
            scan_transaction(args, &txid, &block, &filter)
        }
    }
}

fn scan_block(
    args: &Args,
    block: &BlockHash,
    filters: &[Filter],
) -> anyhow::Result<Vec<Arc<Inscription>>> {
    let rpc = bitcoincore_rpc::Client::new(&args.rpc_host(), args.rpc_auth()?)?;
    let block = rpc.get_block(block)?;
    let mut inscriptions = Vec::new();
    for tx in &block.txdata {
        for (input, _) in tx.input.iter().enumerate() {
            for inscription in Inscription::extract_witness(tx, input)? {
                // If any filters are specified, check if the inscription matches a filter and add it
                // If no filters are specified, it automatically matches
                if !filters.is_empty() {
                    if filters.iter().any(|f| f.inscription(&inscription)) {
                        inscriptions.push(inscription);
                    }
                } else {
                    inscriptions.push(inscription);
                }
            }
        }
    }
    Ok(inscriptions)
}

fn scan_transaction(
    args: &Args,
    txid: &Txid,
    block: &Option<BlockHash>,
    filters: &[Filter],
) -> anyhow::Result<Vec<Arc<Inscription>>> {
    let rpc = bitcoincore_rpc::Client::new(&args.rpc_host(), args.rpc_auth()?)?;
    let tx = rpc.get_raw_transaction(txid, block.as_ref())?;
    let inscriptions = Inscription::extract_all(&tx)?;
    let inscriptions: Vec<Arc<Inscription>> = inscriptions
        .into_iter()
        .filter(|inscription| {
            // If any filters are specified, check if the inscription matches a filter and add it
            // If no filters are specified, it automatically matches
            if !filters.is_empty() {
                filters.iter().any(|f| f.inscription(inscription))
            } else {
                true
            }
        })
        .collect();
    Ok(inscriptions)
}
