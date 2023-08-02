use obsidian_rust_interface::NoteReference;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::collections::HashMap;
use time::macros::format_description;
use time::OffsetDateTime;

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub struct EntryMetadata {
    // A constant value of "dayone-import", present in the metadata
    #[serde(rename = "type")]
    pub note_type: String,

    pub journal: String,

    #[serde(rename = "dayoneId")]
    pub uuid: String,

    #[serde(rename = "createdAt", with = "time::serde::rfc3339")]
    pub creation_date: OffsetDateTime,

    #[serde(rename = "lastModifiedAt", with = "time::serde::rfc3339")]
    pub modified_date: OffsetDateTime,

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
        creation_date: OffsetDateTime,
        modified_date: OffsetDateTime, /*, tags: Vec<String>*/
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
                .format(format_description!("[hour]-[minute]-[second]"))
                .expect("Failed to format time"),
        }
    }

    pub fn contents(&self) -> String {
        format!(
            "---\n{frontmatter}---\n\n{body}\n",
            frontmatter =
                serde_yaml::to_string(self.metadata()).expect("Failed to serialise metadata"),
            body = self.markdown.replace('\\', ""),
        )
    }

    pub fn default_filename(&self) -> String {
        format!(
            "{} {}",
            self.metadata
                .creation_date
                .format(format_description!("[year]-[month]-[day]"))
                .expect("Failed to format date"),
            self.title().replace('/', " "),
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
