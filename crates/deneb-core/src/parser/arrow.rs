//! Arrow IPC 解析器
//!
//! 将 Arrow IPC 格式的字节数据解析为 DataTable。

use crate::data::{Column, DataTable, DataType, FieldValue};
use crate::error::{CoreError, DataFormat};

/// 解析 Arrow IPC 格式的字节数据
///
/// # 参数
/// - `bytes`: Arrow IPC 格式的字节数据
///
/// # 返回
/// 解析后的 DataTable
///
/// # 错误
/// - 如果数据格式无效或解析失败
#[cfg(feature = "arrow-format")]
pub fn parse_arrow_ipc(bytes: &[u8]) -> Result<DataTable, CoreError> {
    use arrow::ipc::reader::StreamReader;

    // 创建 StreamReader
    let mut reader = StreamReader::try_new(std::io::Cursor::new(bytes), None)
        .map_err(|e| CoreError::parse_error(format!("Failed to create Arrow IPC reader: {}", e), DataFormat::Arrow))?;

    // 检查是否有数据
    let empty = reader.next().is_none();

    // 重新创建 reader
    let mut reader = StreamReader::try_new(std::io::Cursor::new(bytes), None)
        .map_err(|e| CoreError::parse_error(format!("Failed to create Arrow IPC reader: {}", e), DataFormat::Arrow))?;

    if empty {
        return Err(CoreError::empty_data());
    }

    let mut all_columns: Vec<Column> = Vec::new();

    // 遍历所有 RecordBatch
    while let Some(batch_result) = reader.next() {
        let batch = batch_result
            .map_err(|e| CoreError::parse_error(format!("Failed to read RecordBatch: {}", e), DataFormat::Arrow))?;

        let schema = batch.schema();

        // 处理每一列
        for (i, field) in schema.fields().iter().enumerate() {
            let column_name = field.name().clone();
            let array = batch.column(i).as_ref();

            // 如果是第一批数据，创建新列
            if all_columns.iter().all(|c| c.name != column_name) {
                let data_type = arrow_type_to_data_type(field.data_type());
                all_columns.push(Column::empty(column_name.clone(), data_type));
            }

            // 找到对应的列并添加数据
            if let Some(column) = all_columns.iter_mut().find(|c| c.name == column_name) {
                let values = arrow_array_to_values(array);
                column.extend(values);
            }
        }
    }

    // 验证数据一致性
    let table = DataTable::with_columns(all_columns);
    table.validate()?;

    // 检查是否为空
    if table.is_empty() {
        return Err(CoreError::empty_data());
    }

    Ok(table)
}

/// 将 Arrow 数组转换为 FieldValue 序列
///
/// 此函数为 pub(crate)，以便 parquet 解析器可以复用
#[cfg(feature = "arrow-format")]
pub(crate) fn arrow_array_to_values(array: &dyn arrow::array::Array) -> Vec<FieldValue> {
    use arrow::array::*;
    use arrow::datatypes::DataType as ArrowDataType;

    let len = array.len();
    let mut values = Vec::with_capacity(len);

    let data_type = array.data_type();

    for i in 0..len {
        if array.is_null(i) {
            values.push(FieldValue::Null);
            continue;
        }

        let value = match data_type {
            // 整数类型
            ArrowDataType::Int8 => {
                let array = array.as_any().downcast_ref::<Int8Array>().unwrap();
                FieldValue::Numeric(array.value(i) as f64)
            }
            ArrowDataType::Int16 => {
                let array = array.as_any().downcast_ref::<Int16Array>().unwrap();
                FieldValue::Numeric(array.value(i) as f64)
            }
            ArrowDataType::Int32 => {
                let array = array.as_any().downcast_ref::<Int32Array>().unwrap();
                FieldValue::Numeric(array.value(i) as f64)
            }
            ArrowDataType::Int64 => {
                let array = array.as_any().downcast_ref::<Int64Array>().unwrap();
                FieldValue::Numeric(array.value(i) as f64)
            }
            ArrowDataType::UInt8 => {
                let array = array.as_any().downcast_ref::<UInt8Array>().unwrap();
                FieldValue::Numeric(array.value(i) as f64)
            }
            ArrowDataType::UInt16 => {
                let array = array.as_any().downcast_ref::<UInt16Array>().unwrap();
                FieldValue::Numeric(array.value(i) as f64)
            }
            ArrowDataType::UInt32 => {
                let array = array.as_any().downcast_ref::<UInt32Array>().unwrap();
                FieldValue::Numeric(array.value(i) as f64)
            }
            ArrowDataType::UInt64 => {
                let array = array.as_any().downcast_ref::<UInt64Array>().unwrap();
                FieldValue::Numeric(array.value(i) as f64)
            }

            // 浮点类型
            ArrowDataType::Float16 => {
                let array = array.as_any().downcast_ref::<Float16Array>().unwrap();
                FieldValue::Numeric(array.value(i).to_f64())
            }
            ArrowDataType::Float32 => {
                let array = array.as_any().downcast_ref::<Float32Array>().unwrap();
                FieldValue::Numeric(array.value(i) as f64)
            }
            ArrowDataType::Float64 => {
                let array = array.as_any().downcast_ref::<Float64Array>().unwrap();
                FieldValue::Numeric(array.value(i))
            }

            // 字符串类型
            ArrowDataType::Utf8 => {
                let array = array.as_any().downcast_ref::<StringArray>().unwrap();
                FieldValue::Text(array.value(i).to_string())
            }
            ArrowDataType::LargeUtf8 => {
                let array = array.as_any().downcast_ref::<LargeStringArray>().unwrap();
                FieldValue::Text(array.value(i).to_string())
            }

            // 时间戳类型
            ArrowDataType::Timestamp(_, _) => {
                let array = array.as_any().downcast_ref::<TimestampMillisecondArray>().unwrap();
                // 转换为 unix epoch seconds
                FieldValue::Timestamp(array.value(i) as f64 / 1000.0)
            }

            // 日期类型
            ArrowDataType::Date32 => {
                let array = array.as_any().downcast_ref::<Date32Array>().unwrap();
                // Date32 是从 epoch 1970-01-01 开始的天数
                FieldValue::Timestamp(array.value(i) as f64 * 86400.0)
            }
            ArrowDataType::Date64 => {
                let array = array.as_any().downcast_ref::<Date64Array>().unwrap();
                // Date64 是从 epoch 1970-01-01 开始的毫秒数
                FieldValue::Timestamp(array.value(i) as f64 / 1000.0)
            }

            // 布尔类型
            ArrowDataType::Boolean => {
                let array = array.as_any().downcast_ref::<BooleanArray>().unwrap();
                FieldValue::Bool(array.value(i))
            }

            // Null 类型
            ArrowDataType::Null => {
                FieldValue::Null
            }

            // 其他类型转为文本
            _ => {
                FieldValue::Text(format!("{:?}", array))
            }
        };

        values.push(value);
    }

    values
}

/// 将 Arrow 数据类型映射为 DataType
///
/// 此函数为 pub(crate)，以便 parquet 解析器可以复用
#[cfg(feature = "arrow-format")]
pub(crate) fn arrow_type_to_data_type(dt: &arrow::datatypes::DataType) -> DataType {
    use arrow::datatypes::DataType as ArrowDataType;

    match dt {
        // 整数和浮点类型 -> Quantitative
        ArrowDataType::Int8 | ArrowDataType::Int16 | ArrowDataType::Int32 | ArrowDataType::Int64 |
        ArrowDataType::UInt8 | ArrowDataType::UInt16 | ArrowDataType::UInt32 | ArrowDataType::UInt64 |
        ArrowDataType::Float16 | ArrowDataType::Float32 | ArrowDataType::Float64 => {
            DataType::Quantitative
        }

        // 字符串类型 -> Nominal
        ArrowDataType::Utf8 | ArrowDataType::LargeUtf8 | ArrowDataType::Binary | ArrowDataType::LargeBinary => {
            DataType::Nominal
        }

        // 时间戳和日期类型 -> Temporal
        ArrowDataType::Timestamp(_, _) | ArrowDataType::Date32 | ArrowDataType::Date64 |
        ArrowDataType::Time32(_) | ArrowDataType::Time64(_) | ArrowDataType::Duration(_) |
        ArrowDataType::Interval(_) => {
            DataType::Temporal
        }

        // 布尔类型 -> Nominal
        ArrowDataType::Boolean => DataType::Nominal,

        // Null 类型 -> Nominal
        ArrowDataType::Null => DataType::Nominal,

        // List 和 Struct 类型 -> Nominal
        ArrowDataType::List(_) | ArrowDataType::LargeList(_) | ArrowDataType::FixedSizeList(_, _) |
        ArrowDataType::Struct(_) => {
            DataType::Nominal
        }

        // Dictionary 类型 -> Nominal
        ArrowDataType::Dictionary(_, _) => DataType::Nominal,

        // 其他类型 -> Nominal
        _ => DataType::Nominal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{DataType, FieldValue};
    use arrow::array::{Int32Array, StringArray, Float64Array};
    use arrow::datatypes::{Schema, Field, DataType as ArrowDataType};
    use arrow::ipc::writer::StreamWriter;
    use arrow::record_batch::RecordBatch;
    use std::sync::Arc;

    #[test]
    fn test_parse_simple_ipc() {
        // 创建测试数据
        let schema = Schema::new(vec![
            Field::new("x", ArrowDataType::Int32, false),
            Field::new("y", ArrowDataType::Float64, false),
            Field::new("category", ArrowDataType::Utf8, false),
        ]);

        let x = Int32Array::from(vec![1, 2, 3, 4, 5]);
        let y = Float64Array::from(vec![10.0, 20.0, 30.0, 40.0, 50.0]);
        let category = StringArray::from(vec!["A", "B", "A", "B", "A"]);

        let batch = RecordBatch::try_new(Arc::new(schema), vec![
            Arc::new(x),
            Arc::new(y),
            Arc::new(category),
        ]).unwrap();

        // 写入 IPC 格式
        let mut buffer = Vec::new();
        {
            let mut writer = StreamWriter::try_new(&mut buffer, &batch.schema()).unwrap();
            writer.write(&batch).unwrap();
            writer.finish().unwrap();
        }

        // 解析
        let table = parse_arrow_ipc(&buffer).unwrap();

        // 验证
        assert_eq!(table.column_count(), 3);
        assert_eq!(table.row_count(), 5);

        let x_col = table.get_column("x").unwrap();
        assert_eq!(x_col.data_type, DataType::Quantitative);
        assert_eq!(x_col.len(), 5);
        assert_eq!(x_col.get(0), Some(&FieldValue::Numeric(1.0)));
        assert_eq!(x_col.get(4), Some(&FieldValue::Numeric(5.0)));

        let y_col = table.get_column("y").unwrap();
        assert_eq!(y_col.data_type, DataType::Quantitative);
        assert_eq!(y_col.get(0), Some(&FieldValue::Numeric(10.0)));

        let category_col = table.get_column("category").unwrap();
        assert_eq!(category_col.data_type, DataType::Nominal);
        assert_eq!(category_col.get(0), Some(&FieldValue::Text("A".to_string())));
    }

    #[test]
    fn test_parse_null_values() {
        // 创建包含 null 值的测试数据
        let schema = Schema::new(vec![
            Field::new("value", ArrowDataType::Int32, true),
        ]);

        let value = Int32Array::from(vec![Some(1), None, Some(3), None, Some(5)]);

        let batch = RecordBatch::try_new(Arc::new(schema), vec![
            Arc::new(value),
        ]).unwrap();

        // 写入 IPC 格式
        let mut buffer = Vec::new();
        {
            let mut writer = StreamWriter::try_new(&mut buffer, &batch.schema()).unwrap();
            writer.write(&batch).unwrap();
            writer.finish().unwrap();
        }

        // 解析
        let table = parse_arrow_ipc(&buffer).unwrap();

        // 验证 null 值
        let value_col = table.get_column("value").unwrap();
        assert_eq!(value_col.get(0), Some(&FieldValue::Numeric(1.0)));
        assert_eq!(value_col.get(1), Some(&FieldValue::Null));
        assert_eq!(value_col.get(2), Some(&FieldValue::Numeric(3.0)));
    }

    #[test]
    fn test_parse_empty_data() {
        // 创建空数据
        let schema = Schema::new(vec![
            Field::new("x", ArrowDataType::Int32, false),
        ]);

        let x = Int32Array::from(Vec::<i32>::new());

        let batch = RecordBatch::try_new(Arc::new(schema), vec![
            Arc::new(x),
        ]).unwrap();

        // 写入 IPC 格式
        let mut buffer = Vec::new();
        {
            let mut writer = StreamWriter::try_new(&mut buffer, &batch.schema()).unwrap();
            writer.write(&batch).unwrap();
            writer.finish().unwrap();
        }

        // 解析应该返回空数据错误
        let result = parse_arrow_ipc(&buffer);
        assert!(matches!(result, Err(CoreError::EmptyData)));
    }

    #[test]
    fn test_parse_invalid_data() {
        // 无效数据
        let invalid_data = b"not arrow data";

        let result = parse_arrow_ipc(invalid_data);
        assert!(matches!(result, Err(CoreError::ParseError { .. })));
    }

    #[test]
    fn test_arrow_type_to_data_type() {
        use arrow::datatypes::DataType as ArrowDataType;

        // 测试数值类型
        assert_eq!(
            arrow_type_to_data_type(&ArrowDataType::Int32),
            DataType::Quantitative
        );
        assert_eq!(
            arrow_type_to_data_type(&ArrowDataType::Float64),
            DataType::Quantitative
        );

        // 测试字符串类型
        assert_eq!(
            arrow_type_to_data_type(&ArrowDataType::Utf8),
            DataType::Nominal
        );

        // 测试时间类型
        assert_eq!(
            arrow_type_to_data_type(&ArrowDataType::Timestamp(arrow::datatypes::TimeUnit::Millisecond, None)),
            DataType::Temporal
        );
        assert_eq!(
            arrow_type_to_data_type(&ArrowDataType::Date32),
            DataType::Temporal
        );

        // 测试布尔类型
        assert_eq!(
            arrow_type_to_data_type(&ArrowDataType::Boolean),
            DataType::Nominal
        );
    }

    #[test]
    fn test_parse_timestamp() {
        use arrow::datatypes::TimeUnit;

        // 创建时间戳数据
        let schema = Schema::new(vec![
            Field::new("timestamp", ArrowDataType::Timestamp(TimeUnit::Millisecond, None), false),
        ]);

        let timestamp = arrow::array::TimestampMillisecondArray::from(vec![
            1_000_000_000,  // 1970-01-01 00:16:40
            1_500_000_000,  // 1970-01-18 11:40:00
        ]);

        let batch = RecordBatch::try_new(Arc::new(schema), vec![
            Arc::new(timestamp),
        ]).unwrap();

        // 写入 IPC 格式
        let mut buffer = Vec::new();
        {
            let mut writer = StreamWriter::try_new(&mut buffer, &batch.schema()).unwrap();
            writer.write(&batch).unwrap();
            writer.finish().unwrap();
        }

        // 解析
        let table = parse_arrow_ipc(&buffer).unwrap();

        // 验证时间戳转换为秒
        let ts_col = table.get_column("timestamp").unwrap();
        assert_eq!(ts_col.data_type, DataType::Temporal);
        assert_eq!(ts_col.get(0), Some(&FieldValue::Timestamp(1_000_000.0))); // 毫秒转秒
    }
}
