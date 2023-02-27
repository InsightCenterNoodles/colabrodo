use crate::value_tools::*;

// struct SelectionRange {
//     inc_key_start: i64,
//     inc_key_end: i64,
// }

#[derive(CBORTransform, Debug, Clone)]
pub struct Selection {
    pub name: String,
    pub rows: Option<Vec<i64>>,
    pub row_ranges: Option<Vec<i64>>,
}

#[derive(CBORTransform, Debug, Clone)]
pub struct TableColumnInfo {
    pub name: String,
    #[vserde(rename = "type")]
    pub type_: String,
}

#[derive(CBORTransform, Debug, Default)]
pub struct TableInitData {
    pub columns: Vec<TableColumnInfo>,
    pub keys: Vec<i64>,
    pub data: Vec<Vec<Value>>,
    pub selections: Option<Vec<Selection>>,
}
