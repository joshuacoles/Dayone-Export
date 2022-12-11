use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::exit;
use walkdir::{DirEntry, WalkDir};
use crate::{Entry, EntryMetadata};
use itertools::Itertools;

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
    let contents = fs::read_to_string(p0.path()).unwrap();
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

struct Vault {
    root: PathBuf,
    default_export: PathBuf,
    should_overwrite_existing: bool,
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

    fn export_entries(&self, entries: Vec<Entry>) {
        let existing = self.read_existing();

        for entry in entries {
            match existing.get(&entry.uuid) {
                Some(path) => {
                    if self.should_overwrite(&entry, path) {
                        fs::write(path, entry.contents()).unwrap();
                    }
                }

                None => {
                    fs::write(self.default_export.join(entry.default_filename()), entry.contents()).unwrap();
                }
            }
        }
    }
}

pub fn main_2() -> ! {
    let vault = Vault {
        root: PathBuf::from("/Users/joshuacoles/Developer/checkouts/joshuacoles/dayone-export-standalone/out"),
        default_export: PathBuf::from("/Users/joshuacoles/Developer/checkouts/joshuacoles/dayone-export-standalone/out/journals"),
        should_overwrite_existing: true,
    };

    vault.export_entries(vec![]);

    exit(0)
}
