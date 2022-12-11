use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::exit;
use walkdir::{DirEntry, WalkDir};
use crate::Entry;

fn is_hidden(entry: &DirEntry) -> bool {
    entry.file_name()
        .to_str()
        .map(|s| s.starts_with("."))
        .unwrap_or(false)
}

fn is_markdown(entry: &DirEntry) -> bool {
    entry.path().extension().contains(&"md")
}

pub fn read_existing() -> HashMap<String, PathBuf> {
    let walker = WalkDir::new("/Users/joshuacoles/Obsidian Sync/My Life").into_iter();
    let result: HashMap<String, PathBuf> = walker.filter_entry(|e| !is_hidden(e) && is_markdown(e))
        .filter_map(|e| e.ok())
        .filter_map(|entry| {
            parse_metadata(&entry)
                .map(|entry_info| (entry_info.uuid, entry.into_path()))
        }).collect();

    result
}

pub fn write(existing: HashMap<String, PathBuf>, entries: Vec<Entry>) {
    for entry in entries {
        match existing.get(&entry.uuid) {
            Some(path) => {
                let should_overwrite = true;

                if should_overwrite {
                    fs::write(path, entry.contents()).unwrap();
                }
            },
            None => {
                let root: PathBuf = PathBuf::new();
                fs::write(root.join(entry.default_filename()), entry.contents()).unwrap();
            }
        }
    }
}

pub fn main_2() -> ! {
    let existing = read_existing();
    write(existing, Vec::new());

    exit(0);
}

fn parse_metadata(p0: &DirEntry) -> Option<Entry> {
    todo!()
}
