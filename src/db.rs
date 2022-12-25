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

async fn find_journal(conn: &mut SqliteConnection, name: &str) -> sqlx::Result<i64> {
    let journal = conn.fetch_one(
        sqlx::query("
            select Z_PK as id
            from ZJOURNAL
            where ZNAME = ?;
        ").bind(name)
    ).await?;

    let result: i64 = journal.try_get("id")?;
    Ok(result)
}

pub async fn get_entries(conn: &mut SqliteConnection, journal_name: &str) -> sqlx::Result<Vec<Entry>> {
    let id = find_journal(conn, journal_name).await?;
    let entries = entries_for_journal(conn, id).await?;
    Ok(entries)
}

pub async fn entries_for_journal(conn: &mut SqliteConnection, id: i64) -> sqlx::Result<Vec<Entry>> {
    let entries = conn.fetch(
        sqlx::query("
            select journal.ZNAME                                    as journal,
                   ZUUID                                            as uuid,
                   ZMARKDOWNTEXT                                    as markdown,
                   datetime(ZCREATIONDATE, 'unixepoch', '31 years') as creation_date,
                   datetime(ZMODIFIEDDATE, 'unixepoch', '31 years') as modified_date,
                   tag.ZNAME                                        as tag
            from ZENTRY
                     left join ZJOURNAL journal on ZENTRY.ZJOURNAL = journal.Z_PK
                     left join Z_12TAGS tag_entry on ZENTRY.Z_PK = tag_entry.Z_12ENTRIES
                     left join ZTAG tag on tag.Z_PK = tag_entry.Z_45TAGS1
            where journal.Z_PK = ?
              and (tag.ZNAME != 'grateful' or tag.ZNAME is null or tag.ZNAME == 'obsidian');
    ").bind(id)).try_collect::<Vec<SqliteRow>>().await?;

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
                    vec![]
                ),
            };

            // BCK: This is done later peeked reference means we can't call map until after extracting the other properties
            let tags: Vec<String> = rows.map(|row| row.get::<'_, String, _>("tag")).filter(|tag| !tag.is_empty()).collect();
            entry.metadata.tags = tags;
            dbg!(&entry.metadata);

            entry
        })
        .collect();

    Ok(entries)
}
