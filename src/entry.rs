use time::OffsetDateTime;
use serde::{Deserialize, Serialize};
use time::macros::format_description;

#[derive(Serialize, Deserialize, Debug)]
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
    pub journal: String,
    pub uuid: String,
    pub markdown: String,
    pub creation_date: OffsetDateTime,
    pub modified_date: OffsetDateTime,
}

impl Entry {
    pub fn metadata(&self) -> EntryMetadata {
        EntryMetadata::new(self.journal.clone(), self.uuid.clone(), self.creation_date, self.modified_date)
    }

    pub fn title(&self) -> String {
        match self.markdown.trim().split('\n').next() {
            Some(first_line) if first_line.starts_with('#') => first_line.replace('#', "").trim().to_string(),
            Some(first_line) if first_line.chars().all(|x| x.is_whitespace() || x.is_alphanumeric()) => first_line.to_string(),
            _ => self.creation_date.format(format_description!("[hour]-[minute]-[second]")).expect("Failed to format time")
        }
    }

    pub fn contents(&self) -> String {
        format!(
            "---\n{frontmatter}---\n\n{body}\n",
            frontmatter = serde_yaml::to_string(&self.metadata()).expect("Failed to serialise metadata"),
            body = self.markdown.replace('\\', ""),
        )
    }

    pub fn default_filename(&self) -> String {
        format!(
            "{} {}.md",
            self.creation_date.format(format_description!("[year]-[month]-[day]")).expect("Failed to format date"),
            self.title().replace("/", " "),
        )
    }
}
