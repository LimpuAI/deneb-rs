//! JSON 解析器
//!
//! 解析 JSON 格式的数据为 DataTable，支持对象数组和列式格式。

use crate::data::{DataTable, Column, FieldValue, DataType};
use crate::error::{CoreError, DataFormat};

/// 解析 JSON 字符串为 DataTable
///
/// # Arguments
///
/// * `input` - JSON 格式的字符串
///
/// # Returns
///
/// 返回解析后的 DataTable，如果解析失败则返回错误
///
/// # Examples
///
/// ```no_run
/// use deneb_core::parser::json::parse_json;
/// let json = r#"[{"x": 1, "y": "a"}, {"x": 2, "y": "b"}]"#;
/// let table = parse_json(json).unwrap();
/// ```
#[cfg(feature = "json")]
pub fn parse_json(input: &str) -> Result<DataTable, CoreError> {
    let input = input.trim();
    if input.is_empty() {
        return Err(CoreError::empty_data());
    }

    // 尝试解析为 JSON Value
    let value: serde_json::Value = serde_json::from_str(input).map_err(|e| {
        CoreError::parse_error(format!("invalid JSON: {}", e), DataFormat::Json)
    })?;

    parse_json_value(value)
}

/// 从 serde_json::Value 构建 DataTable
///
/// # Arguments
///
/// * `value` - serde_json::Value
///
/// # Returns
///
/// 返回解析后的 DataTable，如果解析失败则返回错误
#[cfg(feature = "json")]
pub fn parse_json_value(value: serde_json::Value) -> Result<DataTable, CoreError> {
    match value {
        serde_json::Value::Array(arr) => {
            if arr.is_empty() {
                return Ok(DataTable::new());
            }

            // 检查是否为列式格式
            if let Ok(table) = try_parse_column_format(&arr) {
                return Ok(table);
            }

            // 对象数组格式
            parse_object_array(&arr)
        }
        serde_json::Value::Object(obj) => {
            // 可能是列式格式的根对象
            try_parse_column_format_root(&obj)
        }
        _ => Err(CoreError::parse_error(
            "JSON must be an array or object",
            DataFormat::Json,
        )),
    }
}

/// 解析对象数组格式
///
/// 格式：[{"x": 1, "y": "a"}, {"x": 2, "y": "b"}]
fn parse_object_array(arr: &[serde_json::Value]) -> Result<DataTable, CoreError> {
    // 收集所有字段名
    let mut field_names: Vec<String> = Vec::new();
    let mut field_values: std::collections::HashMap<String, Vec<FieldValue>> =
        std::collections::HashMap::new();

    for item in arr {
        if let Some(obj) = item.as_object() {
            for (key, value) in obj {
                if !field_names.contains(key) {
                    field_names.push(key.clone());
                }

                let field_value = parse_json_value_to_field(value);
                field_values
                    .entry(key.clone())
                    .or_insert_with(Vec::new)
                    .push(field_value);
            }
        }
    }

    // 填充缺失的字段为 Null
    for key in &field_names {
        field_values
            .entry(key.clone())
            .or_insert_with(|| vec![FieldValue::Null; arr.len()]);
    }

    // 确保所有字段长度一致
    for (_key, values) in &mut field_values {
        while values.len() < arr.len() {
            values.push(FieldValue::Null);
        }
    }

    // 构建列并推断类型
    let mut table = DataTable::new();

    for name in &field_names {
        let values = field_values.get(name).unwrap();

        // 推断类型
        let data_type = infer_type_from_values(values);

        let column = Column::new(name.clone(), data_type, values.clone());
        table.add_column(column);
    }

    Ok(table)
}

/// 尝试解析列式格式（从数组）
///
/// 格式：[{"columns": ["x", "y"], "types": ["quantitative", "nominal"], "data": [[1, "a"], [2, "b"]]}]
fn try_parse_column_format(arr: &[serde_json::Value]) -> Result<DataTable, CoreError> {
    if arr.len() != 1 {
        return Err(CoreError::parse_error(
            "column format must have exactly one object",
            DataFormat::Json,
        ));
    }

    let obj = arr[0]
        .as_object()
        .ok_or_else(|| CoreError::parse_error("column format root must be an object", DataFormat::Json))?;

    parse_column_format_object(obj)
}

/// 尝试解析列式格式（从根对象）
///
/// 格式：{"columns": ["x", "y"], "types": ["quantitative", "nominal"], "data": [[1, "a"], [2, "b"]]}
fn try_parse_column_format_root(obj: &serde_json::Map<String, serde_json::Value>) -> Result<DataTable, CoreError> {
    // 检查是否是列式格式（必须有 columns 和 data 字段）
    if obj.contains_key("columns") && obj.contains_key("data") {
        parse_column_format_object(obj)
    } else {
        Err(CoreError::parse_error(
            "unknown JSON format (expected object array or column format)",
            DataFormat::Json,
        ))
    }
}

/// 解析列式格式对象
fn parse_column_format_object(obj: &serde_json::Map<String, serde_json::Value>) -> Result<DataTable, CoreError> {
    let columns = obj
        .get("columns")
        .and_then(|v| v.as_array())
        .ok_or_else(|| CoreError::parse_error("missing 'columns' field", DataFormat::Json))?;

    let column_names: Vec<String> = columns
        .iter()
        .map(|v| {
            v.as_str()
                .ok_or_else(|| CoreError::parse_error("column name must be a string", DataFormat::Json))
                .map(|s| s.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;

    if column_names.is_empty() {
        return Err(CoreError::parse_error("no columns specified", DataFormat::Json));
    }

    // 获取可选的类型声明
    let declared_types: Option<Vec<DataType>> = obj
        .get("types")
        .and_then(|v| v.as_array())
        .map(|types| {
            types
                .iter()
                .map(|v| {
                    v.as_str()
                        .and_then(|s| parse_data_type_string(s))
                        .ok_or_else(|| {
                            CoreError::parse_error(format!("invalid data type: {}", v), DataFormat::Json)
                        })
                })
                .collect()
        })
        .transpose()?;

    let data = obj
        .get("data")
        .and_then(|v| v.as_array())
        .ok_or_else(|| CoreError::parse_error("missing 'data' field", DataFormat::Json))?;

    if data.is_empty() {
        // 空数据，创建空列
        let mut table = DataTable::new();
        for (i, name) in column_names.iter().enumerate() {
            let data_type = declared_types
                .as_ref()
                .and_then(|types| types.get(i).copied())
                .unwrap_or(DataType::Nominal);
            table.add_column(Column::empty(name, data_type));
        }
        return Ok(table);
    }

    // 验证数据行长度
    let expected_len = column_names.len();
    for (i, row) in data.iter().enumerate() {
        let row_array = row
            .as_array()
            .ok_or_else(|| {
                CoreError::parse_error(
                    format!("data row {} must be an array", i),
                    DataFormat::Json,
                )
            })?;

        if row_array.len() != expected_len {
            return Err(CoreError::parse_error(
                format!(
                    "data row {} has {} columns, expected {}",
                    i,
                    row_array.len(),
                    expected_len
                ),
                DataFormat::Json,
            ));
        }
    }

    // 构建列
    let mut table = DataTable::new();

    for (col_idx, col_name) in column_names.iter().enumerate() {
        let declared_type = declared_types.as_ref().and_then(|types| types.get(col_idx).copied());

        let mut values = Vec::new();
        for row in data {
            if let Some(row_array) = row.as_array() {
                if let Some(value) = row_array.get(col_idx) {
                    let field_value = parse_json_value_to_field(value);
                    values.push(field_value);
                }
            }
        }

        // 如果没有声明类型，则推断
        let data_type = if let Some(dt) = declared_type {
            dt
        } else {
            infer_type_from_values(&values)
        };

        let column = Column::new(col_name.clone(), data_type, values);
        table.add_column(column);
    }

    Ok(table)
}

/// 将 serde_json::Value 转换为 FieldValue
fn parse_json_value_to_field(value: &serde_json::Value) -> FieldValue {
    match value {
        serde_json::Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                FieldValue::Numeric(f)
            } else if let Some(i) = n.as_i64() {
                FieldValue::Numeric(i as f64)
            } else if let Some(u) = n.as_u64() {
                FieldValue::Numeric(u as f64)
            } else {
                FieldValue::Null
            }
        }
        serde_json::Value::String(s) => {
            let trimmed = s.trim();

            // 尝试解析时间戳
            if is_timestamp(trimmed) {
                if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(trimmed) {
                    return FieldValue::Timestamp(dt.timestamp() as f64);
                } else if let Ok(date) = chrono::NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
                    return FieldValue::Timestamp(
                        date.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp() as f64
                    );
                }
            }

            // 检查布尔值
            match trimmed.to_lowercase().as_str() {
                "true" | "yes" => return FieldValue::Bool(true),
                "false" | "no" => return FieldValue::Bool(false),
                _ => {}
            }

            FieldValue::Text(trimmed.to_string())
        }
        serde_json::Value::Bool(b) => FieldValue::Bool(*b),
        serde_json::Value::Null => FieldValue::Null,
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            // 嵌套结构转为字符串
            FieldValue::Text(value.to_string())
        }
    }
}

/// 从字段值列表推断数据类型
fn infer_type_from_values(values: &[FieldValue]) -> DataType {
    if values.is_empty() {
        return DataType::Nominal;
    }

    let mut type_counts: std::collections::HashMap<DataType, usize> = std::collections::HashMap::new();

    for value in values {
        let dt = match value {
            FieldValue::Numeric(_) => DataType::Quantitative,
            FieldValue::Timestamp(_) => DataType::Temporal,
            FieldValue::Bool(_) => DataType::Nominal, // 布尔值归为 Nominal
            FieldValue::Text(_) => DataType::Nominal,
            FieldValue::Null => continue,
        };
        *type_counts.entry(dt).or_insert(0) += 1;
    }

    if type_counts.is_empty() {
        // 全是 Null
        return DataType::Nominal;
    }

    // 按优先级返回：Quantitative > Temporal > Nominal
    if type_counts.contains_key(&DataType::Quantitative) {
        return DataType::Quantitative;
    }
    if type_counts.contains_key(&DataType::Temporal) {
        return DataType::Temporal;
    }
    DataType::Nominal
}

/// 检查字符串是否为时间戳格式
fn is_timestamp(s: &str) -> bool {
    if s.len() >= 10 && s.chars().nth(4) == Some('-') && s.chars().nth(7) == Some('-') {
        if chrono::DateTime::parse_from_rfc3339(s).is_ok() {
            return true;
        }
        if chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").is_ok() {
            return true;
        }
    }
    false
}

/// 从字符串解析数据类型
fn parse_data_type_string(s: &str) -> Option<DataType> {
    match s.to_lowercase().as_str() {
        "quantitative" => Some(DataType::Quantitative),
        "temporal" => Some(DataType::Temporal),
        "nominal" => Some(DataType::Nominal),
        "ordinal" => Some(DataType::Ordinal),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_object_array() {
        let json = r#"[{"x": 1, "y": "a"}, {"x": 2, "y": "b"}]"#;
        let table = parse_json(json).unwrap();

        assert_eq!(table.column_count(), 2);
        assert_eq!(table.row_count(), 2);

        let x_col = table.get_column("x").unwrap();
        assert_eq!(x_col.data_type, DataType::Quantitative);
        assert_eq!(x_col.get(0), Some(&FieldValue::Numeric(1.0)));
        assert_eq!(x_col.get(1), Some(&FieldValue::Numeric(2.0)));

        let y_col = table.get_column("y").unwrap();
        assert_eq!(y_col.data_type, DataType::Nominal);
        assert_eq!(y_col.get(0), Some(&FieldValue::Text("a".to_string())));
        assert_eq!(y_col.get(1), Some(&FieldValue::Text("b".to_string())));
    }

    #[test]
    fn test_parse_json_column_format() {
        let json = r#"{
            "columns": ["x", "y"],
            "types": ["quantitative", "nominal"],
            "data": [[1, "a"], [2, "b"]]
        }"#;
        let table = parse_json(json).unwrap();

        assert_eq!(table.column_count(), 2);
        assert_eq!(table.row_count(), 2);

        let x_col = table.get_column("x").unwrap();
        assert_eq!(x_col.data_type, DataType::Quantitative);
        assert_eq!(x_col.get(0), Some(&FieldValue::Numeric(1.0)));

        let y_col = table.get_column("y").unwrap();
        assert_eq!(y_col.data_type, DataType::Nominal);
    }

    #[test]
    fn test_parse_json_column_format_without_types() {
        let json = r#"{
            "columns": ["x", "y"],
            "data": [[1, "a"], [2, "b"]]
        }"#;
        let table = parse_json(json).unwrap();

        assert_eq!(table.column_count(), 2);

        let x_col = table.get_column("x").unwrap();
        assert_eq!(x_col.data_type, DataType::Quantitative);

        let y_col = table.get_column("y").unwrap();
        assert_eq!(y_col.data_type, DataType::Nominal);
    }

    #[test]
    fn test_parse_json_empty_array() {
        let json = r#"[]"#;
        let table = parse_json(json).unwrap();
        assert_eq!(table.column_count(), 0);
    }

    #[test]
    fn test_parse_json_empty_input() {
        let json = "";
        let result = parse_json(json);
        assert!(matches!(result, Err(CoreError::EmptyData)));
    }

    #[test]
    fn test_parse_json_invalid_json() {
        let json = r#"{"x": 1, "#;
        let result = parse_json(json);
        assert!(matches!(result, Err(CoreError::ParseError { .. })));
    }

    #[test]
    fn test_parse_json_missing_fields() {
        let json = r#"[{"x": 1, "y": "a"}, {"x": 2}]"#;
        let table = parse_json(json).unwrap();

        assert_eq!(table.column_count(), 2);

        let y_col = table.get_column("y").unwrap();
        assert_eq!(y_col.get(0), Some(&FieldValue::Text("a".to_string())));
        assert_eq!(y_col.get(1), Some(&FieldValue::Null));
    }

    #[test]
    fn test_parse_json_timestamp() {
        let json = r#"[{"date": "2023-01-01"}, {"date": "2023-01-02"}]"#;
        let table = parse_json(json).unwrap();

        let date_col = table.get_column("date").unwrap();
        assert_eq!(date_col.data_type, DataType::Temporal);

        if let Some(FieldValue::Timestamp(ts)) = date_col.get(0) {
            assert!(*ts > 0.0);
        } else {
            panic!("Expected Timestamp value");
        }
    }

    #[test]
    fn test_parse_json_bool() {
        let json = r#"[{"active": true}, {"active": false}]"#;
        let table = parse_json(json).unwrap();

        let active_col = table.get_column("active").unwrap();
        assert_eq!(active_col.data_type, DataType::Nominal);
        assert_eq!(active_col.get(0), Some(&FieldValue::Bool(true)));
        assert_eq!(active_col.get(1), Some(&FieldValue::Bool(false)));
    }

    #[test]
    fn test_parse_json_null() {
        let json = r#"[{"value": null}, {"value": 42}]"#;
        let table = parse_json(json).unwrap();

        let value_col = table.get_column("value").unwrap();
        assert_eq!(value_col.get(0), Some(&FieldValue::Null));
        assert_eq!(value_col.get(1), Some(&FieldValue::Numeric(42.0)));
    }

    #[test]
    fn test_parse_json_number_types() {
        let json = r#"[{"int": 42, "float": 3.14, "negative": -10}]"#;
        let table = parse_json(json).unwrap();

        let int_col = table.get_column("int").unwrap();
        assert_eq!(int_col.get(0), Some(&FieldValue::Numeric(42.0)));

        let float_col = table.get_column("float").unwrap();
        assert_eq!(float_col.get(0), Some(&FieldValue::Numeric(3.14)));

        let neg_col = table.get_column("negative").unwrap();
        assert_eq!(neg_col.get(0), Some(&FieldValue::Numeric(-10.0)));
    }

    #[test]
    fn test_parse_json_column_format_empty_data() {
        let json = r#"{
            "columns": ["x", "y"],
            "data": []
        }"#;
        let table = parse_json(json).unwrap();

        assert_eq!(table.column_count(), 2);
        assert_eq!(table.row_count(), 0);
    }

    #[test]
    fn test_parse_json_column_format_inconsistent_length() {
        let json = r#"{
            "columns": ["x", "y"],
            "data": [[1, "a"], [2]]
        }"#;
        let result = parse_json(json);
        assert!(matches!(result, Err(CoreError::ParseError { .. })));
    }

    #[test]
    fn test_parse_data_type_string() {
        assert_eq!(parse_data_type_string("quantitative"), Some(DataType::Quantitative));
        assert_eq!(parse_data_type_string("temporal"), Some(DataType::Temporal));
        assert_eq!(parse_data_type_string("nominal"), Some(DataType::Nominal));
        assert_eq!(parse_data_type_string("ordinal"), Some(DataType::Ordinal));
        assert_eq!(parse_data_type_string("invalid"), None);
    }

    #[test]
    fn test_parse_json_mixed_numeric_types() {
        // JSON 中整数和浮点数都应该解析为 Numeric
        let json = r#"[{"value": 1}, {"value": 2.5}, {"value": -3}]"#;
        let table = parse_json(json).unwrap();

        let col = table.get_column("value").unwrap();
        assert_eq!(col.data_type, DataType::Quantitative);
        assert_eq!(col.get(0), Some(&FieldValue::Numeric(1.0)));
        assert_eq!(col.get(1), Some(&FieldValue::Numeric(2.5)));
        assert_eq!(col.get(2), Some(&FieldValue::Numeric(-3.0)));
    }

    #[test]
    fn test_parse_json_nested_structure() {
        // 嵌套结构应该转为字符串
        let json = r#"[{"nested": {"a": 1}}, {"nested": [1, 2, 3]}]"#;
        let table = parse_json(json).unwrap();

        let col = table.get_column("nested").unwrap();
        assert_eq!(col.data_type, DataType::Nominal);
        assert!(matches!(col.get(0), Some(FieldValue::Text(_))));
        assert!(matches!(col.get(1), Some(FieldValue::Text(_))));
    }
}
