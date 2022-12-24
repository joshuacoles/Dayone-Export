use std::collections::HashMap;
use std::path::PathBuf;
use futures::{Stream, TryStreamExt};
use walkdir::{DirEntry, WalkDir};
use crate::entry::{Entry, parse_entry};

fn is_hidden(entry: &DirEntry) -> bool {
    entry.file_name()
        .to_str()
        .map(|s| s.starts_with("."))
        .unwrap_or(false)
}

fn is_markdown(entry: &DirEntry) -> bool {
    entry.path().extension().contains(&"md")
}

pub struct Vault {
    pub root: PathBuf,
    pub default_export: PathBuf,
    pub should_update_existing: bool,
    pub group_by_journal: bool,
}

impl Vault {
    pub(crate) fn read_existing(&self) -> HashMap<String, (PathBuf, Entry)> {
        let walker = WalkDir::new(&self.root).into_iter();
        let result: HashMap<String, (PathBuf, Entry)> = walker
            .filter_entry(|e| !is_hidden(e))
            .filter_map(|e| e.ok())
            .filter(|e| !is_hidden(e) && is_markdown(e))
            .filter_map(|entry| {
                parse_entry(&entry)
                    .map(|entry_info| (entry_info.metadata.uuid.clone(), (entry.into_path(), entry_info)))
            }).collect();

        result
    }

    pub async fn export_entries(&self, entries: &mut (impl Stream<Item=sqlx::Result<Entry>> + Unpin), existing: &HashMap<String, (PathBuf, Entry)>) -> Result<(), anyhow::Error> {
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
