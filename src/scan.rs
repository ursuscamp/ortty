use anyhow::anyhow;
use bitcoincore_rpc::RpcApi;

use crate::{
    args::{Args, ScanMode},
    inscription::Inscription,
};

pub fn scan(args: &Args) -> anyhow::Result<Vec<Inscription>> {
    match args.scan_mode()? {
        ScanMode::Block => scan_block(args),
        ScanMode::Transaction => scan_transaction(args),
        ScanMode::Input => scan_input(args),
    }
}

fn scan_block(args: &Args) -> anyhow::Result<Vec<Inscription>> {
    let rpc = bitcoincore_rpc::Client::new(&args.rpc_host(), args.rpc_auth())?;
    let blockhash = args.block.expect("Blockhash validation error");
    let block = rpc.get_block(&blockhash)?;
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

fn scan_transaction(args: &Args) -> anyhow::Result<Vec<Inscription>> {
    let rpc = bitcoincore_rpc::Client::new(&args.rpc_host(), args.rpc_auth())?;
    let txid = args.tx.expect("Txid validation error");
    let tx = rpc.get_raw_transaction(&txid, args.block.as_ref())?;
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

fn scan_input(args: &Args) -> anyhow::Result<Vec<Inscription>> {
    let rpc = bitcoincore_rpc::Client::new(&args.rpc_host(), args.rpc_auth())?;
    let txid = args.tx.expect("Txid validation error");
    let tx = rpc.get_raw_transaction(&txid, args.block.as_ref())?;
    let input = args.input.expect("Input validation error");
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
