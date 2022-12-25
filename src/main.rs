#![feature(let_else)]
#![feature(option_result_contains)]
#![feature(let_chains)]

mod walk;
mod db;
mod entry;

use std::collections::HashMap;
use std::fmt::Debug;
use std::path::PathBuf;
use std::process::exit;
use anyhow::Context;
use sqlx::{ConnectOptions, Executor, Row, SqliteConnection};
use sqlx::sqlite::SqliteConnectOptions;
use clap::Parser;
use itertools::Itertools;
use crate::walk::Vault;

async fn export_journal(cli: &Cli) -> anyhow::Result<()> {
    let vault = cli.vault();

    let mut conn = db::connect_db(&cli.database_file).await.expect("Failed to connect to database");
    let entries = db::entries_for_journals(&mut conn, &cli.journals).await.expect("Failed read entries from database");

    tokio::fs::create_dir_all(&vault.default_export)
        .await.context("Creating new entries export location")?;

    if vault.group_by_journal {
        for journal in &cli.journals {
            tokio::fs::create_dir_all(&vault.default_export.join(journal))
                .await.context("Creating journal specific new entries export location")?;        }
    }

    let existing_entries = vault.read_existing();

    if cli.list_existing {
        existing_entries.values()
            .for_each(|(path, entry)|
                println!("{} {} {}",
                         entry.metadata.journal,
                         entry.metadata.uuid,
                         path.to_string_lossy()
                )
            );
    }

    if cli.list_stats {
        println!("Journal\tExisting\tIncoming");

        let incoming_grouped: HashMap<&String, usize> = entries.iter()
            .group_by(|entry| &entry.metadata.journal)
            .into_iter()
            .map(|(journal, group)| (journal, group.count()))
            .collect();

        let existing_grouped: HashMap<&String, usize> = existing_entries.values()
            .group_by(|(_, entry)| &entry.metadata.journal)
            .into_iter()
            .map(|(journal, group)| (journal, group.count()))
            .collect();

        for journal in incoming_grouped.keys().chain(existing_grouped.keys()).unique() {
            println!(
                "{}\t{}\t\t{}",
                journal,
                existing_grouped.get(journal).unwrap_or(&0),
                incoming_grouped.get(journal).unwrap_or(&0)
            );
        }
    }

    if cli.dry_run {
        exit(0)
    }

    vault.export_entries(&entries, &existing_entries).await?;

    Ok(())
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(long = "journal", short = 'j', help = "The name of a journal to export")]
    journals: Vec<String>,

    #[arg(long, short, help = "If true will group entries by their journal")]
    group_by_journal: bool,

    #[arg(long = "database", short = 'd', help = "Path to the dayone sqlite database")]
    database_file: PathBuf,

    #[arg(long, short, help = "The root of the vault which will be searched for existing entries")]
    vault: PathBuf,

    #[arg(long, short = 'o', help = "Where to place new entries that have not yet been exported")]
    default_output: PathBuf,

    #[arg(short = 'w', long = "overwrite", help = "If existing files should be updated with newer DayOne content if available")]
    should_update_existing: bool,

    #[arg(long)]
    dry_run: bool,

    #[arg(long, help = "Print stats for the overall journal export, broken down by journal.")]
    list_stats: bool,

    #[arg(long, help = "List the existing entries found and then exit")]
    list_existing: bool,
}

impl Cli {
    fn vault(&self) -> Vault {
        Vault {
            root: self.vault.clone(),
            default_export: self.default_output.clone(),
            should_update_existing: self.should_update_existing,
            group_by_journal: self.group_by_journal,
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli: Cli = Cli::parse();
    export_journal(&cli).await
}
