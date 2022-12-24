use std::path::PathBuf;
use time::{PrimitiveDateTime, UtcOffset};
use crate::{ConnectOptions, Executor, Row, SqliteConnection, SqliteConnectOptions, Stream, TryStreamExt};
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

pub async fn get_entries<'a>(conn: &'a mut SqliteConnection, journal_name: &str) -> sqlx::Result<impl Stream<Item=sqlx::Result<Entry>> + Sized  + 'a> {
    let id = find_journal(conn, journal_name).await?;
    let entries = entries_for_journal(conn, id);
    Ok(entries)
}

pub fn entries_for_journal(conn: &mut SqliteConnection, id: i64) -> impl Stream<Item=sqlx::Result<Entry>> + '_ {
    let entries = conn.fetch(
        sqlx::query("
            select journal.ZNAME                                    as journal,
                   ZUUID                                            as uuid,
                   ZMARKDOWNTEXT                                    as markdown,
                   datetime(ZCREATIONDATE, 'unixepoch', '31 years') as creation_date,
                   datetime(ZMODIFIEDDATE, 'unixepoch', '31 years') as modified_date
            from ZENTRY
                     left join ZJOURNAL journal on ZENTRY.ZJOURNAL = journal.Z_PK
                     left join Z_12TAGS tag_entry on ZENTRY.Z_PK = tag_entry.Z_12ENTRIES
                     left join ZTAG tag on tag.Z_PK = tag_entry.Z_45TAGS1
            where journal.Z_PK = ?
              and (tag.ZNAME != 'grateful' or tag.ZNAME is null or tag.ZNAME == 'obsidian');
    ").bind(id));

    entries.map_ok(|row| Entry {
        markdown: row.get("markdown"),
        metadata: EntryMetadata::new(
            row.get("journal"),
            row.get("uuid"),
            row.get::<'_, PrimitiveDateTime, _>("creation_date").assume_offset(UtcOffset::UTC),
            row.get::<'_, PrimitiveDateTime, _>("modified_date").assume_offset(UtcOffset::UTC),
            Vec::new(),
        ),
    })
}
