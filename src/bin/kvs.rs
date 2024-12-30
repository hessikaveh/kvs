use anyhow::Result;
use clap::{Parser, Subcommand};
use kvs::kv_store::KvStore;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Get {
        #[arg(index = 1)]
        key: String,
    },
    Set {
        #[arg(index = 1)]
        key: String,

        #[arg(index = 2)]
        value: String,
    },

    Rm {
        #[arg(index = 1)]
        key: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut store: KvStore = KvStore::open("wal.mp")?;
    match &cli.command {
        Commands::Get { key } => {
            if let Some(value) = store.get(key.to_owned())? {
                println!("{}", value);
            } else {
                println!("Key not found");
            }
            Ok(())
        }
        Commands::Rm { key } => {
            if let Err(e) = store.remove(key.to_owned()) {
                println!("Key not found");
                return Err(e);
            }
            Ok(())
        }
        Commands::Set { key, value } => {
            let _ = store.set(key.to_owned(), value.to_owned());
            Ok(())
        }
    }
}
