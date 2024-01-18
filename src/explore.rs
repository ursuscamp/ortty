use bitcoincore_rpc::{Client, RpcApi};
use inquire::Select;

use crate::{args::Args, filter::Filter, inscription::Inscription};

#[derive(Clone, Copy)]
enum View {
    MainMenu,
    RecentBlocks,
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
            filters: vec![],
        })
    }
}

pub fn explore(args: &Args) -> anyhow::Result<()> {
    let mut state = State::new(args)?;
    while let Some(view) = state.view.last().copied() {
        match view {
            View::MainMenu => main_menu(&mut state)?,
            View::RecentBlocks => select_recent_block(&mut state)?,
            View::ViewBlock(blockheight) => view_block(&mut state, blockheight)?,
        };
    }
    Ok(())
}

fn main_menu(state: &mut State) -> anyhow::Result<()> {
    let options = vec!["Recent Blocks", "Quit"];
    let picked = Select::new("Interactive Explorer", options).prompt()?;
    match picked {
        "Recent Blocks" => state.view.push(View::RecentBlocks),
        "Quit" => state.view.clear(),
        _ => unreachable!(),
    }
    Ok(())
}

fn select_recent_block(state: &mut State) -> anyhow::Result<()> {
    let latest_block = state.client.get_blockchain_info()?;
    let block_number = latest_block.blocks - 1;
    let oldest_block = block_number.checked_sub(100).unwrap_or_default();
    let mut options: Vec<_> = (oldest_block..=block_number).collect();
    options.reverse();
    let picked = Select::new("Select block to view", options).prompt()?;
    state.view.pop();
    state.view.push(View::ViewBlock(picked));
    Ok(())
}

fn view_block(state: &mut State, blockheight: u64) -> anyhow::Result<()> {
    let bh = state.client.get_block_hash(blockheight)?;
    let block = state.client.get_block(&bh)?;
    let mut inscriptions = Vec::with_capacity(300);
    for tx in block.txdata {
        let txins = Inscription::extract_all(&tx)?.into_iter().filter(|i| {
            // If filters aren't empty, then filter inscriptions by them
            state.filters.is_empty()
                || (state.filters.len() > 0 && state.filters.iter().any(|f| f.inscription(&i)))
        });
        inscriptions.extend(txins);
    }
    inscriptions.iter().for_each(|i| {
        i.print().ok();
    });
    state.view.pop();
    Ok(())
}
