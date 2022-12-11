use std::path::{Path, PathBuf};
use time::{PrimitiveDateTime, UtcOffset};
use crate::{ConnectOptions, Entry, Error, Executor, Row, SqliteConnection, SqliteConnectOptions, Stream, TryStreamExt};

pub async fn connect_db(database_file: &PathBuf) -> Result<SqliteConnection, sqlx::Error> {
    SqliteConnectOptions::new()
        .filename(database_file)
        .read_only(true)
        .connect().await
}

async fn find_journal(conn: &mut SqliteConnection, name: &str) -> Result<i64, sqlx::Error> {
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

pub async fn get_entries<'a>(conn: &'a mut SqliteConnection, journal_name: &str) -> sqlx::Result<impl Stream<Item=Result<Entry, Error>> + Sized  + 'a> {
    let id = find_journal(conn, journal_name).await?;
    let entries = entries_for_journal(conn, id);
    Ok(entries)
}

pub fn entries_for_journal(conn: &mut SqliteConnection, id: i64) -> impl Stream<Item=Result<Entry, sqlx::Error>> + '_ {
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
              and (tag.ZNAME != 'grateful' or tag.ZNAME is null);
    ").bind(id));

    entries.map_ok(|row| Entry {
        uuid: row.get("uuid"),
        journal: row.get("journal"),
        markdown: row.get("markdown"),
        creation_date: row.get::<'_, PrimitiveDateTime, _>("creation_date").assume_offset(UtcOffset::UTC),
        modified_date: row.get::<'_, PrimitiveDateTime, _>("modified_date").assume_offset(UtcOffset::UTC),
    })
}
