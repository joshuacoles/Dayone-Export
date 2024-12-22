use obsidian_rust_interface::NoteReference;
use serde::{Deserialize, Serialize};
use serde_yml::Value;
use std::collections::HashMap;
use chrono::{DateTime, Utc};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub struct EntryMetadata {
    // A constant value of "dayone-import", present in the metadata
    #[serde(rename = "type")]
    pub note_type: String,

    pub journal: String,

    #[serde(rename = "dayoneId")]
    pub uuid: String,

    #[serde(rename = "createdAt")]
    pub creation_date: DateTime<Utc>,

    #[serde(rename = "lastModifiedAt")]
    pub modified_date: DateTime<Utc>,

    pub link: String,

    // pub tags: Vec<String>,
    /**
     * We support additional metadata in the notes, for example if they have been reviewed.
     */
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl EntryMetadata {
    pub fn validate(&self) -> bool {
        self.note_type == "dayone-import"
    }
}

impl EntryMetadata {
    pub fn new(
        journal: String,
        uuid: String,
        creation_date: DateTime<Utc>,
        modified_date: DateTime<Utc>, /*, tags: Vec<String>*/
    ) -> EntryMetadata {
        EntryMetadata {
            note_type: "dayone-import".to_string(),
            journal,

            link: format!("dayone://view?entryId={}", uuid),
            uuid,
            // tags,
            creation_date,
            modified_date,
            extra: HashMap::new(),
        }
    }

    pub fn without_extra_fields(&self) -> EntryMetadata {
        EntryMetadata {
            extra: HashMap::new(),
            ..self.clone()
        }
    }
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub metadata: EntryMetadata,
    pub markdown: String,
}

impl Entry {
    pub fn metadata(&self) -> &EntryMetadata {
        &self.metadata
    }

    // TODO: This has issues with unneeded escaping coming out
    pub fn title(&self) -> String {
        match self.markdown.trim().split('\n').next() {
            Some(first_line) if first_line.starts_with('#') => {
                first_line.replace('#', "").trim().to_string()
            }
            Some(first_line)
                if first_line
                    .chars()
                    .all(|x| x.is_whitespace() || x.is_alphanumeric()) =>
            {
                first_line.to_string()
            }
            _ => self
                .metadata
                .creation_date
                .format("%H-%M-%S")
                .to_string(),
        }
    }

    pub fn contents(&self) -> String {
        format!(
            "---\n{frontmatter}---\n\n{body}\n",
            frontmatter =
                serde_yml::to_string(self.metadata()).expect("Failed to serialise metadata"),
            body = self.markdown.replace('\\', ""),
        )
    }

    pub fn default_filename(&self) -> String {
        // Remove special characters disallowed by obsidian in file names.
        // If we don't like what comes out we can always rename it.
        let safe_title = self
            .title()
            .replace('/', " ")
            .replace('\\', " ")
            .replace(":", " ");

        format!(
            "{} {}",
            self.metadata
                .creation_date
                .format("%Y-%m-%d"),
            safe_title,
        )
    }
}

pub fn parse_entry(vn: &NoteReference) -> Option<Entry> {
    let (metadata, contents) = vn.parts::<EntryMetadata>().ok()?;
    let metadata = metadata?;

    Some(Entry {
        metadata,
        markdown: contents,
    })
}
