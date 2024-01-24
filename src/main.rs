use clap::Parser;
use crossterm::style::Stylize;
use explore::explore;

use crate::args::Args;

mod args;
mod explore;
mod filter;
mod inscription;
mod scan;

fn main() -> anyhow::Result<()> {
    dotenv::dotenv()?;

    let args = Args::parse();

    match args.command {
        args::Commands::Scan { .. } => scan(&args)?,
        args::Commands::Explore => explore(&args)?,
    }
    Ok(())
}

fn scan(args: &Args) -> Result<(), anyhow::Error> {
    let inscriptions = scan::scan(&args)?;
    Ok(for inscription in inscriptions {
        if let Some(true) = args.web() {
            inscription.open_web()?;
        }

        if let Some(extract) = args.extract() {
            let fname = format!(
                "{}.{}",
                inscription.inscription_id(),
                inscription.file_extension()
            );
            let path = extract.join(fname);
            println!("Writing {}...", path.to_str().unwrap_or_default());
            inscription.write_to_file(&path)?;
        } else {
            if args.inscription_id().unwrap_or_default() {
                println!("{}:", inscription.inscription_id().yellow());
            }
            inscription.print()?;
            println!("");
        }
    })
}
