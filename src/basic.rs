use std::path::PathBuf;
use filetime::{FileTime, set_file_times};
use crate::{Entry, Error, Stream, TryStreamExt};

pub async fn basic_export(entries: &mut (impl Stream<Item = sqlx::Result<Entry>> + Unpin), journal_root: PathBuf) -> Result<(), Error> {
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
