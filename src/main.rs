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

    vault: Vault,
}

async fn export_journal(config: &Config) -> anyhow::Result<()> {
    let mut conn = db::connect_db(&config.database_file).await.expect("Failed to connect to database");
    let mut entries = db::get_entries(&mut conn, &config.journal_name).await.expect("Failed read entries from database");

    tokio::fs::create_dir_all(&config.vault.default_export)
        .await.context("Creating new entries export location")?;

    if config.vault.group_by_journal {
        tokio::fs::create_dir_all(&config.vault.default_export.join(&config.journal_name))
            .await.context("Creating journal specific new entries export location")?;
    }

    config.vault.export_entries(&mut entries).await?;

    Ok(())
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(long, short, help = "The name of the journal to be exported")]
    journal: String,

    #[arg(long, short, help = "If true will group entries by their journal")]
    group_by_journal: bool,

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

            vault: Vault {
                root: cli.vault.clone(),
                default_export: cli.default_output.clone(),
                should_update_existing: cli.should_update_existing,
                group_by_journal: cli.group_by_journal,
            },
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli: Cli = Cli::parse();
    export_journal(&cli.into()).await
}
