use std::path::PathBuf;
use filetime::{FileTime, set_file_times};
use crate::{Entry, Stream, TryStreamExt};
use crate::walk::Vault;
use anyhow::Result;

pub async fn basic_export(entries: &mut (impl Stream<Item = sqlx::Result<Entry>> + Unpin), journal_root: PathBuf) -> Result<()> {
    while let Some(entry) = entries.try_next().await? {
        let file_name = entry.default_filename();

        let file_path = journal_root.join(file_name);

        tokio::fs::write(&file_path, entry.contents()).await?;

        set_file_times(
            &file_path,
            FileTime::from_unix_time(entry.creation_date.unix_timestamp(), 0),
            FileTime::now(),
        )?;
    }

    Ok(())
}

pub async fn walk_export(entries: &mut (impl Stream<Item = sqlx::Result<Entry>> + Unpin)) -> Result<()> {
    let vault = Vault {
        root: PathBuf::from("/Users/joshuacoles/Developer/checkouts/joshuacoles/dayone-export-standalone/out"),
        default_export: PathBuf::from("/Users/joshuacoles/Developer/checkouts/joshuacoles/dayone-export-standalone/out/journals"),
        should_overwrite_existing: true,
    };

    vault.export_entries(entries).await?;

    Ok(())
}
