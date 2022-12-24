use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
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

    let metadata = serde_yaml::from_str::<EntryMetadata>(&metadata_block).ok()?;

    if !metadata.validate() {
        return None;
    }

    // Read next "---" which is left by the take while
    lines.next()?;

    let rest = lines.join("\n");

    Some(Entry {
        metadata,
        markdown: rest,
    })
}

pub struct Vault {
    pub root: PathBuf,
    pub default_export: PathBuf,
    pub should_update_existing: bool,
    pub group_by_journal: bool,
}

impl Vault {
    fn read_existing(&self) -> HashMap<String, (PathBuf, Entry)> {
        let walker = WalkDir::new(&self.root).into_iter();
        let result: HashMap<String, (PathBuf, Entry)> = walker
            .filter_map(|e| e.ok())
            .filter(|e| !is_hidden(e) && is_markdown(e))
            .filter_map(|entry| {
                parse_entry(&entry)
                    .map(|entry_info| (entry_info.metadata.uuid.clone(), (entry.into_path(), entry_info)))
            }).collect();

        result
    }

    pub async fn export_entries(&self, entries: &mut (impl Stream<Item=sqlx::Result<Entry>> + Unpin)) -> Result<(), anyhow::Error> {
        let existing = self.read_existing();

        println!("Found {} existing entries", existing.len());

        if self.should_update_existing {
            println!("These will be overwritten in place with updated content if newer DayOne content is available.");
        }

        while let Some(entry) = entries.try_next().await? {
            match existing.get(&entry.metadata.uuid) {
                Some((path, existing_entry)) => {
                    if self.should_update_existing && existing_entry.metadata.modified_date < entry.metadata.modified_date {
                        println!("Updating entry at {}", path.to_string_lossy());
                        tokio::fs::write(path, entry.contents()).await?;
                    }
                }

                None => {
                    let path = if self.group_by_journal { self.default_export.join(&entry.metadata.journal) } else { self.default_export.clone() };
                    let path = path.join(entry.default_filename());

                    tokio::fs::write(path, entry.contents()).await?;
                }
            }
        }

        Ok(())
    }
}
