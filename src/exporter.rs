use crate::entry::{parse_entry, Entry, EntryMetadata};
use obsidian_rust_interface::Vault;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct ExportConfig {
    pub vault: Vault,
    pub default_export: PathBuf,
    pub should_update_existing_content: bool,
    pub group_by_journal: bool,
}

impl ExportConfig {
    pub(crate) fn read_existing(&self) -> HashMap<String, (PathBuf, Entry)> {
        let result: HashMap<String, (PathBuf, Entry)> = self
            .vault
            .notes()
            .filter_map(|note| note.ok())
            .filter_map(|note| parse_entry(&note).map(|entry| (note.path().to_path_buf(), entry)))
            .map(|(path, entry)| (entry.metadata.uuid.clone(), (path, entry)))
            .collect();

        result
    }

    pub async fn export_entries(
        &self,
        entries: &Vec<Entry>,
        existing: &HashMap<String, (PathBuf, Entry)>,
    ) -> Result<(), anyhow::Error> {
        println!("Found {} existing entries", existing.len());

        if self.should_update_existing_content {
            println!("These will be overwritten in place with updated content if newer DayOne content is available.");
        }

        for entry in entries {
            match existing.get(&entry.metadata.uuid) {
                Some((path, existing_entry)) => {
                    // If we want to update existing content and we have newer content to serve, replace file entirely.
                    if self.should_update_existing_content
                        && existing_entry.metadata.modified_date < entry.metadata.modified_date
                    {
                        println!("Updating entry at {}", path.to_string_lossy());
                        tokio::fs::write(path, entry.contents()).await?;
                    } else if existing_entry.metadata != entry.metadata.without_extra_fields() {
                        // If we have new metadata to serve, for example new tags, always update,
                        // but keep content and new metadata fields.

                        // FIXME: This triggers too much as the incoming entry will never have any
                        //  extra fields.

                        println!("Updating metadata entry at {}", path.to_string_lossy());

                        let updated_metadata = EntryMetadata {
                            extra: existing_entry.metadata.extra.clone(),
                            ..entry.metadata.clone()
                        };

                        let updated_entry = Entry {
                            metadata: updated_metadata,
                            markdown: existing_entry.markdown.clone().trim_start().to_string(),
                        };
                        tokio::fs::write(path, updated_entry.contents()).await?;
                    }
                }

                None => {
                    let parent = if self.group_by_journal {
                        self.default_export.join(&entry.metadata.journal)
                    } else {
                        self.default_export.clone()
                    };
                    let path = number_existing(&parent, &entry.default_filename(), "md");
                    tokio::fs::write(path, entry.contents()).await?;
                }
            }
        }

        Ok(())
    }
}

/// When there is an existing file, we add a (n) suffix to the filename.
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
