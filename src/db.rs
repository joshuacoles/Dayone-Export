use crate::entry::{Entry, EntryMetadata};
use crate::{ConnectOptions, Filters, Row, SqliteConnectOptions, SqliteConnection};
use chrono::NaiveDateTime;
use futures::TryStreamExt;
use itertools::Itertools;
use sqlx::sqlite::SqliteRow;
use std::path::PathBuf;

pub async fn connect_db(database_file: &PathBuf) -> sqlx::Result<SqliteConnection> {
    SqliteConnectOptions::new()
        .filename(database_file)
        .read_only(true)
        .connect()
        .await
}

fn rows_to_entries(rows: &[SqliteRow]) -> Vec<Entry> {
    rows.iter()
        .chunk_by(|row| row.get::<'_, String, _>("uuid"))
        .into_iter()
        .map(|(uuid, rows)| {
            let mut rows = rows.peekable();
            let row = rows.peek().unwrap();
            Entry {
                markdown: row.get("markdown"),
                metadata: EntryMetadata::new(
                    row.get("journal"),
                    uuid,
                    row.get::<'_, NaiveDateTime, _>("creation_date").and_utc(),
                    row.get::<'_, NaiveDateTime, _>("modified_date").and_utc(),
                ),
            }
        })
        .collect()
}

pub async fn entries_for_filter(conn: &mut SqliteConnection, filters: &Filters) -> sqlx::Result<Vec<Entry>> {
    let mut query_builder = sqlx::QueryBuilder::new(
        "
            select journal.ZNAME                                                        as journal,
                   ZUUID                                                                as uuid,
                   ZMARKDOWNTEXT                                                        as markdown,
                   datetime(datetime(ZCREATIONDATE, 'unixepoch', '31 years'), '+1 day') as creation_date,
                   datetime(datetime(ZMODIFIEDDATE, 'unixepoch', '31 years'), '+1 day') as modified_date
            from ZENTRY
                     left join ZJOURNAL journal on ZENTRY.ZJOURNAL = journal.Z_PK
        "
    );

    let mut where_added = false;

    // Add journal filter if journals are specified
    if let Some(journals) = &filters.only_journals && !journals.is_empty() {
        query_builder.push(" WHERE journal.ZNAME IN (");
        let mut separated = query_builder.separated(", ");
        for journal in journals {
            separated.push_bind(journal);
        }
        separated.push_unseparated(")");
        where_added = true;
    }

    // Add creation date filters if specified
    if let Some(after) = &filters.after {
        query_builder.push(if where_added { " AND " } else { " WHERE " });
        query_builder.push(" ZCREATIONDATE >= strftime('%s', ?) - 978307200 ");
        query_builder.push_bind(after.to_rfc3339());
        where_added = true;
    }

    if let Some(before) = &filters.before {
        query_builder.push(if where_added { " AND " } else { " WHERE " });
        query_builder.push(" ZCREATIONDATE <= strftime('%s', ?) - 978307200 ");
        query_builder.push_bind(before.to_rfc3339());
    }

    let entries = query_builder
        .build()
        .fetch(conn)
        .try_collect::<Vec<SqliteRow>>()
        .await?;

    Ok(rows_to_entries(&entries))
}
