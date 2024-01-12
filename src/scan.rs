use anyhow::anyhow;
use bitcoin::{BlockHash, Txid};
use bitcoincore_rpc::RpcApi;

use crate::{
    args::{Args, ScanMode},
    inscription::Inscription,
};

pub fn scan(args: &Args) -> anyhow::Result<Vec<Inscription>> {
    match args.scan_mode()? {
        ScanMode::Block(block) => scan_block(args, &block),
        ScanMode::Transaction(txid, block) => scan_transaction(args, &txid, &block),
        ScanMode::Input(input, txid, block) => scan_input(args, input, &txid, &block),
    }
}

fn scan_block(args: &Args, block: &BlockHash) -> anyhow::Result<Vec<Inscription>> {
    let rpc = bitcoincore_rpc::Client::new(&args.rpc_host(), args.rpc_auth())?;
    let block = rpc.get_block(block)?;
    let mut inscriptions = Vec::new();
    for tx in block.txdata {
        for input in tx.input {
            if let Some(inscription) = Inscription::extract_witness(&input)? {
                inscriptions.push(inscription);
            }
        }
    }
    Ok(inscriptions)
}

fn scan_transaction(
    args: &Args,
    txid: &Txid,
    block: &Option<BlockHash>,
) -> anyhow::Result<Vec<Inscription>> {
    let rpc = bitcoincore_rpc::Client::new(&args.rpc_host(), args.rpc_auth())?;
    let tx = rpc.get_raw_transaction(&txid, block.as_ref())?;
    let inscriptions: anyhow::Result<Vec<Option<Inscription>>> = tx
        .input
        .iter()
        .map(|txin| Inscription::extract_witness(txin))
        .collect();
    let inscriptions: Vec<Inscription> = inscriptions?
        .into_iter()
        .filter(Option::is_some)
        .map(Option::unwrap)
        .collect();
    Ok(inscriptions)
}

fn scan_input(
    args: &Args,
    input: usize,
    txid: &Txid,
    blockhash: &Option<BlockHash>,
) -> anyhow::Result<Vec<Inscription>> {
    let rpc = bitcoincore_rpc::Client::new(&args.rpc_host(), args.rpc_auth())?;
    let tx = rpc.get_raw_transaction(&txid, blockhash.as_ref())?;
    let txin = tx
        .input
        .get(input)
        .ok_or_else(|| anyhow!("Missing input"))?;
    let inscription = Inscription::extract_witness(txin)?;
    if let Some(inscription) = inscription {
        return Ok(vec![inscription]);
    }
    Ok(vec![])
}
