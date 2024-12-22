#![feature(let_chains)]

mod db;
mod entry;
mod exporter;

use crate::exporter::ExportConfig;
use anyhow::Context;
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use clap::Parser;
use itertools::Itertools;
use obsidian_rust_interface::Vault;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{ConnectOptions, Row, SqliteConnection};
use std::collections::HashMap;
use std::fmt::Debug;
use std::path::PathBuf;

struct Filters {
    only_journals: Option<Vec<String>>,
    after: Option<DateTime<Utc>>,
    before: Option<DateTime<Utc>>,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(
        long = "journal",
        short = 'j',
        help = "The name of a journal to export. If none are provided, all journals will be exported"
    )]
    journals: Option<Vec<String>>,

    #[arg(
        long = "after",
        help = "Filter entries created after this date (format: YYYY-MM-DD)"
    )]
    after: Option<String>,

    #[arg(
        long = "before",
        help = "Filter entries created before this date (format: YYYY-MM-DD)"
    )]
    before: Option<String>,

    #[arg(long, short, help = "If true will group entries by their journal")]
    group_by_journal: bool,

    #[arg(
        long = "database",
        short = 'd',
        help = "Path to the dayone sqlite database"
    )]
    database_file: PathBuf,

    #[arg(
        long,
        short,
        help = "The root of the vault which will be searched for existing entries"
    )]
    vault: PathBuf,

    #[arg(
        long,
        short = 'o',
        help = "Where to place new entries that have not yet been exported"
    )]
    default_output: PathBuf,

    #[arg(
        short = 'w',
        long = "overwrite",
        help = "If existing files should be updated with newer DayOne content if available"
    )]
    should_update_existing: bool,

    #[arg(long)]
    dry_run: bool,

    #[arg(
        long,
        help = "Print stats for the overall journal export, broken down by journal."
    )]
    list_stats: bool,

    #[arg(long, help = "List the existing entries found and then exit")]
    list_existing: bool,
}

impl Cli {
    fn vault(&self) -> ExportConfig {
        ExportConfig {
            vault: Vault::open(&self.vault),
            default_export: self.default_output.clone(),
            should_update_existing_content: self.should_update_existing,
            group_by_journal: self.group_by_journal,
        }
    }

    fn parse_date_time(string: &str) -> anyhow::Result<DateTime<Utc>> {
        if let Some(date_time) = DateTime::parse_from_rfc3339(string).ok() {
            return Ok(date_time.into());
        }

        if let Some(date) = NaiveDate::parse_from_str(string, "YYYY-mm-dd").ok() {
            return Ok(date.and_time(NaiveTime::default()).and_utc());
        }

        Err(anyhow::anyhow!("Could not parse date"))
    }

    fn entry_filters(&self) -> Filters {
        Filters {
            only_journals: self.journals.clone(),
            after: self
                .after
                .clone()
                .map(|after| Self::parse_date_time(&after).expect("Failed to parse after date")),
            before: self
                .before
                .clone()
                .map(|before| Self::parse_date_time(&before).expect("Failed to parse before date")),
        }
    }
}

async fn export_journal(cli: &Cli) -> anyhow::Result<()> {
    let export_config = cli.vault();

    let mut conn = db::connect_db(&cli.database_file)
        .await
        .expect("Failed to connect to database");

    let entries = db::entries_for_filter(&mut conn, &cli.entry_filters())
        .await
        .expect("Failed read entries from database");

    tokio::fs::create_dir_all(&export_config.default_export)
        .await
        .context("Creating new entries export location")?;

    let journals_to_export = cli.journals.clone().unwrap_or_else(|| {
        entries
            .iter()
            .map(|entry| entry.metadata.journal.clone())
            .unique()
            .collect_vec()
    });

    if export_config.group_by_journal {
        for journal in journals_to_export {
            tokio::fs::create_dir_all(&export_config.default_export.join(journal))
                .await
                .context("Creating journal specific new entries export location")?;
        }
    }

    let existing_entries = export_config.read_existing();

    if cli.list_existing {
        existing_entries.values().for_each(|(path, entry)| {
            println!(
                "{} {} {}",
                entry.metadata.journal,
                entry.metadata.uuid,
                path.to_string_lossy()
            )
        });
    }

    if cli.list_stats {
        println!("Journal\tExisting\tIncoming");

        let incoming_grouped: HashMap<&String, usize> = entries
            .iter()
            .chunk_by(|entry| &entry.metadata.journal)
            .into_iter()
            .map(|(journal, group)| (journal, group.count()))
            .collect();

        let existing_grouped: HashMap<&String, usize> = existing_entries
            .values()
            .chunk_by(|(_, entry)| &entry.metadata.journal)
            .into_iter()
            .map(|(journal, group)| (journal, group.count()))
            .collect();

        for journal in incoming_grouped
            .keys()
            .chain(existing_grouped.keys())
            .unique()
        {
            println!(
                "{}\t{}\t\t{}",
                journal,
                existing_grouped.get(journal).unwrap_or(&0),
                incoming_grouped.get(journal).unwrap_or(&0)
            );
        }
    }

    if cli.dry_run {
        println!(
            "Would have exported {} entries, skipping for dry-run",
            entries.len()
        );
    } else {
        export_config
            .export_entries(&entries, &existing_entries)
            .await?;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli: Cli = Cli::parse();
    export_journal(&cli).await
}
