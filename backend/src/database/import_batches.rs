//! Records of CSV imports.

use rusqlite::{Connection, OptionalExtension, Row, params};

use super::DbError;
use crate::models::{ImportBatch, WithId};

pub fn insert(conn: &Connection, b: &ImportBatch) -> Result<i64, DbError> {
    conn.execute(
        "INSERT INTO import_batches
            (exchange, market, file_name, imported_at, rows_total, rows_inserted, rows_duplicate)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            b.exchange,
            b.market,
            b.file_name,
            b.imported_at,
            b.rows_total,
            b.rows_inserted,
            b.rows_duplicate
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Records how many rows were new vs. already imported, once saving is done.
pub fn set_counts(
    conn: &Connection,
    id: i64,
    rows_inserted: u32,
    rows_duplicate: u32,
) -> Result<(), DbError> {
    conn.execute(
        "UPDATE import_batches SET rows_inserted = ?1, rows_duplicate = ?2 WHERE id = ?3",
        params![rows_inserted, rows_duplicate, id],
    )?;
    Ok(())
}

pub fn get(conn: &Connection, id: i64) -> Result<Option<WithId<ImportBatch>>, DbError> {
    Ok(conn
        .query_row("SELECT * FROM import_batches WHERE id = ?1", [id], from_row)
        .optional()?)
}

fn from_row(r: &Row<'_>) -> rusqlite::Result<WithId<ImportBatch>> {
    Ok(WithId {
        id: r.get("id")?,
        item: ImportBatch {
            exchange: r.get("exchange")?,
            market: r.get("market")?,
            file_name: r.get("file_name")?,
            imported_at: r.get("imported_at")?,
            rows_total: r.get("rows_total")?,
            rows_inserted: r.get("rows_inserted")?,
            rows_duplicate: r.get("rows_duplicate")?,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{open_in_memory, test_data};

    #[test]
    fn insert_and_get() {
        let conn = open_in_memory().unwrap();
        let batch = test_data::import_batch();
        let id = insert(&conn, &batch).unwrap();
        assert_eq!(get(&conn, id).unwrap(), Some(WithId { id, item: batch }));
        assert_eq!(get(&conn, id + 1).unwrap(), None);
    }
}
