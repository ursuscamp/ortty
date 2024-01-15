use bitcoin::{BlockHash, Txid};
use bitcoincore_rpc::RpcApi;

use crate::{
    args::{Args, ScanMode},
    filter::Filter,
    inscription::Inscription,
};

pub fn scan(args: &Args) -> anyhow::Result<Vec<Inscription>> {
    match args.scan_mode()? {
        ScanMode::Block(block, filter) => scan_block(args, &block, &filter),
        ScanMode::Transaction(txid, block, filter) => {
            scan_transaction(args, &txid, &block, &filter)
        }
        ScanMode::Input(input, txid, block) => scan_input(args, input, &txid, &block),
    }
}

fn scan_block(
    args: &Args,
    block: &BlockHash,
    filters: &[Filter],
) -> anyhow::Result<Vec<Inscription>> {
    let rpc = bitcoincore_rpc::Client::new(&args.rpc_host(), args.rpc_auth()?)?;
    let block = rpc.get_block(block)?;
    let mut inscriptions = Vec::new();
    for tx in &block.txdata {
        for (input, _) in tx.input.iter().enumerate() {
            if let Some(inscription) = Inscription::extract_witness(&tx, input)? {
                // If any filters are specified, check if the inscription matches a filter and add it
                // If no filters are specified, it automatically matches
                if filters.len() > 0 {
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
) -> anyhow::Result<Vec<Inscription>> {
    let rpc = bitcoincore_rpc::Client::new(&args.rpc_host(), args.rpc_auth()?)?;
    let tx = rpc.get_raw_transaction(&txid, block.as_ref())?;
    let inscriptions: anyhow::Result<Vec<Option<Inscription>>> = tx
        .input
        .iter()
        .enumerate()
        .map(|(input, _)| Inscription::extract_witness(&tx, input))
        .collect();
    let inscriptions: Vec<Inscription> = inscriptions?
        .into_iter()
        .filter(Option::is_some)
        .map(Option::unwrap)
        .filter(|inscription| {
            // If any filters are specified, check if the inscription matches a filter and add it
            // If no filters are specified, it automatically matches
            if filters.len() > 0 {
                filters.iter().any(|f| f.inscription(&inscription))
            } else {
                true
            }
        })
        .collect();
    Ok(inscriptions)
}

fn scan_input(
    args: &Args,
    input: usize,
    txid: &Txid,
    blockhash: &Option<BlockHash>,
) -> anyhow::Result<Vec<Inscription>> {
    let rpc = bitcoincore_rpc::Client::new(&args.rpc_host(), args.rpc_auth()?)?;
    let tx = rpc.get_raw_transaction(&txid, blockhash.as_ref())?;
    let inscription = Inscription::extract_witness(&tx, input)?;
    if let Some(inscription) = inscription {
        return Ok(vec![inscription]);
    }
    Ok(vec![])
}
