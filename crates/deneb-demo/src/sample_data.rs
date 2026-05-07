//! 演示数据
//!
//! 为各种图表类型提供 CSV 和 Parquet 格式的测试数据。

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

/// 饼图演示数据（分类 + 数值，CSV）
pub fn pie_chart_csv() -> &'static str {
    "category,value\nElectronics,450\nClothing,320\nFood,280\nBooks,190\nSports,350\nMusic,210"
}

/// 直方图演示数据（数值，CSV）
pub fn histogram_chart_csv() -> &'static str {
    "value\n12\n15\n18\n22\n25\n28\n30\n32\n35\n38\n40\n42\n45\n48\n50\n52\n55\n58\n60\n65\n68\n70\n72\n75\n78\n80\n82\n85\n88\n90"
}

/// 箱线图演示数据（分组 + 数值，CSV）
pub fn boxplot_chart_csv() -> &'static str {
    "group,value\nA,12\nA,15\nA,18\nA,22\nA,25\nA,28\nA,35\nA,38\nA,42\nA,48\nB,20\nB,25\nB,30\nB,35\nB,40\nB,45\nB,50\nB,55\nB,60\nB,70"
}

/// 瀑布图演示数据（分类 + 数值，CSV）
pub fn waterfall_chart_csv() -> &'static str {
    "category,value\nStart,100\nRevenue,50\nCosts,-30\nProfit,20\nTax,-10\nEnd,130"
}

/// K 线图演示数据（日期 + OHLC，CSV）
pub fn candlestick_chart_csv() -> &'static str {
    "date,open,high,low,close\n2024-01,100,120,90,110\n2024-02,110,130,100,125\n2024-03,125,140,115,130\n2024-04,130,135,110,115\n2024-05,115,125,105,120\n2024-06,120,145,115,140"
}

/// 雷达图演示数据（维度 + 数值，CSV）
pub fn radar_chart_csv() -> &'static str {
    "dimension,value\nSpeed,80\nPower,65\nRange,90\nArmor,70\nStealth,85\nAgility,75"
}

/// 热力图演示数据（x + y + 数值，CSV）
pub fn heatmap_chart_csv() -> &'static str {
    "x,y,value\nA,Y1,10\nA,Y2,20\nA,Y3,30\nB,Y1,25\nB,Y2,35\nB,Y3,15\nC,Y1,40\nC,Y2,10\nC,Y3,50"
}

/// 条带图演示数据（分类 + 数值，CSV）
pub fn strip_chart_csv() -> &'static str {
    "category,value\nA,12\nA,18\nA,25\nA,30\nA,35\nA,42\nB,20\nB,28\nB,35\nB,40\nB,48\nB,55\nC,15\nC,22\nC,30\nC,38\nC,45\nC,52"
}

/// 桑基图演示数据（source + target + flow，CSV）
pub fn sankey_chart_csv() -> &'static str {
    "source,target,flow\nCoal,Power,100\nGas,Power,50\nPower,Industry,80\nPower,Homes,70\nSolar,Homes,20\nWind,Industry,30"
}

/// 和弦图演示数据（source + target + flow，CSV）
pub fn chord_chart_csv() -> &'static str {
    "source,target,flow\nA,B,30\nA,C,20\nB,A,25\nB,C,35\nC,A,15\nC,B,40"
}

/// 等高线图演示数据（x + y + 数值，CSV）
pub fn contour_chart_csv() -> &'static str {
    "x,y,value\n1,1,10\n1,2,25\n1,3,15\n2,1,30\n2,2,50\n2,3,35\n3,1,20\n3,2,40\n3,3,25\n4,1,15\n4,2,30\n4,3,20"
}
