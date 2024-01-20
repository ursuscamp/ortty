use std::sync::Arc;

use bitcoincore_rpc::{Client, RpcApi};
use inquire::{MultiSelect, Select};

use crate::{args::Args, filter::Filter, inscription::Inscription};

/// Views are maintained in a stack. The top item in the View stack is rendered as the current
/// view. If the View is finished, it is popped of the stack. If no Views remain, then the
/// application is finished and exits normally.
#[derive(Clone)]
enum View {
    MainMenu,
    SelectBlocks(Option<u64>),
    InscriptionFilters,

    /// This doesn't actually render anything, it is a faux view that retrieve states and pushes
    /// the next view onto the stack
    RetrieveBlockInscriptions(u64),
    SelectInscriptions(Vec<Arc<Inscription>>, Option<usize>),
    PrintInscription(Arc<Inscription>),
}

struct State {
    /// The View stack.
    view: Vec<View>,

    /// JSON RPC client.
    client: Client,

    /// The user's currently selected filters.
    filters: Vec<Filter>,
}

impl State {
    pub fn new(args: &Args) -> anyhow::Result<Self> {
        Ok(State {
            view: vec![View::MainMenu],
            client: Client::new(&args.rpc_host(), args.rpc_auth()?)?,
            filters: Filter::all(),
        })
    }
}

struct InscriptionView(Arc<Inscription>);

impl std::fmt::Display for InscriptionView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{} ({}): {} bytes]",
            self.0.inscription_id(),
            self.0.mime,
            self.0.data.len()
        )
    }
}

pub fn explore(args: &Args) -> anyhow::Result<()> {
    let mut state = State::new(args)?;
    while let Some(view) = state.view.last().cloned() {
        match view {
            View::MainMenu => main_menu(&mut state)?,
            View::SelectBlocks(start) => select_blocks(&mut state, start)?,
            View::InscriptionFilters => set_filters(&mut state)?,
            View::RetrieveBlockInscriptions(blockheight) => {
                retrieve_block_inscriptions(&mut state, blockheight)?
            }
            View::SelectInscriptions(inscriptions, selected) => {
                select_inscriptions(&mut state, &inscriptions, selected)?
            }
            View::PrintInscription(inscription) => print_inscription(&mut state, inscription)?,
        };
    }
    Ok(())
}

fn main_menu(state: &mut State) -> anyhow::Result<()> {
    let options = vec!["View Blocks", "Inscription Filters", "Quit"];
    let picked = Select::new("Interactive Explorer", options).prompt()?;
    match picked {
        "View Blocks" => state.view.push(View::SelectBlocks(None)),
        "Inscription Filters" => state.view.push(View::InscriptionFilters),
        "Quit" => state.view.clear(),
        _ => unreachable!(),
    }
    Ok(())
}

fn select_blocks(state: &mut State, start: Option<u64>) -> anyhow::Result<()> {
    let block_number = match start {
        Some(sb) => sb,
        None => {
            let latest_block = state.client.get_blockchain_info()?;
            latest_block.blocks - 1
        }
    };
    let oldest_block = block_number.checked_sub(100).unwrap_or_default();
    let mut options: Vec<_> = (oldest_block..=block_number)
        .map(|i| i.to_string())
        .collect();

    options.push("Previous Page".into());
    options.push("Next Page".into());
    options.push("Home".into());
    options.reverse();
    let picked = Select::new("Select block to view", options)
        .with_page_size(30)
        .prompt()?;
    match picked.as_str() {
        "Previous Page" => {
            state.view.pop();
            return Ok(());
        }
        "Next Page" => {
            state
                .view
                .push(View::SelectBlocks(oldest_block.checked_sub(1)));
            return Ok(());
        }
        "Home" => {
            state.view.clear();
            state.view.push(View::MainMenu);
            return Ok(());
        }
        _ => {
            let picked: u64 = picked.parse()?;
            state.view.push(View::RetrieveBlockInscriptions(picked));
            return Ok(());
        }
    }
}

fn set_filters(state: &mut State) -> anyhow::Result<()> {
    let options = Filter::all();
    let selected: Vec<usize> = options
        .iter()
        .enumerate()
        .filter_map(|(idx, opt)| {
            if state.filters.contains(opt) {
                Some(idx)
            } else {
                None
            }
        })
        .collect();
    let mut new_filters = MultiSelect::new("Select inscription types to filter", options)
        .with_default(&selected)
        .prompt()?;
    new_filters.sort();
    state.filters = new_filters;
    state.view.pop();
    Ok(())
}

fn retrieve_block_inscriptions(state: &mut State, blockheight: u64) -> anyhow::Result<()> {
    let bh = state.client.get_block_hash(blockheight)?;
    let block = state.client.get_block(&bh)?;
    let mut inscriptions = Vec::with_capacity(300);
    for tx in block.txdata {
        let txins = Inscription::extract_all(&tx)?
            .into_iter()
            .filter(|i| state.filters.iter().any(|f| f.inscription(&i)));
        inscriptions.extend(txins);
    }
    state.view.pop();
    if inscriptions.is_empty() {
        println!("No results found");
        return Ok(());
    }
    state
        .view
        .push(View::SelectInscriptions(inscriptions, None));
    Ok(())
}

fn select_inscriptions(
    state: &mut State,
    inscriptions: &[Arc<Inscription>],
    index: Option<usize>,
) -> anyhow::Result<()> {
    let iviews: Vec<InscriptionView> = inscriptions
        .into_iter()
        .cloned()
        .map(InscriptionView)
        .collect();
    let selected = Select::new("Select inscription", iviews)
        .with_starting_cursor(index.unwrap_or_default())
        .raw_prompt()?;

    // Overwrite the selector index so that the next round it will start at the same index
    match state.view.last_mut() {
        Some(View::SelectInscriptions(_, o)) => *o = Some(selected.index),
        _ => {}
    }
    state.view.push(View::PrintInscription(selected.value.0));
    Ok(())
}

fn print_inscription(state: &mut State, inscription: Arc<Inscription>) -> anyhow::Result<()> {
    inscription.print()?;
    state.view.pop();
    Ok(())
}
