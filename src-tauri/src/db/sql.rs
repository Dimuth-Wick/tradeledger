//! Conversions between Rust types and SQLite values that rusqlite doesn't
//! provide itself.

use std::str::FromStr;

use rusqlite::Row;
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use rust_decimal::Decimal;

/// Wrapper so a `Decimal` can be written to / read from a TEXT column.
/// (Rust doesn't allow implementing rusqlite's traits directly on a type from
/// another crate, so we wrap it.)
pub struct SqlDec(pub Decimal);

impl ToSql for SqlDec {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.0.to_string()))
    }
}

impl FromSql for SqlDec {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        Decimal::from_str(value.as_str()?)
            .map(SqlDec)
            .map_err(|e| FromSqlError::Other(Box::new(e)))
    }
}

/// Read a NOT NULL decimal column.
pub fn dec(row: &Row<'_>, col: &str) -> rusqlite::Result<Decimal> {
    Ok(row.get::<_, SqlDec>(col)?.0)
}

/// Read a nullable decimal column.
pub fn opt_dec(row: &Row<'_>, col: &str) -> rusqlite::Result<Option<Decimal>> {
    Ok(row.get::<_, Option<SqlDec>>(col)?.map(|d| d.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn decimal_keeps_every_digit() {
        let conn = Connection::open_in_memory().unwrap();
        let original = Decimal::from_str("0.000000012345678901").unwrap();
        let back: Decimal = conn
            .query_row("SELECT ?1 AS v", [SqlDec(original)], |r| dec(r, "v"))
            .unwrap();
        assert_eq!(back, original);
        assert_eq!(back.to_string(), "0.000000012345678901");
    }

    #[test]
    fn invalid_decimal_text_is_an_error_not_zero() {
        let conn = Connection::open_in_memory().unwrap();
        let result = conn.query_row("SELECT 'abc' AS v", [], |r| dec(r, "v"));
        assert!(result.is_err());
    }
}
