use bitcoincore_rpc::{Client, RpcApi};
use inquire::{MultiSelect, Select};

use crate::{args::Args, filter::Filter, inscription::Inscription};

#[derive(Clone, Copy)]
enum View {
    MainMenu,
    ViewBlocks(Option<u64>),
    InscriptionFilters,
    ViewBlock(u64),
}

struct State {
    view: Vec<View>,
    client: Client,
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

pub fn explore(args: &Args) -> anyhow::Result<()> {
    let mut state = State::new(args)?;
    while let Some(view) = state.view.last().copied() {
        match view {
            View::MainMenu => main_menu(&mut state)?,
            View::ViewBlocks(start) => view_blocks(&mut state, start)?,
            View::InscriptionFilters => set_filters(&mut state)?,
            View::ViewBlock(blockheight) => view_block(&mut state, blockheight)?,
        };
    }
    Ok(())
}

fn main_menu(state: &mut State) -> anyhow::Result<()> {
    let options = vec!["View Blocks", "Inscription Filters", "Quit"];
    let picked = Select::new("Interactive Explorer", options).prompt()?;
    match picked {
        "View Blocks" => state.view.push(View::ViewBlocks(None)),
        "Inscription Filters" => state.view.push(View::InscriptionFilters),
        "Quit" => state.view.clear(),
        _ => unreachable!(),
    }
    Ok(())
}

fn view_blocks(state: &mut State, start: Option<u64>) -> anyhow::Result<()> {
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
                .push(View::ViewBlocks(oldest_block.checked_sub(1)));
            return Ok(());
        }
        "Home" => {
            state.view.clear();
            state.view.push(View::MainMenu);
            return Ok(());
        }
        _ => {
            let picked: u64 = picked.parse()?;
            state.view.push(View::ViewBlock(picked));
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

fn view_block(state: &mut State, blockheight: u64) -> anyhow::Result<()> {
    let bh = state.client.get_block_hash(blockheight)?;
    let block = state.client.get_block(&bh)?;
    let mut inscriptions = Vec::with_capacity(300);
    for tx in block.txdata {
        let txins = Inscription::extract_all(&tx)?
            .into_iter()
            .filter(|i| state.filters.iter().any(|f| f.inscription(&i)));
        inscriptions.extend(txins);
    }
    inscriptions.iter().for_each(|i| {
        i.print().ok();
    });
    state.view.pop();
    Ok(())
}
