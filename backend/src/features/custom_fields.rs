//! User-defined trade fields (e.g. "Session": Asia/London/NY, "Confidence":
//! 1–10, "Followed plan?": yes/no) with validation and friendly errors.

use std::str::FromStr;

use rusqlite::Connection;
use rust_decimal::Decimal;
use serde::Deserialize;
use thiserror::Error;

use crate::database::custom_fields::{CustomField, FieldValue};
use crate::database::{self as db, DbError};
use crate::models::FieldType;

const MAX_NAME: usize = 40;
const MAX_OPTIONS: usize = 30;
const MAX_TEXT: usize = 500;
/// How a ticked checkbox is stored. Unticked = no value at all.
const CHECKED: &str = "true";

#[derive(Debug, Error)]
pub enum FieldError {
    #[error("name can't be empty")]
    EmptyName,
    #[error("name is too long (max 40 characters)")]
    NameTooLong,
    #[error("a field named \"{0}\" already exists")]
    Duplicate(String),
    #[error("a choice field needs at least one option")]
    NoOptions,
    #[error("too many options (max 30)")]
    TooManyOptions,
    #[error("options must be 1–40 characters and not repeat: {0:?}")]
    BadOption(String),
    #[error("option \"{0}\" is used by {1} trade(s); clear it there first")]
    OptionInUse(String, u32),
    #[error("{field}: {problem}")]
    BadValue { field: String, problem: String },
    #[error("{0} not found")]
    NotFound(&'static str),
    #[error(transparent)]
    Db(#[from] DbError),
}

impl From<rusqlite::Error> for FieldError {
    fn from(e: rusqlite::Error) -> Self {
        FieldError::Db(e.into())
    }
}

fn clean_name(conn: &Connection, name: &str, except: Option<i64>) -> Result<String, FieldError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(FieldError::EmptyName);
    }
    if name.chars().count() > MAX_NAME {
        return Err(FieldError::NameTooLong);
    }
    if db::custom_fields::name_taken(conn, name, except)? {
        return Err(FieldError::Duplicate(name.to_string()));
    }
    Ok(name.to_string())
}

/// Trimmed, non-empty, ≤ 40 chars, no repeats (ignoring case). Only choice
/// fields keep options; other types always store none.
fn clean_options(field_type: FieldType, options: &[String]) -> Result<Vec<String>, FieldError> {
    if field_type != FieldType::Select {
        return Ok(Vec::new());
    }
    let mut out: Vec<String> = Vec::new();
    for raw in options {
        let o = raw.trim();
        let repeated = out.iter().any(|x| x.eq_ignore_ascii_case(o));
        if o.is_empty() || o.chars().count() > MAX_NAME || repeated {
            return Err(FieldError::BadOption(raw.clone()));
        }
        out.push(o.to_string());
    }
    if out.is_empty() {
        return Err(FieldError::NoOptions);
    }
    if out.len() > MAX_OPTIONS {
        return Err(FieldError::TooManyOptions);
    }
    Ok(out)
}

pub fn list(conn: &Connection) -> Result<Vec<CustomField>, FieldError> {
    Ok(db::custom_fields::list(conn)?)
}

pub fn create(
    conn: &Connection,
    name: &str,
    field_type: FieldType,
    options: &[String],
) -> Result<i64, FieldError> {
    let name = clean_name(conn, name, None)?;
    let options = clean_options(field_type, options)?;
    Ok(db::custom_fields::create(
        conn, &name, field_type, &options,
    )?)
}

/// Renames and/or changes options. The type can't change: existing values
/// would stop making sense. An option still used by trades can't be removed.
pub fn update(
    conn: &Connection,
    id: i64,
    name: &str,
    options: &[String],
) -> Result<(), FieldError> {
    let field = db::custom_fields::get(conn, id)?.ok_or(FieldError::NotFound("field"))?;
    let name = clean_name(conn, name, Some(id))?;
    let options = clean_options(field.field_type, options)?;
    for old in &field.options {
        if !options.contains(old) {
            let used = db::custom_fields::count_value(conn, id, old)?;
            if used > 0 {
                return Err(FieldError::OptionInUse(old.clone(), used));
            }
        }
    }
    db::custom_fields::update(conn, id, &name, &options)?;
    Ok(())
}

pub fn delete(conn: &Connection, id: i64) -> Result<(), FieldError> {
    db::custom_fields::delete(conn, id)?
        .then_some(())
        .ok_or(FieldError::NotFound("field"))
}

/// Checks a typed value against its field and returns what to store
/// (`None` = clear the value).
pub fn normalize_value(field: &CustomField, raw: &str) -> Result<Option<String>, FieldError> {
    let v = raw.trim();
    let bad = |problem: &str| FieldError::BadValue {
        field: field.name.clone(),
        problem: problem.to_string(),
    };
    if v.is_empty() {
        return Ok(None);
    }
    match field.field_type {
        FieldType::Text => {
            if v.chars().count() > MAX_TEXT {
                return Err(bad("text is too long (max 500 characters)"));
            }
            Ok(Some(v.to_string()))
        }
        FieldType::Number => Decimal::from_str(v)
            .map(|d| Some(d.normalize().to_string()))
            .map_err(|_| bad("enter a number like 7 or 2.5")),
        FieldType::Select => field
            .options
            .iter()
            .find(|o| o.eq_ignore_ascii_case(v))
            .map(|o| Some(o.clone()))
            .ok_or_else(|| bad("pick one of the listed options")),
        FieldType::Checkbox => match v {
            "true" => Ok(Some(CHECKED.to_string())),
            "false" => Ok(None),
            _ => Err(bad("must be ticked or not")),
        },
    }
}

/// One value from the trade detail form. `value: None` or "" clears it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValueInput {
    pub field_id: i64,
    pub value: Option<String>,
}

/// Saves several values of one trade. All-or-nothing: one invalid value
/// rejects the whole batch and nothing changes.
pub fn set_values(
    conn: &Connection,
    trade_id: i64,
    values: &[ValueInput],
) -> Result<Vec<FieldValue>, FieldError> {
    let tx = conn.unchecked_transaction()?;
    if db::trades::get(&tx, trade_id)?.is_none() {
        return Err(FieldError::NotFound("trade"));
    }
    for input in values {
        let field =
            db::custom_fields::get(&tx, input.field_id)?.ok_or(FieldError::NotFound("field"))?;
        match normalize_value(&field, input.value.as_deref().unwrap_or(""))? {
            Some(v) => db::custom_fields::set_value(&tx, trade_id, field.id, &v)?,
            None => db::custom_fields::clear_value(&tx, trade_id, field.id)?,
        }
    }
    let saved = db::custom_fields::values_for_trade(&tx, trade_id)?;
    tx.commit()?;
    Ok(saved)
}

pub fn values(conn: &Connection, trade_id: i64) -> Result<Vec<FieldValue>, FieldError> {
    Ok(db::custom_fields::values_for_trade(conn, trade_id)?)
}

#[cfg(test)]
mod tests;
