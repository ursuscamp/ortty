use std::{path::PathBuf, sync::Arc};

use bitcoincore_rpc::{Client, RpcApi};
use crossterm::style::Stylize;
use inquire::{MultiSelect, Select};

use crate::{args::Args, filter::Filter, inscription::Inscription};

mod opts;

use opts::*;

/// Views are maintained in a stack. The top item in the View stack is rendered as the current
/// view. If the View is finished, it is popped of the stack. If no Views remain, then the
/// application is finished and exits normally.
#[derive(Clone)]
enum View {
    MainMenu,
    SelectBlocks {
        starting_block: Option<u64>,
        index: Option<usize>,
    },
    InscriptionFilters,
    ExtraOptions,

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

    // Extra options that the user can set
    extra_opts: ExtraOptions,
}

impl State {
    pub fn new(args: &Args) -> anyhow::Result<Self> {
        Ok(State {
            view: vec![View::MainMenu],
            client: Client::new(&args.rpc_host(), args.rpc_auth()?)?,
            filters: Filter::all(),
            extra_opts: ExtraOptions::default(),
        })
    }
}

enum InscriptionView {
    Home,
    Inscription(Arc<Inscription>),
}

impl std::fmt::Display for InscriptionView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InscriptionView::Home => f.write_str("Home"),
            InscriptionView::Inscription(i) => {
                write!(
                    f,
                    "[{} ({}): {} bytes]",
                    i.inscription_id().to_string().red(),
                    i.mime.to_string().blue(),
                    i.data.len().to_string().green()
                )
            }
        }
    }
}

pub fn explore(args: &Args) -> anyhow::Result<()> {
    let mut state = State::new(args)?;
    while let Some(view) = state.view.last().cloned() {
        match view {
            View::MainMenu => main_menu(&mut state)?,
            View::SelectBlocks {
                starting_block,
                index,
            } => select_blocks(&mut state, starting_block, index)?,
            View::InscriptionFilters => set_filters(&mut state)?,
            View::ExtraOptions => set_extra_options(&mut state)?,
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
    let options = vec![
        "View Blocks",
        "Inscription Filters",
        "Extra Options",
        "Quit",
    ];
    let picked = Select::new("Interactive Explorer", options).prompt()?;
    match picked {
        "View Blocks" => state.view.push(View::SelectBlocks {
            starting_block: None,
            index: None,
        }),
        "Inscription Filters" => state.view.push(View::InscriptionFilters),
        "Extra Options" => state.view.push(View::ExtraOptions),
        "Quit" => state.view.clear(),
        _ => unreachable!(),
    }
    Ok(())
}

fn select_blocks(
    state: &mut State,
    start: Option<u64>,
    index: Option<usize>,
) -> anyhow::Result<()> {
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
        .with_page_size(page_size())
        .with_starting_cursor(index.unwrap_or_default())
        .raw_prompt()?;

    // Replace the current index with the new selected index so that when this view get rendered
    // again next time it will start on the same index as the last picked option
    if let Some(View::SelectBlocks { index, .. }) = state.view.last_mut() {
        *index = Some(picked.index)
    }

    match picked.value.as_str() {
        "Previous Page" => {
            state.view.pop();
        }
        "Next Page" => {
            state.view.push(View::SelectBlocks {
                starting_block: oldest_block.checked_sub(1),
                index: None,
            });
        }
        "Home" => {
            state.view.clear();
            state.view.push(View::MainMenu);
        }
        _ => {
            let picked: u64 = picked.value.parse()?;
            state.view.push(View::RetrieveBlockInscriptions(picked));
        }
    }
    Ok(())
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

fn set_extra_options(state: &mut State) -> anyhow::Result<()> {
    let selected = MultiSelect::new("Set additional options", ExtraOption::all())
        .with_default(&state.extra_opts.current_set_indexes())
        .prompt()?;
    state.extra_opts.set_opts(&selected);
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
            .filter(|i| state.filters.iter().any(|f| f.inscription(i)));
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
    let iviews: Vec<InscriptionView> = [InscriptionView::Home]
        .into_iter()
        .chain(
            inscriptions
                .iter()
                .cloned()
                .map(InscriptionView::Inscription),
        )
        .collect();
    let selected = Select::new("Select inscription", iviews)
        .with_starting_cursor(index.unwrap_or_default())
        .with_page_size(page_size())
        .raw_prompt()?;

    // Overwrite the selector index so that the next round it will start at the same index
    if let Some(View::SelectInscriptions(_, o)) = state.view.last_mut() {
        *o = Some(selected.index)
    }
    match selected.value {
        InscriptionView::Home => {
            state.view.clear();
            state.view.push(View::MainMenu);
        }
        InscriptionView::Inscription(i) => state.view.push(View::PrintInscription(i)),
    }
    Ok(())
}

fn print_inscription(state: &mut State, inscription: Arc<Inscription>) -> anyhow::Result<()> {
    if state.extra_opts.web {
        inscription.open_web()?;
    }

    if state.extra_opts.extract {
        let fname = format!(
            "{}.{}",
            inscription.inscription_id(),
            inscription.file_extension()
        );
        let p = PathBuf::from(&fname);
        println!("Writing inscription to {}...", fname.green());
        inscription.write_to_file(&p)?;
    }

    if state.extra_opts.render {
        inscription.print(false)?;
    }

    println!();

    state.view.pop();
    Ok(())
}

fn page_size() -> usize {
    let (_, rows) = crossterm::terminal::size().unwrap_or((80, 20));
    (rows / 4) as usize
}
