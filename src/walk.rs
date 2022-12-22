use std::collections::HashMap;
use std::fs;
use std::future::Future;
use std::path::PathBuf;
use std::process::exit;
use futures::TryStreamExt;
use walkdir::{DirEntry, WalkDir};
use crate::Stream;
use itertools::Itertools;
use crate::entry::{Entry, EntryMetadata};

fn is_hidden(entry: &DirEntry) -> bool {
    entry.file_name()
        .to_str()
        .map(|s| s.starts_with("."))
        .unwrap_or(false)
}

fn is_markdown(entry: &DirEntry) -> bool {
    entry.path().extension().contains(&"md")
}

fn parse_entry(p0: &DirEntry) -> Option<Entry> {
    let contents = fs::read_to_string(p0.path()).expect("Failed to read existing entry");
    let mut lines = contents.lines();

    let first_line = lines.next()?;

    if first_line != "---" {
        return None;
    }

    let metadata_block = lines
        .take_while_ref(|line| *line != "---")
        .join("\n");

    let meta = serde_yaml::from_str::<EntryMetadata>(&metadata_block).ok()?;

    if !meta.validate() {
        return None
    }

    // Read next "---" which is left by the take while
    lines.next()?;

    let rest = lines.join("\n");

    Some(Entry {
        journal: meta.journal,
        uuid: meta.uuid,
        creation_date: meta.creation_date,
        modified_date: meta.modified_date,
        markdown: rest,
    })
}

pub struct Vault {
    pub root: PathBuf,
    pub default_export: PathBuf,
    pub should_overwrite_existing: bool,
}

impl Vault {
    fn read_existing(&self) -> HashMap<String, PathBuf> {
        let walker = WalkDir::new(&self.root).into_iter();
        let result: HashMap<String, PathBuf> = walker.filter_entry(|e| !is_hidden(e) && is_markdown(e))
            .filter_map(|e| e.ok())
            .filter_map(|entry| {
                parse_entry(&entry)
                    .map(|entry_info| (entry_info.uuid, entry.into_path()))
            }).collect();

        result
    }

    fn should_overwrite(&self, _entry: &Entry, _path: &PathBuf) -> bool {
        self.should_overwrite_existing
    }

    pub async fn export_entries(&self, entries: &mut (impl Stream<Item=sqlx::Result<Entry>> + Unpin)) -> Result<(), anyhow::Error> {
        let existing = self.read_existing();

        while let Some(entry) = entries.try_next().await? {
            match existing.get(&entry.uuid) {
                Some(path) => {
                    if self.should_overwrite(&entry, path) {
                        tokio::fs::write(path, entry.contents()).await?;
                    }
                }

                None => {
                    tokio::fs::write(self.default_export.join(entry.default_filename()), entry.contents()).await?;
                }
            }
        }

        Ok(())
    }
}
