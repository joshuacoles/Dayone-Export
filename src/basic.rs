use std::path::PathBuf;
use filetime::{FileTime, set_file_times};
use crate::{Stream, TryStreamExt};
use crate::walk::Vault;
use anyhow::{Context, Result};
use crate::entry::Entry;

pub async fn basic_export(entries: &mut (impl Stream<Item = sqlx::Result<Entry>> + Unpin), journal_root: PathBuf) -> Result<()> {
    while let Some(entry) = entries.try_next().await? {
        let file_name = entry.default_filename();

        let file_path = journal_root.join(file_name);

        tokio::fs::write(&file_path, entry.contents()).await
            .context(format!("Writing entry {} to '{:?}'", entry.metadata.uuid, &file_path))?;

        set_file_times(
            &file_path,
            FileTime::from_unix_time(entry.metadata.creation_date.unix_timestamp(), 0),
            FileTime::now(),
        )?;
    }

    Ok(())
}

pub async fn walk_export(vault: &Vault, entries: &mut (impl Stream<Item = sqlx::Result<Entry>> + Unpin)) -> Result<()> {
    vault.export_entries(entries).await?;

    Ok(())
}
