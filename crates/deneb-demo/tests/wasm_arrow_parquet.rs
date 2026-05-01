//! WASM 集成测试 — Arrow IPC 和 Parquet 格式解析

use arrow::array::Int64Array;
use arrow::datatypes::{DataType, Field, Schema};
use arrow::ipc::writer::StreamWriter;
use arrow::record_batch::RecordBatch;
use parquet::arrow::arrow_writer::ArrowWriter;

use deneb_demo::wasm_host::{ParserPaths, WasmHost};

/// 测试数据对应的 WASM 文件路径
const VIZ_WASM: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../target/wasm32-wasip2/release/deneb_wit_wasm.wasm"
);
const ARROW_WASM: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../limpuai-wit/target/wasm32-wasip2/release/limpuai_wit_arrow.wasm"
);
const PARQUET_WASM: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../limpuai-wit/target/wasm32-wasip2/release/limpuai_wit_parquet.wasm"
);

/// 创建带解析器的 WasmHost
fn create_host() -> WasmHost {
    WasmHost::from_file_with_parsers(
        VIZ_WASM,
        ParserPaths {
            arrow: Some(ARROW_WASM.to_string()),
            parquet: Some(PARQUET_WASM.to_string()),
        },
    ).expect("Failed to create WasmHost")
}

/// 创建 Arrow IPC 测试数据（x: [1,2,3], y: [10,20,30]）
fn make_arrow_ipc() -> Vec<u8> {
    let schema = Schema::new(vec![
        Field::new("x", DataType::Int64, false),
        Field::new("y", DataType::Int64, false),
    ]);
    let x = Int64Array::from(vec![1, 2, 3]);
    let y = Int64Array::from(vec![10, 20, 30]);
    let batch = RecordBatch::try_new(
        std::sync::Arc::new(schema),
        vec![std::sync::Arc::new(x), std::sync::Arc::new(y)],
    ).unwrap();

    let mut buf = Vec::new();
    {
        let mut writer = StreamWriter::try_new(&mut buf, &batch.schema()).unwrap();
        writer.write(&batch).unwrap();
        writer.finish().unwrap();
    }
    buf
}

/// 创建 Parquet 测试数据（x: [1,2,3], y: [10,20,30]）
fn make_parquet() -> Vec<u8> {
    let schema = Schema::new(vec![
        Field::new("x", DataType::Int64, false),
        Field::new("y", DataType::Int64, false),
    ]);
    let x = Int64Array::from(vec![1, 2, 3]);
    let y = Int64Array::from(vec![10, 20, 30]);
    let batch = RecordBatch::try_new(
        std::sync::Arc::new(schema),
        vec![std::sync::Arc::new(x), std::sync::Arc::new(y)],
    ).unwrap();

    let mut buf = Vec::new();
    {
        let mut writer = ArrowWriter::try_new(&mut buf, batch.schema().clone(), None).unwrap();
        writer.write(&batch).unwrap();
        writer.close().unwrap();
    }
    buf
}

#[test]
fn test_wasm_parse_arrow() {
    let mut host = create_host();
    let arrow_data = make_arrow_ipc();
    let table = host.parse_arrow(&arrow_data).expect("parse_arrow failed");

    assert_eq!(table.columns.len(), 2, "should have 2 columns");
    assert_eq!(table.rows.len(), 3, "should have 3 rows");
    assert_eq!(table.columns[0].name, "x");
    assert_eq!(table.columns[1].name, "y");
}

#[test]
fn test_wasm_parse_parquet() {
    let mut host = create_host();
    let parquet_data = make_parquet();
    let table = host.parse_parquet(&parquet_data).expect("parse_parquet failed");

    assert_eq!(table.columns.len(), 2, "should have 2 columns");
    assert_eq!(table.rows.len(), 3, "should have 3 rows");
    assert_eq!(table.columns[0].name, "x");
    assert_eq!(table.columns[1].name, "y");
}

#[test]
fn test_wasm_render_arrow() {
    use deneb_wit::wit_types::WitChartSpec;

    let mut host = create_host();
    let arrow_data = make_arrow_ipc();
    let spec = WitChartSpec {
        mark: "line".to_string(),
        x_field: "x".to_string(),
        y_field: "y".to_string(),
        color_field: None,
        width: 800.0,
        height: 600.0,
        title: Some("Arrow Test".to_string()),
        theme: None,
    };

    let result = host.render(&arrow_data, "arrow", &spec).expect("render arrow failed");
    assert!(!result.layers.is_empty(), "should have render layers");
}

#[test]
fn test_wasm_render_parquet() {
    use deneb_wit::wit_types::WitChartSpec;

    let mut host = create_host();
    let parquet_data = make_parquet();
    let spec = WitChartSpec {
        mark: "line".to_string(),
        x_field: "x".to_string(),
        y_field: "y".to_string(),
        color_field: None,
        width: 800.0,
        height: 600.0,
        title: Some("Parquet Test".to_string()),
        theme: None,
    };

    let result = host.render(&parquet_data, "parquet", &spec).expect("render parquet failed");
    assert!(!result.layers.is_empty(), "should have render layers");
}
