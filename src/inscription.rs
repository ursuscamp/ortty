use anyhow::anyhow;
use bitcoincore_rpc::RpcApi;
use image::{DynamicImage, EncodableLayout, ImageFormat};
use std::{collections::VecDeque, path::PathBuf, sync::Arc};

use bitcoin::{opcodes::all::OP_IF, script::Instruction, Transaction, TxIn, Txid};
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
    pub input: usize,
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
            if let Some(inscription) = Inscription::extract_witness(tx, idx)? {
                inscriptions.push(inscription);
            }
        }
        Ok(inscriptions)
    }

    pub fn extract_witness(
        tx: &Transaction,
        input: usize,
    ) -> anyhow::Result<Option<Arc<Inscription>>> {
        let txin = tx
            .input
            .get(input)
            .ok_or_else(|| anyhow!("Missing input"))?;
        if let Some((mime, data)) = extract_inscription(txin) {
            let parsed = parse_data(&data, &mime);
            return Ok(Some(Arc::new(Inscription {
                txid: tx.txid(),
                input,
                mime,
                data,
                parsed,
            })));
        }
        Ok(None)
    }

    pub fn print(&self) -> anyhow::Result<()> {
        match &self.parsed {
            ParsedData::Binary => println!("{}", hex::encode(self.data.as_bytes())),
            ParsedData::Html(text) | ParsedData::Text(text) => println!("{text}"),
            ParsedData::Image(image) => print_image(image)?,
            ParsedData::Json(value) => print_json(value)?,
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
        format!("{}i{}", self.txid, self.input)
    }
}

fn extract_inscription(txin: &TxIn) -> Option<(String, Vec<u8>)> {
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
    let mime_type = extract_mime_type(&mut ins, tag).unwrap_or_default();

    // Ignore everything else until we find `OP_PUSH 0`
    while let Some(i) = ins.pop_front() {
        if i.push_bytes().map(|pb| pb.is_empty()).unwrap_or_default() {
            break;
        }
    }

    // Extract the data
    let bytes = extract_data(&mut ins);

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

fn extract_data(instructions: &mut VecDeque<Instruction<'_>>) -> Vec<u8> {
    let mut data = Vec::new();
    while let Some(ins) = instructions.pop_front() {
        match ins {
            Instruction::PushBytes(pb) => data.extend(pb.as_bytes()),
            Instruction::Op(_) => break,
        }
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

fn print_json(value: &serde_json::Value) -> anyhow::Result<()> {
    let formatted = to_colored_json(value, ColorMode::On)?;
    println!("{formatted}");
    Ok(())
}

pub(crate) fn fetch_and_print(
    args: &crate::args::Args,
    inscription_id: &InscriptionId,
) -> anyhow::Result<()> {
    let client = bitcoincore_rpc::Client::new(&args.rpc_host(), args.rpc_auth()?)?;
    let tx = client.get_raw_transaction(&inscription_id.0, None)?;
    let inscription = Inscription::extract_witness(&tx, inscription_id.1)?
        .ok_or_else(|| anyhow!("Inscription not found"))?;
    inscription.print()?;
    println!();

    Ok(())
}
