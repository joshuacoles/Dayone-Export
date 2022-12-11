#![feature(let_else)]
#![feature(option_result_contains)]
#![feature(let_chains)]

mod walk;
mod db;
mod basic;
mod entry;

use std::fmt::Debug;
use std::path::PathBuf;
use sqlx::{ConnectOptions, Executor, Row, SqliteConnection};
use sqlx::sqlite::SqliteConnectOptions;
use futures::{Stream, TryStreamExt};

struct Config {
    journal_name: String,
    export_root: PathBuf,
    database_file: PathBuf,
}

async fn export_journal(config: &Config) -> anyhow::Result<()> {
    let mut conn = db::connect_db(&config.database_file).await?;
    let mut entries = db::get_entries(&mut conn, &config.journal_name).await?;

    let journal_root = config.export_root.join(config.journal_name.replace('/', "-"));
    tokio::fs::create_dir_all(&journal_root).await?;

    basic_export(&mut entries, journal_root).await?;
    walk_export(&mut entries).await?;

    Ok(())
}

use clap::Parser;
use crate::basic::{basic_export, walk_export};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(long, short)]
    journal: String,
    #[arg(long, short)]
    output: PathBuf,
    #[arg(long, short)]
    database: PathBuf,
}

impl From<Cli> for Config {
    fn from(cli: Cli) -> Self {
        Config {
            journal_name: cli.journal,
            export_root: cli.output,
            database_file: cli.database,
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli: Cli = Cli::parse();
    export_journal(&cli.into()).await
}
