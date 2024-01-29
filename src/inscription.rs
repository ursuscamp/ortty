use anyhow::anyhow;
use bitcoincore_rpc::RpcApi;
use image::{DynamicImage, EncodableLayout, ImageFormat};
use std::{collections::VecDeque, path::PathBuf, sync::Arc};

use bitcoin::{
    opcodes::all::{OP_ENDIF, OP_IF},
    script::Instruction,
    Script, Transaction, TxIn, Txid,
};
use colored_json::{to_colored_json, ColorMode};

#[derive(Clone)]
pub enum ParsedData {
    Binary,
    Html(String),
    Image(DynamicImage),
    Json(serde_json::Value),
    Text(String),
}

impl ParsedData {
    pub fn is_brc20(&self) -> bool {
        match self {
            ParsedData::Json(json) => json.get("p").unwrap_or(&serde_json::Value::Null) == "brc-20",
            _ => false,
        }
    }

    pub fn is_text(&self) -> bool {
        matches!(
            self,
            ParsedData::Html(_) | ParsedData::Json(_) | ParsedData::Text(_)
        )
    }

    pub fn is_json(&self) -> bool {
        matches!(self, ParsedData::Json(_))
    }

    pub fn is_html(&self) -> bool {
        matches!(self, ParsedData::Html(_))
    }

    pub fn is_image(&self) -> bool {
        matches!(self, ParsedData::Image(_))
    }
}

#[derive(Debug, Clone)]
pub struct InscriptionId(Txid, usize);

impl std::str::FromStr for InscriptionId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut id = s.split('i');
        let txid = id
            .next()
            .and_then(|v| v.parse().ok())
            .ok_or_else(|| anyhow!("Inscription ID parse error"))?;
        let input = id
            .next()
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| anyhow!("Inscription ID parse error"))?;
        Ok(InscriptionId(txid, input))
    }
}

impl std::fmt::Display for InscriptionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}i{}", self.0, self.1)
    }
}

#[derive(Clone)]
pub struct Inscription {
    pub txid: Txid,
    pub index: usize,
    pub mime: String,
    pub data: Vec<u8>,
    pub parsed: ParsedData,
}

impl std::fmt::Display for Inscription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[Inscription {}]", self.inscription_id())
    }
}

impl Inscription {
    pub fn extract_all(tx: &Transaction) -> anyhow::Result<Vec<Arc<Inscription>>> {
        let mut inscriptions = Vec::with_capacity(1);
        for (idx, _) in tx.input.iter().enumerate() {
            inscriptions.extend(Inscription::extract_witness(tx, idx)?);
        }
        Ok(inscriptions)
    }

    pub fn extract_witness(
        tx: &Transaction,
        input: usize,
    ) -> anyhow::Result<Vec<Arc<Inscription>>> {
        let txin = tx
            .input
            .get(input)
            .ok_or_else(|| anyhow!("Missing input"))?;
        if let Some(inscriptions) = extract_inscription(txin) {
            let arc_ins = inscriptions
                .into_iter()
                .enumerate()
                .map(|(index, (mime, data))| {
                    let parsed = parse_data(&data, &mime);
                    Arc::new(Inscription {
                        txid: tx.txid(),
                        index,
                        mime,
                        data,
                        parsed,
                    })
                })
                .collect();
            return Ok(arc_ins);
        }
        Ok(Vec::new())
    }

    pub fn print(&self, raw_json: bool) -> anyhow::Result<()> {
        match &self.parsed {
            ParsedData::Binary => println!("{}", hex::encode(self.data.as_bytes())),
            ParsedData::Html(text) | ParsedData::Text(text) => println!("{text}"),
            ParsedData::Image(image) => print_image(image)?,
            ParsedData::Json(value) => print_json(value, raw_json)?,
        }

        Ok(())
    }

    pub fn write_to_file(&self, path: &PathBuf) -> anyhow::Result<()> {
        match path.parent() {
            Some(dir) if !dir.exists() => std::fs::create_dir_all(dir)?,
            _ => {}
        }
        std::fs::write(path, &self.data)?;
        Ok(())
    }

    /// Guess file extension for file based on data heuristic
    pub fn file_extension(&self) -> String {
        match self.parsed {
            ParsedData::Binary => "dat".into(),
            ParsedData::Html(_) => "html".into(),
            ParsedData::Image(_) => image::guess_format(&self.data)
                .map(ImageFormat::extensions_str)
                .unwrap_or_default()
                .first()
                .unwrap_or(&"dat")
                .to_string(),
            ParsedData::Json(_) => "json".into(),
            ParsedData::Text(_) => "txt".into(),
        }
    }

    /// Open an inscription the default indexer
    pub fn open_web(&self) -> anyhow::Result<()> {
        open::that(format!(
            "https://ordinals.com/inscription/{}",
            self.inscription_id(),
        ))?;
        Ok(())
    }

    pub fn inscription_id(&self) -> String {
        format!("{}i{}", self.txid, self.index)
    }
}

fn extract_inscription(txin: &TxIn) -> Option<Vec<(String, Vec<u8>)>> {
    let tapscript = txin.witness.tapscript()?;
    let inscriptions = extract_script(tapscript);
    Some(inscriptions)
}

fn extract_script(script: &Script) -> Vec<(String, Vec<u8>)> {
    let instructions: Result<VecDeque<_>, _> = script.instructions().collect();
    let mut inscriptions = Vec::new();
    if instructions.is_err() {
        return inscriptions;
    }
    let mut instructions = instructions.unwrap();

    while !instructions.is_empty() {
        if extract_op0(&mut instructions).is_none() {
            continue;
        }

        if extract_opif(&mut instructions).is_none() {
            continue;
        }

        if extract_ord(&mut instructions).is_none() {
            continue;
        }

        if extract_push1(&mut instructions).is_none() {
            continue;
        }

        if let Some(media_type) = extract_media_type(&mut instructions) {
            if extract_until_op0(&mut instructions).is_none() {
                continue;
            }
            let data = extract_data(&mut instructions);

            if extract_opendif(&mut instructions).is_none() {
                continue;
            }

            inscriptions.push((media_type, data));
        }
    }

    inscriptions
}

fn extract_op0(script: &mut VecDeque<Instruction<'_>>) -> Option<()> {
    if script.pop_front()?.push_bytes()?.is_empty() {
        return Some(());
    }
    None
}

fn extract_opif(script: &mut VecDeque<Instruction<'_>>) -> Option<()> {
    if script.pop_front()?.opcode()? == OP_IF {
        return Some(());
    }
    None
}

fn extract_ord(script: &mut VecDeque<Instruction<'_>>) -> Option<()> {
    if script.pop_front()?.push_bytes()?.as_bytes() == b"ord" {
        return Some(());
    }
    None
}

fn extract_push1(script: &mut VecDeque<Instruction<'_>>) -> Option<()> {
    if script.pop_front()?.push_bytes()?.as_bytes() == [1] {
        return Some(());
    }
    None
}

fn extract_until_op0(script: &mut VecDeque<Instruction<'_>>) -> Option<()> {
    while !script.is_empty() {
        if script.pop_front()?.push_bytes()?.is_empty() {
            return Some(());
        }
    }
    None
}

fn extract_media_type(script: &mut VecDeque<Instruction<'_>>) -> Option<String> {
    script
        .pop_front()?
        .push_bytes()
        .and_then(|b| std::str::from_utf8(b.as_bytes()).ok())
        .map(Into::into)
}

fn extract_opendif(script: &mut VecDeque<Instruction<'_>>) -> Option<()> {
    if script.get(0)?.opcode()? == OP_ENDIF {
        script.pop_front();
        return Some(());
    }
    None
}

fn extract_data(instructions: &mut VecDeque<Instruction<'_>>) -> Vec<u8> {
    let mut data = Vec::new();
    while let Some(ins) = instructions.get(0) {
        match ins {
            Instruction::PushBytes(pb) => data.extend(pb.as_bytes()),
            Instruction::Op(_) => break,
        }
        instructions.pop_front();
    }
    data
}

fn parse_data(data: &[u8], mime: &str) -> ParsedData {
    if let Ok(text) = std::str::from_utf8(data) {
        if mime.to_lowercase().contains("html") {
            return ParsedData::Html(text.into());
        } else if let Ok(value) = serde_json::from_str(text) {
            return ParsedData::Json(value);
        } else {
            return ParsedData::Text(text.into());
        }
    }

    if let Ok(image) = image::load_from_memory(data) {
        return ParsedData::Image(image);
    }

    ParsedData::Binary
}

fn print_image(image: &DynamicImage) -> anyhow::Result<()> {
    let config = viuer::Config {
        absolute_offset: false,
        y: 1,
        width: Some(40),
        ..Default::default()
    };
    viuer::print(image, &config)?;
    Ok(())
}

fn print_json(value: &serde_json::Value, raw_json: bool) -> anyhow::Result<()> {
    let formatted = if raw_json {
        serde_json::to_string(value)?
    } else {
        to_colored_json(value, ColorMode::On)?
    };
    println!("{formatted}");
    Ok(())
}

pub(crate) fn fetch_and_print(
    args: &crate::args::Args,
    inscription_id: &InscriptionId,
) -> anyhow::Result<()> {
    let client = bitcoincore_rpc::Client::new(&args.rpc_host(), args.rpc_auth()?)?;
    let tx = client.get_raw_transaction(&inscription_id.0, None)?;
    let inscriptions = Inscription::extract_witness(&tx, inscription_id.1)
        .map_err(|_| anyhow!("Inscription not found"))?;
    for inscription in inscriptions {
        inscription.print(args.raw())?;
    }
    println!();

    Ok(())
}

#[cfg(test)]
mod tests {
    use bitcoin::opcodes::{all::OP_CHECKSIG, OP_FALSE};

    use super::*;

    #[test]
    fn test_normal_inscription() {
        let script = bitcoin::script::Builder::new()
            .push_opcode(OP_FALSE)
            .push_opcode(OP_IF)
            .push_slice(b"ord")
            .push_slice([1])
            .push_slice(b"text/plain")
            .push_slice([])
            .push_slice(b"hello world")
            .push_opcode(OP_ENDIF)
            .into_script();
        let results = extract_script(&script);
        assert_eq!(results.len(), 1);
        assert_eq!(results, [("text/plain".into(), b"hello world".to_vec())]);
    }

    #[test]
    fn test_ignore_preceding() {
        let script = bitcoin::script::Builder::new()
            .push_opcode(OP_CHECKSIG)
            .push_opcode(OP_FALSE)
            .push_opcode(OP_IF)
            .push_slice(b"ord")
            .push_slice([1])
            .push_slice(b"text/plain")
            .push_slice([])
            .push_slice(b"hello world")
            .push_opcode(OP_ENDIF)
            .into_script();
        let results = extract_script(&script);
        assert_eq!(results.len(), 1);
        assert_eq!(results, [("text/plain".into(), b"hello world".to_vec())]);
    }

    #[test]
    fn test_multiple_inscriptions_per_witness() {
        let script = bitcoin::script::Builder::new()
            .push_opcode(OP_CHECKSIG)
            .push_opcode(OP_FALSE)
            .push_opcode(OP_IF)
            .push_slice(b"ord")
            .push_slice([1])
            .push_slice(b"text/plain")
            .push_slice([])
            .push_slice(b"hello world")
            .push_opcode(OP_ENDIF)
            .push_opcode(OP_FALSE)
            .push_opcode(OP_IF)
            .push_slice(b"ord")
            .push_slice([1])
            .push_slice(b"text/plain")
            .push_slice([])
            .push_slice(b"goodbye world")
            .push_opcode(OP_ENDIF)
            .into_script();
        let results = extract_script(&script);
        assert_eq!(results.len(), 2);
        assert_eq!(
            results,
            [
                ("text/plain".into(), b"hello world".to_vec()),
                ("text/plain".into(), b"goodbye world".to_vec())
            ]
        );
    }
}
