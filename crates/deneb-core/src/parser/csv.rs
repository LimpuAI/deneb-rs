//! CSV 解析器
//!
//! 解析 CSV 格式的数据为 DataTable，支持自定义分隔符、注释行、引号字段和转义。
//! 包含自动类型推断功能。

use crate::data::{DataTable, Column, FieldValue, DataType};
use crate::error::{CoreError, DataFormat};

/// 解析 CSV 字符串为 DataTable
///
/// # Arguments
///
/// * `input` - CSV 格式的字符串
///
/// # Returns
///
/// 返回解析后的 DataTable，如果解析失败则返回错误
///
/// # Examples
///
/// ```no_run
/// use deneb_core::parser::csv::parse_csv;
/// let csv = "x,y\n1,2\n3,4";
/// let table = parse_csv(csv).unwrap();
/// ```
#[cfg(feature = "csv")]
pub fn parse_csv(input: &str) -> Result<DataTable, CoreError> {
    parse_csv_with_delimiter(input, ',')
}

/// 使用自定义分隔符解析 CSV 字符串为 DataTable
///
/// # Arguments
///
/// * `input` - CSV 格式的字符串
/// * `delimiter` - 字段分隔符（默认逗号）
///
/// # Returns
///
/// 返回解析后的 DataTable，如果解析失败则返回错误
///
/// # Examples
///
/// ```no_run
/// use deneb_core::parser::csv::parse_csv_with_delimiter;
/// let csv = "x;y\n1;2\n3;4";
/// let table = parse_csv_with_delimiter(csv, ';').unwrap();
/// ```
#[cfg(feature = "csv")]
pub fn parse_csv_with_delimiter(input: &str, delimiter: char) -> Result<DataTable, CoreError> {
    let input = input.trim();
    if input.is_empty() {
        return Err(CoreError::empty_data());
    }

    let lines: Vec<&str> = input.lines().collect();
    if lines.is_empty() {
        return Err(CoreError::empty_data());
    }

    // 找到第一个非注释行作为 header
    let header_line = lines
        .iter()
        .find(|line| !line.trim().starts_with('#'))
        .ok_or_else(|| CoreError::parse_error("no header row found", DataFormat::Csv))?;

    let headers = parse_csv_line(header_line, delimiter)?;

    if headers.is_empty() {
        return Err(CoreError::parse_error("empty header row", DataFormat::Csv));
    }

    // 第一遍：收集所有列的值
    let mut column_values: Vec<Vec<String>> = vec![Vec::new(); headers.len()];

    let mut data_started = false;
    for line in &lines {
        let line = line.trim();

        // 跳过注释行和空行
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // 跳过 header 行
        if !data_started {
            data_started = true;
            continue;
        }

        // 解析数据行
        let fields = parse_csv_line(line, delimiter)?;

        // 填充列值，不足的用空字符串填充
        for (i, col) in column_values.iter_mut().enumerate() {
            if i < fields.len() {
                col.push(fields[i].to_string());
            } else {
                col.push(String::new());
            }
        }
    }

    // 第二遍：推断类型并构建列
    let mut table = DataTable::new();

    for (i, header) in headers.iter().enumerate() {
        let values = &column_values[i];
        let data_type = infer_type(values);

        let field_values: Vec<FieldValue> = values
            .iter()
            .map(|v| parse_field_value(v, &data_type))
            .collect();

        let column = Column::new(header.clone(), data_type, field_values);
        table.add_column(column);
    }

    Ok(table)
}

/// 解析 CSV 行，处理引号和转义
fn parse_csv_line(line: &str, delimiter: char) -> Result<Vec<String>, CoreError> {
    let mut fields = Vec::new();
    let mut current_field = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '"' => {
                if in_quotes {
                    // 检查是否是转义的引号 ("")
                    if chars.peek() == Some(&'"') {
                        current_field.push('"');
                        chars.next();
                    } else {
                        in_quotes = false;
                    }
                } else {
                    in_quotes = true;
                }
            }
            c if c == delimiter && !in_quotes => {
                fields.push(current_field.trim().to_string());
                current_field = String::new();
            }
            c => {
                current_field.push(c);
            }
        }
    }

    // 添加最后一个字段
    fields.push(current_field.trim().to_string());

    Ok(fields)
}

/// 推断列的数据类型
///
/// # Arguments
///
/// * `values` - 列中的所有字符串值
///
/// # Returns
///
/// 推断出的数据类型，优先级：Numeric > Timestamp > Bool > Text > Null
fn infer_type(values: &[String]) -> DataType {
    if values.is_empty() {
        return DataType::Nominal;
    }

    let mut type_votes: std::collections::HashMap<DataType, usize> = std::collections::HashMap::new();

    for value in values {
        let trimmed = value.trim();

        if trimmed.is_empty() {
            // 空字符串不算投票
            continue;
        }

        let detected_type = detect_value_type(trimmed);
        *type_votes.entry(detected_type).or_insert(0) += 1;
    }

    if type_votes.is_empty() {
        // 全是空字符串
        return DataType::Nominal;
    }

    // 按优先级排序：Numeric > Timestamp > Bool > Text
    let priority = [
        DataType::Quantitative,
        DataType::Temporal,
        DataType::Nominal, // Bool 映射到 Nominal
        DataType::Ordinal,
    ];

    for dtype in &priority {
        if type_votes.contains_key(dtype) {
            return *dtype;
        }
    }

    DataType::Nominal
}

/// 检测单个值的数据类型
fn detect_value_type(value: &str) -> DataType {
    // 检查布尔值
    if matches!(value.to_lowercase().as_str(), "true" | "false" | "yes" | "no") {
        return DataType::Nominal; // 布尔值归为 Nominal
    }

    // 检查时间戳（ISO 8601 格式）
    if is_timestamp(value) {
        return DataType::Temporal;
    }

    // 检查数值
    if value.parse::<f64>().is_ok() {
        return DataType::Quantitative;
    }

    // 默认为文本
    DataType::Nominal
}

/// 检查字符串是否为时间戳格式
fn is_timestamp(s: &str) -> bool {
    // 尝试解析 ISO 8601 格式
    // 支持的格式：YYYY-MM-DD, YYYY-MM-DDTHH:MM:SS, YYYY-MM-DDTHH:MM:SSZ, 等

    // 基本的日期格式检查
    if s.len() >= 10 && s.chars().nth(4) == Some('-') && s.chars().nth(7) == Some('-') {
        // 尝试用 chrono 解析
        if chrono::DateTime::parse_from_rfc3339(s).is_ok() {
            return true;
        }

        // 尝试解析日期部分
        if chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").is_ok() {
            return true;
        }
    }

    false
}

/// 根据推断的类型解析字段值
fn parse_field_value(value: &str, data_type: &DataType) -> FieldValue {
    let trimmed = value.trim();

    if trimmed.is_empty() {
        return FieldValue::Null;
    }

    match data_type {
        DataType::Quantitative => {
            trimmed
                .parse::<f64>()
                .map(FieldValue::Numeric)
                .unwrap_or(FieldValue::Text(trimmed.to_string()))
        }
        DataType::Temporal => {
            // 尝试解析时间戳
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(trimmed) {
                FieldValue::Timestamp(dt.timestamp() as f64)
            } else if let Ok(date) = chrono::NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
                FieldValue::Timestamp(date.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp() as f64)
            } else {
                FieldValue::Text(trimmed.to_string())
            }
        }
        DataType::Nominal | DataType::Ordinal => {
            // 检查布尔值
            match trimmed.to_lowercase().as_str() {
                "true" | "yes" => FieldValue::Bool(true),
                "false" | "no" => FieldValue::Bool(false),
                _ => FieldValue::Text(trimmed.to_string()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_csv_basic() {
        let csv = "x,y\n1,2\n3,4";
        let table = parse_csv(csv).unwrap();

        assert_eq!(table.column_count(), 2);
        assert_eq!(table.row_count(), 2);

        let x_col = table.get_column("x").unwrap();
        assert_eq!(x_col.len(), 2);
        assert_eq!(x_col.get(0), Some(&FieldValue::Numeric(1.0)));
        assert_eq!(x_col.get(1), Some(&FieldValue::Numeric(3.0)));
    }

    #[test]
    fn test_parse_csv_with_delimiter() {
        let csv = "x;y\n1;2\n3;4";
        let table = parse_csv_with_delimiter(csv, ';').unwrap();

        assert_eq!(table.column_count(), 2);
        assert_eq!(table.row_count(), 2);
    }

    #[test]
    fn test_parse_csv_with_comments() {
        let csv = "# This is a comment\nx,y\n# Another comment\n1,2\n3,4";
        let table = parse_csv(csv).unwrap();

        assert_eq!(table.column_count(), 2);
        assert_eq!(table.row_count(), 2);
    }

    #[test]
    fn test_parse_csv_with_quotes() {
        let csv = "name,value\n\"hello, world\",42\n\"test \"\"quoted\"\"\",100";
        let table = parse_csv(csv).unwrap();

        assert_eq!(table.column_count(), 2);
        assert_eq!(table.row_count(), 2);

        let name_col = table.get_column("name").unwrap();
        assert_eq!(name_col.get(0), Some(&FieldValue::Text("hello, world".to_string())));
        assert_eq!(name_col.get(1), Some(&FieldValue::Text("test \"quoted\"".to_string())));
    }

    #[test]
    fn test_parse_csv_type_inference() {
        let csv = "number,date,text,bool\n1,2023-01-01,hello,true\n2,2023-01-02,world,false";
        let table = parse_csv(csv).unwrap();

        let number_col = table.get_column("number").unwrap();
        assert_eq!(number_col.data_type, DataType::Quantitative);

        let date_col = table.get_column("date").unwrap();
        assert_eq!(date_col.data_type, DataType::Temporal);

        let text_col = table.get_column("text").unwrap();
        assert_eq!(text_col.data_type, DataType::Nominal);

        let bool_col = table.get_column("bool").unwrap();
        assert_eq!(bool_col.data_type, DataType::Nominal);
        assert_eq!(bool_col.get(0), Some(&FieldValue::Bool(true)));
        assert_eq!(bool_col.get(1), Some(&FieldValue::Bool(false)));
    }

    #[test]
    fn test_parse_csv_empty_values() {
        let csv = "x,y\n1,\n,4\n3,";
        let table = parse_csv(csv).unwrap();

        assert_eq!(table.column_count(), 2);
        assert_eq!(table.row_count(), 3);

        let x_col = table.get_column("x").unwrap();
        assert_eq!(x_col.get(0), Some(&FieldValue::Numeric(1.0)));
        assert_eq!(x_col.get(1), Some(&FieldValue::Null));
        assert_eq!(x_col.get(2), Some(&FieldValue::Numeric(3.0)));

        let y_col = table.get_column("y").unwrap();
        assert_eq!(y_col.get(0), Some(&FieldValue::Null));
        assert_eq!(y_col.get(1), Some(&FieldValue::Numeric(4.0)));
        assert_eq!(y_col.get(2), Some(&FieldValue::Null));
    }

    #[test]
    fn test_parse_csv_empty_input() {
        let csv = "";
        let result = parse_csv(csv);
        assert!(matches!(result, Err(CoreError::EmptyData)));
    }

    #[test]
    fn test_parse_csv_only_header() {
        let csv = "x,y,z";
        let table = parse_csv(csv).unwrap();

        assert_eq!(table.column_count(), 3);
        assert_eq!(table.row_count(), 0);
    }

    #[test]
    fn test_parse_csv_inconsistent_row_length() {
        let csv = "x,y\n1,2,3\n4";
        let table = parse_csv(csv).unwrap();

        assert_eq!(table.column_count(), 2);
        assert_eq!(table.row_count(), 2);

        let x_col = table.get_column("x").unwrap();
        assert_eq!(x_col.get(0), Some(&FieldValue::Numeric(1.0)));
        assert_eq!(x_col.get(1), Some(&FieldValue::Numeric(4.0)));

        let y_col = table.get_column("y").unwrap();
        assert_eq!(y_col.get(0), Some(&FieldValue::Numeric(2.0)));
        assert_eq!(y_col.get(1), Some(&FieldValue::Null));
    }

    #[test]
    fn test_infer_type() {
        // 数值优先
        let values = vec!["1".to_string(), "2.5".to_string(), "text".to_string()];
        assert_eq!(infer_type(&values), DataType::Quantitative);

        // 时间戳优先
        let values = vec!["2023-01-01".to_string(), "2023-01-02".to_string()];
        assert_eq!(infer_type(&values), DataType::Temporal);

        // 布尔值
        let values = vec!["true".to_string(), "false".to_string()];
        assert_eq!(infer_type(&values), DataType::Nominal);

        // 文本
        let values = vec!["hello".to_string(), "world".to_string()];
        assert_eq!(infer_type(&values), DataType::Nominal);

        // 空值
        let values: Vec<String> = vec![];
        assert_eq!(infer_type(&values), DataType::Nominal);

        // 全空字符串
        let values = vec!["".to_string(), "".to_string()];
        assert_eq!(infer_type(&values), DataType::Nominal);
    }

    #[test]
    fn test_is_timestamp() {
        assert!(is_timestamp("2023-01-01"));
        // 2023-01-01T12:00:00 格式没有时区，不视为有效时间戳
        assert!(is_timestamp("2023-01-01T12:00:00Z"));
        assert!(is_timestamp("2023-01-01T12:00:00+08:00"));
        assert!(!is_timestamp("not a date"));
        assert!(!is_timestamp("12345"));
        assert!(!is_timestamp("2023-01-01T12:00:00")); // 无时区信息
    }

    #[test]
    fn test_parse_csv_line() {
        let line = "hello,world,test";
        let fields = parse_csv_line(line, ',').unwrap();
        assert_eq!(fields, vec!["hello", "world", "test"]);

        let line = "\"hello, world\",test,42";
        let fields = parse_csv_line(line, ',').unwrap();
        assert_eq!(fields, vec!["hello, world", "test", "42"]);

        let line = "\"test \"\"quoted\"\"\",value";
        let fields = parse_csv_line(line, ',').unwrap();
        assert_eq!(fields, vec!["test \"quoted\"", "value"]);
    }

    #[test]
    fn test_parse_csv_mixed_types() {
        let csv = "value\n1\n2.5\ntext\n2023-01-01";
        let table = parse_csv(csv).unwrap();

        let col = table.get_column("value").unwrap();
        // 应该推断为 Quantitative（因为优先级最高且有数值）
        assert_eq!(col.data_type, DataType::Quantitative);
    }

    #[test]
    fn test_parse_csv_timestamp_iso8601() {
        let csv = "date\n2023-01-01\n2023-01-01T12:00:00\n2023-01-01T12:00:00Z";
        let table = parse_csv(csv).unwrap();

        let col = table.get_column("date").unwrap();
        assert_eq!(col.data_type, DataType::Temporal);

        // 验证时间戳值
        if let Some(FieldValue::Timestamp(ts)) = col.get(0) {
            assert!(*ts > 0.0);
        } else {
            panic!("Expected Timestamp value");
        }
    }
}
