use std::{collections::VecDeque, str::FromStr};

use anyhow::anyhow;
use bitcoin::{opcodes::all::OP_IF, script::Instruction, BlockHash, Transaction, TxIn, Txid};
use bitcoincore_rpc::RpcApi;
use clap::Parser;

use crate::args::Args;

mod args;

fn main() -> anyhow::Result<()> {
    dotenv::dotenv()?;

    let args = Args::parse();

    let tx = load_tx(&args)?;
    let input = args.input.unwrap_or_default();
    let txin = tx
        .input
        .get(input)
        .ok_or_else(|| anyhow!("Invalid input"))?;
    let inscription = extract_inscription(txin);

    if let Some((_mime_type, bytes)) = inscription {
        let imgfmt = image::load_from_memory(&bytes)?;
        viuer::print(&imgfmt, &viuer::Config::default())?;
    }
    Ok(())
}

fn load_tx(args: &Args) -> anyhow::Result<Transaction> {
    let rpc = bitcoincore_rpc::Client::new(&args.rpc_host(), args.rpc_auth())?;
    let txid = args.tx.clone().ok_or_else(|| anyhow!("Missing txid"))?;
    let tx = rpc.get_raw_transaction(&txid, args.block.as_ref())?;
    Ok(tx)
}

fn extract_inscription(txin: &TxIn) -> Option<(String, Vec<u8>)> {
    let mime_type;
    let bytes;
    let tapscript = txin.witness.tapscript()?;
    let ins: Result<VecDeque<Instruction<'_>>, _> = tapscript.instructions().collect();
    let mut ins = ins.ok()?;
    ins.pop_front()?; // sig ignored
    ins.pop_front()?; // OP_CHECKSIG ignored

    // Check for OP_0
    let zero_len = ins.pop_front()?.push_bytes()?.len();
    if zero_len > 0 {
        return None;
    }

    // Check OP_IF
    let opif = ins.pop_front()?.opcode()?;
    if opif != OP_IF {
        return None;
    }

    // Check "ord"
    let ord = ins.pop_front()?;
    let ord = ord.push_bytes()?;
    if ord.as_bytes() != "ord".as_bytes() {
        return None;
    }

    // Check for file type or inscription
    let tag = ins.pop_front()?;
    mime_type = extract_mime_type(&mut ins, tag).unwrap_or_default();

    // Extract data
    let tag = ins.pop_front()?;
    bytes = extract_data(&mut ins, tag);

    Some((mime_type, bytes))
}

fn extract_mime_type(
    instructions: &mut VecDeque<Instruction<'_>>,
    tag: Instruction<'_>,
) -> Option<String> {
    if tag.script_num() == Some(1) {
        return Some(
            std::str::from_utf8(instructions.pop_front()?.push_bytes()?.as_bytes())
                .ok()?
                .into(),
        );
    }
    None
}

fn extract_data(instructions: &mut VecDeque<Instruction<'_>>, tag: Instruction<'_>) -> Vec<u8> {
    let mut data = Vec::new();
    if tag.script_num() == Some(0) {
        while let Some(ins) = instructions.pop_front() {
            match ins {
                Instruction::PushBytes(pb) => data.extend(pb.as_bytes()),
                Instruction::Op(_) => break,
            }
        }
    }
    data
}
