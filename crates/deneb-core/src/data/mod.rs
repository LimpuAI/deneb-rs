//! 数据类型定义
//!
//! 提供可视化中使用的数据结构，包括字段值、数据类型、列、数据表和模式。

use std::collections::HashMap;
use std::fmt;

use crate::error::CoreError;

/// 字段值
///
/// 表示单个数据单元的值，可以是数值、文本、时间戳、布尔值或空值。
#[derive(Clone, Debug, PartialEq)]
pub enum FieldValue {
    /// 数值
    Numeric(f64),
    /// 文本
    Text(String),
    /// 时间戳 (unix epoch seconds)
    Timestamp(f64),
    /// 布尔值
    Bool(bool),
    /// 空值
    Null,
}

impl FieldValue {
    /// 获取数值，如果不是数值类型则返回 None
    pub fn as_numeric(&self) -> Option<f64> {
        match self {
            FieldValue::Numeric(v) => Some(*v),
            _ => None,
        }
    }

    /// 获取文本，如果不是文本类型则返回 None
    pub fn as_text(&self) -> Option<&str> {
        match self {
            FieldValue::Text(v) => Some(v),
            _ => None,
        }
    }

    /// 获取时间戳，如果不是时间戳类型则返回 None
    pub fn as_timestamp(&self) -> Option<f64> {
        match self {
            FieldValue::Timestamp(v) => Some(*v),
            _ => None,
        }
    }

    /// 获取布尔值，如果不是布尔类型则返回 None
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            FieldValue::Bool(v) => Some(*v),
            _ => None,
        }
    }

    /// 判断是否为空值
    pub fn is_null(&self) -> bool {
        matches!(self, FieldValue::Null)
    }
}

impl PartialOrd for FieldValue {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (FieldValue::Numeric(a), FieldValue::Numeric(b)) => a.partial_cmp(b),
            (FieldValue::Timestamp(a), FieldValue::Timestamp(b)) => a.partial_cmp(b),
            (FieldValue::Text(a), FieldValue::Text(b)) => a.partial_cmp(b),
            (FieldValue::Bool(a), FieldValue::Bool(b)) => a.partial_cmp(b),
            _ => None,
        }
    }
}

impl fmt::Display for FieldValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FieldValue::Numeric(v) => write!(f, "{}", v),
            FieldValue::Text(v) => write!(f, "{}", v),
            FieldValue::Timestamp(v) => write!(f, "{}", v),
            FieldValue::Bool(v) => write!(f, "{}", v),
            FieldValue::Null => write!(f, "null"),
        }
    }
}

/// 数据类型
///
/// 表示字段的数据类型，用于确定可视化的编码方式。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DataType {
    /// 定量数据（连续数值）
    Quantitative,
    /// 时间数据
    Temporal,
    /// 名义数据（无序类别）
    Nominal,
    /// 序数数据（有序类别）
    Ordinal,
}

impl DataType {
    /// 判断是否为数值类型
    pub fn is_quantitative(&self) -> bool {
        matches!(self, DataType::Quantitative)
    }

    /// 判断是否为时间类型
    pub fn is_temporal(&self) -> bool {
        matches!(self, DataType::Temporal)
    }

    /// 判断是否为离散类型
    pub fn is_discrete(&self) -> bool {
        matches!(self, DataType::Nominal | DataType::Ordinal)
    }
}

impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataType::Quantitative => write!(f, "quantitative"),
            DataType::Temporal => write!(f, "temporal"),
            DataType::Nominal => write!(f, "nominal"),
            DataType::Ordinal => write!(f, "ordinal"),
        }
    }
}

/// 列
///
/// 表示数据表中的一列，包含列名、数据类型和值序列。
#[derive(Clone, Debug, PartialEq)]
pub struct Column {
    /// 列名
    pub name: String,
    /// 数据类型
    pub data_type: DataType,
    /// 值序列
    pub values: Vec<FieldValue>,
}

impl Column {
    /// 创建新列
    pub fn new(name: impl Into<String>, data_type: DataType, values: Vec<FieldValue>) -> Self {
        Self {
            name: name.into(),
            data_type,
            values,
        }
    }

    /// 创建空列
    pub fn empty(name: impl Into<String>, data_type: DataType) -> Self {
        Self {
            name: name.into(),
            data_type,
            values: Vec::new(),
        }
    }

    /// 获取列长度
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// 判断列是否为空
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// 获取指定索引的值
    pub fn get(&self, index: usize) -> Option<&FieldValue> {
        self.values.get(index)
    }

    /// 推入一个值
    pub fn push(&mut self, value: FieldValue) {
        self.values.push(value);
    }

    /// 扩展多个值
    pub fn extend(&mut self, values: impl IntoIterator<Item = FieldValue>) {
        self.values.extend(values);
    }

    /// 清空列
    pub fn clear(&mut self) {
        self.values.clear();
    }
}

/// 数据表
///
/// 表示完整的数据集，由多个列和对应的模式组成。
/// Schema 持有列名到索引的映射，加速列查找。
#[derive(Clone, Debug, PartialEq)]
pub struct DataTable {
    /// 列集合
    pub columns: Vec<Column>,
    /// 模式（列名/类型元信息 + 快速索引）
    pub schema: Schema,
}

impl DataTable {
    /// 创建新数据表
    pub fn new() -> Self {
        Self {
            columns: Vec::new(),
            schema: Schema::new(),
        }
    }

    /// 创建带列的数据表
    pub fn with_columns(columns: Vec<Column>) -> Self {
        let schema = columns.iter().fold(Schema::new(), |mut s, c| {
            s.add_field(&c.name, c.data_type);
            s
        });
        Self { columns, schema }
    }

    /// 添加列
    pub fn add_column(&mut self, column: Column) {
        self.schema.add_field(&column.name, column.data_type);
        self.columns.push(column);
    }

    /// 按列名获取列
    pub fn get_column(&self, name: &str) -> Option<&Column> {
        self.schema
            .index_of(name)
            .and_then(|idx| self.columns.get(idx))
    }

    /// 按列名获取可变列
    pub fn get_column_mut(&mut self, name: &str) -> Option<&mut Column> {
        self.schema
            .index_of(name)
            .and_then(|idx| self.columns.get_mut(idx))
    }

    /// 按索引获取列
    pub fn get_column_by_index(&self, index: usize) -> Option<&Column> {
        self.columns.get(index)
    }

    /// 获取行数（返回第一列的长度，如果无列则返回 0）
    pub fn row_count(&self) -> usize {
        self.columns.first().map_or(0, |c| c.len())
    }

    /// 获取列数
    pub fn column_count(&self) -> usize {
        self.columns.len()
    }

    /// 判断表是否为空
    pub fn is_empty(&self) -> bool {
        self.columns.is_empty() || self.columns.iter().all(|c| c.is_empty())
    }

    /// 获取所有列名
    pub fn column_names(&self) -> Vec<&str> {
        self.columns.iter().map(|c| c.name.as_str()).collect()
    }

    /// 验证所有列长度是否一致
    pub fn validate(&self) -> Result<(), CoreError> {
        if self.columns.is_empty() {
            return Ok(());
        }

        let first_len = self.columns[0].len();
        for (_i, col) in self.columns.iter().enumerate() {
            if col.len() != first_len {
                return Err(CoreError::InvalidEncoding {
                    field: col.name.clone(),
                    reason: format!(
                        "column length mismatch: expected {}, got {}",
                        first_len,
                        col.len()
                    ),
                });
            }
        }

        Ok(())
    }

    /// 清空所有列和模式
    pub fn clear(&mut self) {
        self.columns.clear();
        self.schema.clear();
    }
}

impl Default for DataTable {
    fn default() -> Self {
        Self::new()
    }
}

/// 模式
///
/// 定义数据表的结构，提供列名到索引的快速查找。
#[derive(Clone, Debug, PartialEq)]
pub struct Schema {
    /// 字段列表 (字段名, 数据类型)
    pub fields: Vec<(String, DataType)>,
    /// 字段名到索引的映射
    field_index: HashMap<String, usize>,
}

impl Schema {
    /// 创建新模式
    pub fn new() -> Self {
        Self {
            fields: Vec::new(),
            field_index: HashMap::new(),
        }
    }

    /// 添加字段
    pub fn add_field(&mut self, name: impl Into<String>, data_type: DataType) {
        let name = name.into();
        let index = self.fields.len();
        self.fields.push((name.clone(), data_type));
        self.field_index.insert(name, index);
    }

    /// 按名称查找字段索引
    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.field_index.get(name).copied()
    }

    /// 按名称查找字段类型
    pub fn type_of(&self, name: &str) -> Option<DataType> {
        self.index_of(name)
            .map(|idx| self.fields[idx].1)
    }

    /// 判断是否包含指定字段
    pub fn has_field(&self, name: &str) -> bool {
        self.field_index.contains_key(name)
    }

    /// 获取字段数量
    pub fn len(&self) -> usize {
        self.fields.len()
    }

    /// 判断是否为空
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// 从数据表推断模式
    pub fn infer_from_table(table: &DataTable) -> Self {
        let mut schema = Self::new();
        for column in &table.columns {
            schema.add_field(column.name.clone(), column.data_type);
        }
        schema
    }

    /// 清空模式
    pub fn clear(&mut self) {
        self.fields.clear();
        self.field_index.clear();
    }
}

impl Default for Schema {
    fn default() -> Self {
        Self::new()
    }
}

/// 从 DataTable 构建 Schema
impl From<&DataTable> for Schema {
    fn from(table: &DataTable) -> Self {
        Self::infer_from_table(table)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_value() {
        let num = FieldValue::Numeric(42.0);
        assert_eq!(num.as_numeric(), Some(42.0));
        assert_eq!(num.as_text(), None);
        assert!(!num.is_null());

        let text = FieldValue::Text("hello".to_string());
        assert_eq!(text.as_text(), Some("hello"));
        assert_eq!(text.as_numeric(), None);

        let null = FieldValue::Null;
        assert!(null.is_null());
        assert_eq!(null.as_numeric(), None);
    }

    #[test]
    fn test_field_value_comparison() {
        let num1 = FieldValue::Numeric(1.0);
        let num2 = FieldValue::Numeric(2.0);
        assert_eq!(num1.partial_cmp(&num2), Some(std::cmp::Ordering::Less));

        let text1 = FieldValue::Text("a".to_string());
        let text2 = FieldValue::Text("b".to_string());
        assert_eq!(text1.partial_cmp(&text2), Some(std::cmp::Ordering::Less));

        // 不同类型无法比较
        assert_eq!(num1.partial_cmp(&text1), None);
    }

    #[test]
    fn test_data_type() {
        assert!(DataType::Quantitative.is_quantitative());
        assert!(DataType::Temporal.is_temporal());
        assert!(DataType::Nominal.is_discrete());
        assert!(DataType::Ordinal.is_discrete());

        assert!(!DataType::Quantitative.is_discrete());
        assert!(!DataType::Nominal.is_quantitative());
    }

    #[test]
    fn test_column() {
        let mut col = Column::new("value", DataType::Quantitative, vec![
            FieldValue::Numeric(1.0),
            FieldValue::Numeric(2.0),
        ]);

        assert_eq!(col.len(), 2);
        assert!(!col.is_empty());
        assert_eq!(col.get(0), Some(&FieldValue::Numeric(1.0)));
        assert_eq!(col.get(5), None);

        col.push(FieldValue::Numeric(3.0));
        assert_eq!(col.len(), 3);

        col.clear();
        assert!(col.is_empty());
    }

    #[test]
    fn test_data_table() {
        let col1 = Column::new("x", DataType::Quantitative, vec![
            FieldValue::Numeric(1.0),
            FieldValue::Numeric(2.0),
        ]);
        let col2 = Column::new("y", DataType::Quantitative, vec![
            FieldValue::Numeric(3.0),
            FieldValue::Numeric(4.0),
        ]);

        let mut table = DataTable::with_columns(vec![col1, col2]);

        assert_eq!(table.column_count(), 2);
        assert_eq!(table.row_count(), 2);
        assert!(!table.is_empty());

        assert_eq!(table.get_column("x").map(|c| c.name.as_str()), Some("x"));
        assert_eq!(table.get_column("z"), None);

        assert_eq!(table.column_names(), vec!["x", "y"]);

        // 验证一致性
        assert!(table.validate().is_ok());

        // 添加不一致的列
        table.add_column(Column::new("bad", DataType::Nominal, vec![
            FieldValue::Text("only one".to_string()),
        ]));
        assert!(table.validate().is_err());
    }

    #[test]
    fn test_schema() {
        let mut schema = Schema::new();
        schema.add_field("x", DataType::Quantitative);
        schema.add_field("y", DataType::Quantitative);

        assert_eq!(schema.len(), 2);
        assert!(schema.has_field("x"));
        assert!(!schema.has_field("z"));

        assert_eq!(schema.index_of("x"), Some(0));
        assert_eq!(schema.index_of("y"), Some(1));
        assert_eq!(schema.index_of("z"), None);

        assert_eq!(schema.type_of("x"), Some(DataType::Quantitative));
        assert_eq!(schema.type_of("z"), None);
    }

    #[test]
    fn test_schema_from_table() {
        let table = DataTable::with_columns(vec![
            Column::new("category", DataType::Nominal, vec![]),
            Column::new("value", DataType::Quantitative, vec![]),
        ]);

        let schema = Schema::infer_from_table(&table);
        assert_eq!(schema.len(), 2);
        assert!(schema.has_field("category"));
        assert!(schema.has_field("value"));
    }
}
