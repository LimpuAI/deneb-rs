//! 演示数据
//!
//! 为 4 种图表类型提供 CSV 和 Parquet 格式的测试数据。

/// 折线图演示数据（时间序列，CSV）
pub fn line_chart_csv() -> &'static str {
    "x,y\n0,10\n1,25\n2,18\n3,32\n4,28\n5,45\n6,38\n7,52\n8,48\n9,55\n10,42\n11,60\n12,35\n13,50\n14,65\n15,58\n16,70\n17,45\n18,62\n19,75"
}

/// 柱状图演示数据（分类数据，CSV）
pub fn bar_chart_csv() -> &'static str {
    "category,value\nElectronics,450\nClothing,320\nFood,280\nBooks,190\nSports,350\nMusic,210"
}

/// 散点图演示数据（两组聚类，Parquet）
pub fn scatter_chart_parquet() -> Vec<u8> {
    use arrow::array::{Float64Array, StringArray};
    use arrow::datatypes::{DataType, Field, Schema};
    use arrow::record_batch::RecordBatch;
    use parquet::arrow::arrow_writer::ArrowWriter;

    let schema = Schema::new(vec![
        Field::new("x", DataType::Float64, false),
        Field::new("y", DataType::Float64, false),
        Field::new("group", DataType::Utf8, false),
    ]);

    let x = Float64Array::from(vec![
        1.2, 1.8, 2.1, 1.5, 2.5, 1.9, 2.3, 1.7,
        5.5, 6.1, 5.8, 6.5, 5.2, 6.8, 5.9, 6.3,
        3.5, 3.8, 2.8, 4.2,
    ]);
    let y = Float64Array::from(vec![
        3.4, 4.1, 3.8, 4.5, 3.2, 3.9, 4.3, 3.6,
        7.2, 6.8, 7.5, 7.1, 6.5, 7.8, 6.9, 7.4,
        5.2, 5.8, 4.2, 6.1,
    ]);
    let group = StringArray::from(vec![
        "A", "A", "A", "A", "A", "A", "A", "A",
        "B", "B", "B", "B", "B", "B", "B", "B",
        "A", "B", "A", "B",
    ]);

    let batch = RecordBatch::try_new(
        std::sync::Arc::new(schema),
        vec![std::sync::Arc::new(x), std::sync::Arc::new(y), std::sync::Arc::new(group)],
    ).unwrap();

    let mut buf = Vec::new();
    let mut writer = ArrowWriter::try_new(&mut buf, batch.schema().clone(), None).unwrap();
    writer.write(&batch).unwrap();
    writer.close().unwrap();
    buf
}

/// 面积图演示数据（2 系列，Parquet）
pub fn area_chart_parquet() -> Vec<u8> {
    use arrow::array::Int64Array;
    use arrow::datatypes::{DataType, Field, Schema};
    use arrow::record_batch::RecordBatch;
    use parquet::arrow::arrow_writer::ArrowWriter;

    let schema = Schema::new(vec![
        Field::new("x", DataType::Int64, false),
        Field::new("y1", DataType::Int64, false),
        Field::new("y2", DataType::Int64, false),
    ]);

    let x = Int64Array::from(vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);
    let y1 = Int64Array::from(vec![20, 35, 28, 45, 38, 55, 48, 65, 58, 72, 62, 78]);
    let y2 = Int64Array::from(vec![10, 15, 22, 30, 25, 35, 40, 45, 38, 50, 42, 55]);

    let batch = RecordBatch::try_new(
        std::sync::Arc::new(schema),
        vec![std::sync::Arc::new(x), std::sync::Arc::new(y1), std::sync::Arc::new(y2)],
    ).unwrap();

    let mut buf = Vec::new();
    let mut writer = ArrowWriter::try_new(&mut buf, batch.schema().clone(), None).unwrap();
    writer.write(&batch).unwrap();
    writer.close().unwrap();
    buf
}
