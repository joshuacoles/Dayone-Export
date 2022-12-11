#![feature(let_else)]
#![feature(option_result_contains)]
#![feature(let_chains)]

mod walk;
mod db;

use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::path::PathBuf;
use sqlx::{ConnectOptions, Error, Executor, Row, SqliteConnection};
use sqlx::sqlite::SqliteConnectOptions;
use futures::{Stream, TryStreamExt};
use filetime::{FileTime, set_file_times};
use time::{OffsetDateTime, UtcOffset};

use time::macros::format_description;

struct Config {
    journal_name: String,
    export_root: PathBuf,
    database_file: PathBuf,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EntryMetadata {
    // A constant value of "dayone-import", present in the metadata
    #[serde(rename = "type")]
    note_type: String,

    journal: String,

    #[serde(rename = "dayoneId")]
    uuid: String,

    #[serde(rename = "createdAt", with = "time::serde::rfc3339")]
    creation_date: OffsetDateTime,

    #[serde(rename = "lastModifiedAt", with = "time::serde::rfc3339")]
    modified_date: OffsetDateTime,

    link: String,
}

impl EntryMetadata {
    pub fn validate(&self) -> bool {
        self.note_type == "dayone-import"
    }
}

impl EntryMetadata {
    fn new(journal: String, uuid: String, creation_date: OffsetDateTime, modified_date: OffsetDateTime) -> EntryMetadata {
        EntryMetadata {
            note_type: "dayone-import".to_string(),
            journal,

            link: format!("dayone://view?entryId={}", uuid),
            uuid,

            creation_date,
            modified_date,
        }
    }
}

#[derive(Debug)]
pub struct Entry {
    journal: String,
    uuid: String,
    markdown: String,
    creation_date: OffsetDateTime,
    modified_date: OffsetDateTime,
}

impl Entry {
    fn metadata(&self) -> EntryMetadata {
        EntryMetadata::new(self.journal.clone(), self.uuid.clone(), self.creation_date, self.modified_date)
    }

    fn title(&self) -> String {
        match self.markdown.trim().split('\n').next() {
            Some(first_line) if first_line.starts_with('#') => first_line.replace('#', "").trim().to_string(),
            Some(first_line) if first_line.chars().all(|x| x.is_whitespace() || x.is_alphanumeric()) => first_line.to_string(),
            _ => self.creation_date.format(format_description!("[hour]-[minute]-[second]")).unwrap()
        }
    }

    fn contents(&self) -> String {
        format!(
            "---\n{frontmatter}---\n\n{body}\n",
            frontmatter = serde_yaml::to_string(&self.metadata()).unwrap(),
            body = self.markdown.replace('\\', ""),
        )
    }

    fn default_filename(&self) -> String {
        format!(
            "{} {}.md",
            self.creation_date.format(format_description!("[year]-[month]-[day]")).expect("Failed to format date"),
            self.title(),
        )
    }
}

async fn export_journal(config: &Config) -> Result<(), sqlx::Error> {
    let mut conn = db::connect_db(&config.database_file).await?;
    let mut entries = db::get_entries(&mut conn, &config.journal_name).await?;

    let journal_root = config.export_root.join(config.journal_name.replace('/', "-"));
    tokio::fs::create_dir_all(&journal_root).await?;

    while let Some(entry) = entries.try_next().await? {
        let file_name = entry.default_filename();

        let file_path = journal_root.join(file_name);

        tokio::fs::write(&file_path, entry.contents()).await?;

        set_file_times(
            &file_path,
            FileTime::from_unix_time(entry.creation_date.unix_timestamp(), 0),
            FileTime::now(),
        )?;
    }

    Ok(())
}

use clap::Parser;

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
async fn main() -> Result<(), sqlx::Error> {
    // walk::main_2();
    let cli: Cli = Cli::parse();
    export_journal(&cli.into()).await
}
