use std::{collections::VecDeque, str::FromStr};

use bitcoin::{opcodes::all::OP_IF, script::Instruction, TxIn};
use mime::Mime;

pub struct Inscription {
    pub mime: Mime,
    pub data: Vec<u8>,
}

impl Inscription {
    pub fn extract_witness(input: &TxIn) -> anyhow::Result<Option<Inscription>> {
        if let Some((mime_type, data)) = extract_inscription(input) {
            let mime = Mime::from_str(&mime_type)?;
            return Ok(Some(Inscription { mime, data }));
        }
        Ok(None)
    }

    pub fn print(&self) -> anyhow::Result<()> {
        if let Ok(img) = image::load_from_memory(&self.data) {
            let config = viuer::Config {
                absolute_offset: false,
                y: 1,
                ..Default::default()
            };
            viuer::print(&img, &config)?;
        }

        Ok(())
    }
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
