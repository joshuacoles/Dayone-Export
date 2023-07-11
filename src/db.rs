use std::path::PathBuf;
use itertools::Itertools;
use sqlx::sqlite::SqliteRow;
use time::{PrimitiveDateTime, UtcOffset};
use futures::TryStreamExt;
use crate::{ConnectOptions, Executor, Row, SqliteConnection, SqliteConnectOptions};
use crate::entry::{Entry, EntryMetadata};

pub async fn connect_db(database_file: &PathBuf) -> sqlx::Result<SqliteConnection> {
    SqliteConnectOptions::new()
        .filename(database_file)
        .read_only(true)
        .connect().await
}

pub async fn entries_for_journals(conn: &mut SqliteConnection, journals: &[String]) -> sqlx::Result<Vec<Entry>> {
    let mut entries: Vec<Entry> = Vec::new();

    for journal in journals {
        let mut a = entries_for_journal(conn, journal).await?;
        entries.append(&mut a);
    }

    Ok(entries)
}

pub async fn entries_for_journal(conn: &mut SqliteConnection, name: &str) -> sqlx::Result<Vec<Entry>> {
    let entries = conn.fetch(
        sqlx::query("
            select journal.ZNAME                                    as journal,
                   ZUUID                                            as uuid,
                   ZMARKDOWNTEXT                                    as markdown,
                   datetime(ZCREATIONDATE, 'unixepoch', '31 years') as creation_date,
                   datetime(ZMODIFIEDDATE, 'unixepoch', '31 years') as modified_date
            from ZENTRY
                     left join ZJOURNAL journal on ZENTRY.ZJOURNAL = journal.Z_PK
            where journal.ZNAME = ?;
    ").bind(name)).try_collect::<Vec<SqliteRow>>().await?;

    let entries: Vec<Entry> = entries.iter()
        .group_by(|row| row.get::<'_, String, _>("uuid"))
        .into_iter()
        .map(|(uuid, rows)| {
            let mut rows = rows.peekable();
            let row = rows.peek().unwrap();
            let mut entry = Entry {
                markdown: row.get("markdown"),
                metadata: EntryMetadata::new(
                    row.get("journal"),
                    uuid,
                    row.get::<'_, PrimitiveDateTime, _>("creation_date").assume_offset(UtcOffset::UTC),
                    row.get::<'_, PrimitiveDateTime, _>("modified_date").assume_offset(UtcOffset::UTC),
                    // vec![],
                ),
            };

            // BCK: This is done later peeked reference means we can't call map until after extracting the other properties
            // let tags: Vec<String> = rows.map(|row| row.get::<'_, String, _>("tag")).filter(|tag| !tag.is_empty()).collect();
            // entry.metadata.tags = tags;

            entry
        })
        .collect();

    Ok(entries)
}
