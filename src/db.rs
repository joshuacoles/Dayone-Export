use crate::entry::{Entry, EntryMetadata};
use crate::{ConnectOptions, Executor, Row, SqliteConnectOptions, SqliteConnection};
use futures::TryStreamExt;
use itertools::Itertools;
use sqlx::sqlite::SqliteRow;
use std::path::PathBuf;
use chrono::NaiveDateTime;

pub async fn connect_db(database_file: &PathBuf) -> sqlx::Result<SqliteConnection> {
    SqliteConnectOptions::new()
        .filename(database_file)
        .read_only(true)
        .connect()
        .await
}

pub async fn entries_for_journals(
    conn: &mut SqliteConnection,
    journals: &[String],
) -> sqlx::Result<Vec<Entry>> {
    let mut entries: Vec<Entry> = Vec::new();

    for journal in journals {
        let mut a = entries_for_journal(conn, journal).await?;
        entries.append(&mut a);
    }

    Ok(entries)
}

pub async fn entries_for_journal(
    conn: &mut SqliteConnection,
    name: &str,
) -> sqlx::Result<Vec<Entry>> {
    let entries = conn
        .fetch(
            sqlx::query(
                "
            select journal.ZNAME                                                        as journal,
                   ZUUID                                                                as uuid,
                   ZMARKDOWNTEXT                                                        as markdown,
                   datetime(datetime(ZCREATIONDATE, 'unixepoch', '31 years'), '+1 day') as creation_date,
                   datetime(datetime(ZMODIFIEDDATE, 'unixepoch', '31 years'), '+1 day') as modified_date
            from ZENTRY
                     left join ZJOURNAL journal on ZENTRY.ZJOURNAL = journal.Z_PK
            where journal.ZNAME = ?;
    ",
            )
            .bind(name),
        )
        .try_collect::<Vec<SqliteRow>>()
        .await?;

    let entries: Vec<Entry> = entries
        .iter()
        .chunk_by(|row| row.get::<'_, String, _>("uuid"))
        .into_iter()
        .map(|(uuid, rows)| {
            let mut rows = rows.peekable();
            let row = rows.peek().unwrap();
            let entry = Entry {
                markdown: row.get("markdown"),
                metadata: EntryMetadata::new(
                    row.get("journal"),
                    uuid,
                    row.get::<'_, NaiveDateTime, _>("creation_date")
                        .and_utc(),
                    row.get::<'_, NaiveDateTime, _>("modified_date")
                        .and_utc(),
                ),
            };
            entry
        })
        .collect();

    Ok(entries)
}
