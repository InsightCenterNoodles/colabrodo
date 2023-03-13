//! Common table types as sourced from the spec

use crate::value_tools::*;

/// A selection of a table
#[derive(CBORTransform, Debug, Clone)]
pub struct Selection {
    pub name: String,
    pub rows: Option<Vec<i64>>,
    pub row_ranges: Option<Vec<i64>>,
}

/// Table column definition
#[derive(CBORTransform, Debug, Clone)]
pub struct TableColumnInfo {
    pub name: String,
    #[vserde(rename = "type")]
    pub type_: String,
}

/// Initial table data dump
#[derive(CBORTransform, Debug, Default)]
pub struct TableInitData {
    pub columns: Vec<TableColumnInfo>,
    pub keys: Vec<i64>,
    pub data: Vec<Vec<Value>>,
    pub selections: Option<Vec<Selection>>,
}
