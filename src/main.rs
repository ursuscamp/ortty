use anyhow::bail;
use clap::Parser;

use crate::args::Args;

mod args;
mod filter;
mod inscription;
mod scan;

fn main() -> anyhow::Result<()> {
    dotenv::dotenv()?;

    let args = Args::parse();

    let inscriptions = scan::scan(&args)?;
    for inscription in inscriptions {
        if let Some(true) = args.web() {
            open::that(format!(
                "https://ordinals.com/inscription/{}",
                inscription.inscription_id(),
            ))?;
        }

        if let Some(extract) = args.extract() {
            if !extract.is_dir() {
                bail!("Extract must be a folder");
            }
            let fname = format!(
                "{}.{}",
                inscription.inscription_id(),
                inscription.file_extension()
            );
            let path = extract.join(fname);
            println!("Writing {}...", path.to_str().unwrap_or_default());
            inscription.write_to_file(&path)?;
        } else {
            inscription.print()?;
        }
    }
    Ok(())
}
