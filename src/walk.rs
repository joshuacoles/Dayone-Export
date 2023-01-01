use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};
use crate::entry::{Entry, EntryMetadata, parse_entry};

fn is_hidden(entry: &DirEntry) -> bool {
    entry.file_name()
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}

fn is_markdown(entry: &DirEntry) -> bool {
    entry.path().extension().contains(&"md")
}

pub struct Vault {
    pub root: PathBuf,
    pub default_export: PathBuf,
    pub should_update_existing_content: bool,
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

    pub async fn export_entries(&self, entries: &Vec<Entry>, existing: &HashMap<String, (PathBuf, Entry)>) -> Result<(), anyhow::Error> {
        println!("Found {} existing entries", existing.len());

        if self.should_update_existing_content {
            println!("These will be overwritten in place with updated content if newer DayOne content is available.");
        }

        for entry in entries {
            match existing.get(&entry.metadata.uuid) {
                Some((path, existing_entry)) => {
                    // If we want to update existing content and we have newer content to serve, replace file entirely.
                    if self.should_update_existing_content && existing_entry.metadata.modified_date < entry.metadata.modified_date {
                        println!("Updating entry at {}", path.to_string_lossy());
                        tokio::fs::write(path, entry.contents()).await?;
                    } else if existing_entry.metadata != entry.metadata {
                        // If we have new metadata to serve, for example new tags, always update,
                        // but keep content and new metadata fields.

                        // FIXME: This triggers too much as the incoming entry will never have any
                        //  extra fields.

                        println!("Updating metadata entry at {}", path.to_string_lossy());

                        let updated_metadata = EntryMetadata {
                            extra: existing_entry.metadata.extra.clone(),
                            ..entry.metadata.clone()
                        };

                        let updated_entry = Entry { metadata: updated_metadata, markdown: existing_entry.markdown.clone().trim_start().to_string() };
                        tokio::fs::write(path, updated_entry.contents()).await?;
                    }
                }

                None => {
                    let parent = if self.group_by_journal { self.default_export.join(&entry.metadata.journal) } else { self.default_export.clone() };
                    let path = number_existing(&parent, &entry.default_filename(), "md");
                    tokio::fs::write(path, entry.contents()).await?;
                }
            }
        }

        Ok(())
    }
}

fn number_existing(root: &Path, name: &str, extension: &str) -> PathBuf {
    let mut path = root.join(format!("{}.{}", name, extension));
    let mut i = 2;

    while path.exists() {
        path = path.with_file_name(format!(
            "{name} ({i}).{extension}",
            name = name,
            extension = extension,
            i = i
        ));

        i += 1;
    }

    path
}
