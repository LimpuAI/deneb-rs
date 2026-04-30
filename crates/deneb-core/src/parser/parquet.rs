//! Parquet 解析器
//!
//! 将 Parquet 格式的字节数据解析为 DataTable。

use crate::data::{Column, DataTable};
use crate::error::{CoreError, DataFormat};
use bytes::Bytes;

/// 解析 Parquet 格式的字节数据
///
/// # 参数
/// - `bytes`: Parquet 格式的字节数据
///
/// # 返回
/// 解析后的 DataTable
///
/// # 错误
/// - 如果数据格式无效或解析失败
#[cfg(feature = "parquet-format")]
pub fn parse_parquet(bytes: &[u8]) -> Result<DataTable, CoreError> {
    use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

    // 将 bytes 转换为 Bytes 类型
    let bytes = Bytes::copy_from_slice(bytes);

    // 创建 Parquet reader
    let builder = ParquetRecordBatchReaderBuilder::try_new(bytes)
        .map_err(|e| CoreError::parse_error(format!("Failed to create Parquet reader: {}", e), DataFormat::Parquet))?;

    let schema = builder.schema().clone();

    // 检查是否有数据
    let metadata = builder.metadata();
    if metadata.file_metadata().num_rows() == 0 {
        return Err(CoreError::empty_data());
    }

    let mut reader = builder
        .build()
        .map_err(|e| CoreError::parse_error(format!("Failed to build Parquet reader: {}", e), DataFormat::Parquet))?;

    let mut all_columns: Vec<Column> = Vec::new();

    // 遍历所有 RecordBatch
    while let Some(batch_result) = reader.next() {
        let batch = batch_result
            .map_err(|e| CoreError::parse_error(format!("Failed to read RecordBatch: {}", e), DataFormat::Parquet))?;

        // 处理每一列
        for (i, field) in schema.fields().iter().enumerate() {
            let column_name = field.name().clone();
            let array = batch.column(i).as_ref();

            // 如果是第一批数据，创建新列
            if all_columns.iter().all(|c| c.name != column_name) {
                let data_type = super::arrow::arrow_type_to_data_type(field.data_type());
                all_columns.push(Column::empty(column_name.clone(), data_type));
            }

            // 找到对应的列并添加数据
            if let Some(column) = all_columns.iter_mut().find(|c| c.name == column_name) {
                let values = super::arrow::arrow_array_to_values(array);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{DataType, FieldValue};
    use arrow::array::{Int32Array, StringArray, Float64Array};
    use arrow::datatypes::{Schema, Field, DataType as ArrowDataType};
    use arrow::record_batch::RecordBatch;
    use parquet::arrow::arrow_writer::ArrowWriter;
    use std::sync::Arc;

    #[test]
    fn test_parse_simple_parquet() {
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

        // 写入 Parquet 格式
        let mut buffer = Vec::new();
        {
            let mut writer = ArrowWriter::try_new(&mut buffer, batch.schema(), None)
                .unwrap();
            writer.write(&batch).unwrap();
            writer.close().unwrap();
        }

        // 解析
        let table = parse_parquet(&buffer).unwrap();

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

        // 写入 Parquet 格式
        let mut buffer = Vec::new();
        {
            let mut writer = ArrowWriter::try_new(&mut buffer, batch.schema(), None)
                .unwrap();
            writer.write(&batch).unwrap();
            writer.close().unwrap();
        }

        // 解析
        let table = parse_parquet(&buffer).unwrap();

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

        // 写入 Parquet 格式
        let mut buffer = Vec::new();
        {
            let mut writer = ArrowWriter::try_new(&mut buffer, batch.schema(), None)
                .unwrap();
            writer.write(&batch).unwrap();
            writer.close().unwrap();
        }

        // 解析应该返回空数据错误
        let result = parse_parquet(&buffer);
        assert!(matches!(result, Err(CoreError::EmptyData)));
    }

    #[test]
    fn test_parse_invalid_data() {
        // 无效数据
        let invalid_data = b"not parquet data";

        let result = parse_parquet(invalid_data);
        assert!(matches!(result, Err(CoreError::ParseError { .. })));
    }

    #[test]
    fn test_parse_compressed_data() {
        // 测试 UNCOMPRESSED 格式（确保基本功能正常）
        // 注意：某些压缩格式可能在编译时被禁用，这里使用 UNCOMPRESSED 确保测试可移植
        use parquet::file::properties::WriterProperties;
        use parquet::basic::Compression;

        let schema = Schema::new(vec![
            Field::new("x", ArrowDataType::Int32, false),
            Field::new("y", ArrowDataType::Float64, false),
        ]);

        let x = Int32Array::from(vec![1, 2, 3, 4, 5]);
        let y = Float64Array::from(vec![10.0, 20.0, 30.0, 40.0, 50.0]);

        let batch = RecordBatch::try_new(Arc::new(schema), vec![
            Arc::new(x),
            Arc::new(y),
        ]).unwrap();

        // 使用 UNCOMPRESSED 格式写入 Parquet
        let mut buffer = Vec::new();
        {
            let props = WriterProperties::builder()
                .set_compression(Compression::UNCOMPRESSED)
                .build();

            let mut writer = ArrowWriter::try_new(&mut buffer, batch.schema(), Some(props))
                .unwrap();
            writer.write(&batch).unwrap();
            writer.close().unwrap();
        }

        // 解析
        let table = parse_parquet(&buffer).unwrap();

        // 验证
        assert_eq!(table.column_count(), 2);
        assert_eq!(table.row_count(), 5);

        let x_col = table.get_column("x").unwrap();
        assert_eq!(x_col.data_type, DataType::Quantitative);
        assert_eq!(x_col.len(), 5);
    }

    #[test]
    fn test_parse_timestamp_parquet() {
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

        // 写入 Parquet 格式
        let mut buffer = Vec::new();
        {
            let mut writer = ArrowWriter::try_new(&mut buffer, batch.schema(), None)
                .unwrap();
            writer.write(&batch).unwrap();
            writer.close().unwrap();
        }

        // 解析
        let table = parse_parquet(&buffer).unwrap();

        // 验证时间戳转换为秒
        let ts_col = table.get_column("timestamp").unwrap();
        assert_eq!(ts_col.data_type, DataType::Temporal);
        assert_eq!(ts_col.get(0), Some(&FieldValue::Timestamp(1_000_000.0))); // 毫秒转秒
    }
}
