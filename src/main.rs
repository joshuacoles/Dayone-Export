#![feature(let_else)]
#![feature(option_result_contains)]
#![feature(let_chains)]

mod walk;
mod db;
mod entry;

use std::fmt::Debug;
use std::path::PathBuf;
use anyhow::Context;
use sqlx::{ConnectOptions, Executor, Row, SqliteConnection};
use sqlx::sqlite::SqliteConnectOptions;
use futures::{Stream, TryStreamExt};
use clap::Parser;
use crate::walk::Vault;

struct Config {
    journal_name: String,
    database_file: PathBuf,

    vault_root: PathBuf,
    new_entries_location: PathBuf,
    should_update_existing: bool,
}

async fn export_journal(config: &Config) -> anyhow::Result<()> {
    let mut conn = db::connect_db(&config.database_file).await.expect("Failed to connect to database");
    let mut entries = db::get_entries(&mut conn, &config.journal_name).await.expect("Failed read entries from database");

    tokio::fs::create_dir_all(&config.new_entries_location)
        .await.context("Creating new entries export location")?;

    let vault = Vault {
        root: config.vault_root.clone(),
        default_export: config.new_entries_location.clone(),
        should_update_existing: config.should_update_existing,
    };

    vault.export_entries(&mut entries).await?;

    Ok(())
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(long, short, help = "The name of the journal to be exported")]
    journal: String,

    #[arg(long, short, help = "Path to the dayone sqlite database")]
    database: PathBuf,

    #[arg(long, short, help = "The root of the vault which will be searched for existing entries")]
    vault: PathBuf,

    #[arg(long, short = 'o', help = "Where to place new entries that have not yet been exported")]
    default_output: PathBuf,

    #[arg(short = 'w', long = "overwrite", help = "If existing files should be updated with newer DayOne content if available")]
    should_update_existing: bool,
}

impl From<Cli> for Config {
    fn from(cli: Cli) -> Self {
        Config {
            journal_name: cli.journal,
            database_file: cli.database,

            vault_root: cli.vault,
            new_entries_location: cli.default_output,

            should_update_existing: cli.should_update_existing,
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli: Cli = Cli::parse();
    export_journal(&cli.into()).await
}
