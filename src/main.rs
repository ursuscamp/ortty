use clap::Parser;

use crate::args::Args;

mod args;
mod inscription;
mod scan;

fn main() -> anyhow::Result<()> {
    dotenv::dotenv()?;

    let args = Args::parse();

    let inscriptions = scan::scan(&args)?;
    for inscription in inscriptions {
        inscription.print()?;
    }
    Ok(())
}
