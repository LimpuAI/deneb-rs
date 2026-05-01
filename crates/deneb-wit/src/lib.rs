//! deneb-wit: WASI 0.3 集成层
//!
//! 提供两种使用模式：
//! 1. 库调用模式：宿主直接调用 Rust API
//! 2. 独立组件模式：通过 WIT 接口作为 WASI 组件运行

#![warn(clippy::all)]
#![allow(missing_docs)]  // WIT type fields are self-documenting

pub use deneb_core;
pub use deneb_component;


/// WIT 类型定义 — 与 world.wit 中的 record 一一对应
pub mod wit_types {

    /// WIT 字段模式定义
    #[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
    pub struct WitSchemaField {
        pub name: String,
        pub data_type: String,
    }

    /// WIT 数据表定义（行式存储）
    #[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
    pub struct WitDataTable {
        pub columns: Vec<WitSchemaField>,
        pub rows: Vec<Vec<WitFieldValue>>,
    }

    /// WIT 字段值（支持多种类型）
    #[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
    pub enum WitFieldValue {
        Numeric(f64),
        Text(String),
        Timestamp(f64),
        Boolean(bool),
        Null,
    }

    /// WIT 图表规格定义
    #[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
    pub struct WitChartSpec {
        pub mark: String,
        pub x_field: String,
        pub y_field: String,
        pub color_field: Option<String>,
        pub width: f64,
        pub height: f64,
        pub title: Option<String>,
        pub theme: Option<String>,
    }

    /// WIT 绘图指令定义（展平结构，不支持递归类型）
    #[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
    pub struct WitDrawCmd {
        pub cmd_type: String,
        pub params: Vec<f64>,
        pub fill: Option<String>,
        pub stroke: Option<String>,
        pub stroke_width: Option<f64>,
        pub text_content: Option<String>,
        pub group_depth: u32,
    }

    /// WIT 命中区域定义
    #[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
    pub struct WitHitRegion {
        pub index: u32,
        pub series: Option<u32>,
        pub bounds_x: f64,
        pub bounds_y: f64,
        pub bounds_w: f64,
        pub bounds_h: f64,
    }

    /// WIT 渲染层定义
    #[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
    pub struct WitLayer {
        pub kind: String,
        pub dirty: bool,
        pub z_index: u32,
        pub commands: Vec<WitDrawCmd>,
        pub hit_regions: Vec<WitHitRegion>,
    }

    /// WIT 渲染结果定义
    #[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
    pub struct WitRenderResult {
        pub layers: Vec<WitLayer>,
    }
}

/// 类型转换层 — WIT types ↔ deneb internal types
pub mod convert {
    use super::wit_types::*;
    use deneb_core::{
        DataTable, Column, FieldValue, DataType,
        DrawCmd, LayerKind, Layer, HitRegion,
    };
    use deneb_component::{Mark, Field, Encoding, ChartSpec, ChartOutput};

    /// 转换错误
    #[derive(Debug, Clone, PartialEq)]
    pub enum ConvertError {
        /// 不支持的数据类型
        UnsupportedDataType(String),
        /// 不支持的 mark 类型
        UnsupportedMark(String),
        /// 缺少必需字段
        MissingRequiredField(String),
        /// 类型转换失败
        TypeMismatch(String),
    }

    impl std::fmt::Display for ConvertError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                ConvertError::UnsupportedDataType(s) => write!(f, "Unsupported data type: {}", s),
                ConvertError::UnsupportedMark(s) => write!(f, "Unsupported mark type: {}", s),
                ConvertError::MissingRequiredField(s) => write!(f, "Missing required field: {}", s),
                ConvertError::TypeMismatch(s) => write!(f, "Type mismatch: {}", s),
            }
        }
    }

    impl std::error::Error for ConvertError {}

    /// WIT 字段值转换为内部字段值
    pub fn wit_field_value_to_field_value(value: WitFieldValue) -> FieldValue {
        match value {
            WitFieldValue::Numeric(v) => FieldValue::Numeric(v),
            WitFieldValue::Text(v) => FieldValue::Text(v),
            WitFieldValue::Timestamp(v) => FieldValue::Timestamp(v),
            WitFieldValue::Boolean(v) => FieldValue::Bool(v),
            WitFieldValue::Null => FieldValue::Null,
        }
    }

    /// 内部字段值转换为 WIT 字段值
    pub fn field_value_to_wit_field_value(value: FieldValue) -> WitFieldValue {
        match value {
            FieldValue::Numeric(v) => WitFieldValue::Numeric(v),
            FieldValue::Text(v) => WitFieldValue::Text(v),
            FieldValue::Timestamp(v) => WitFieldValue::Timestamp(v),
            FieldValue::Bool(v) => WitFieldValue::Boolean(v),
            FieldValue::Null => WitFieldValue::Null,
        }
    }

    /// 字符串数据类型转换为内部数据类型
    pub fn str_to_data_type(s: &str) -> Result<DataType, ConvertError> {
        match s.to_lowercase().as_str() {
            "quantitative" => Ok(DataType::Quantitative),
            "temporal" => Ok(DataType::Temporal),
            "nominal" => Ok(DataType::Nominal),
            "ordinal" => Ok(DataType::Ordinal),
            _ => Err(ConvertError::UnsupportedDataType(s.to_string())),
        }
    }

    /// 内部数据类型转换为字符串
    pub fn data_type_to_str(dt: DataType) -> String {
        match dt {
            DataType::Quantitative => "quantitative".to_string(),
            DataType::Temporal => "temporal".to_string(),
            DataType::Nominal => "nominal".to_string(),
            DataType::Ordinal => "ordinal".to_string(),
        }
    }

    /// WIT 数据表转换为内部数据表
    pub fn wit_data_table_to_data_table(wit_table: WitDataTable) -> Result<DataTable, ConvertError> {
        if wit_table.columns.is_empty() {
            return Ok(DataTable::new());
        }

        let mut columns: Vec<Column> = Vec::new();
        let row_count = wit_table.rows.len();

        for (col_idx, wit_col) in wit_table.columns.iter().enumerate() {
            let data_type = str_to_data_type(&wit_col.data_type)?;
            let mut values = Vec::with_capacity(row_count);

            for row in &wit_table.rows {
                if col_idx < row.len() {
                    values.push(wit_field_value_to_field_value(row[col_idx].clone()));
                } else {
                    values.push(FieldValue::Null);
                }
            }

            columns.push(Column::new(wit_col.name.clone(), data_type, values));
        }

        Ok(DataTable::with_columns(columns))
    }

    /// 内部数据表转换为 WIT 数据表
    pub fn data_table_to_wit_data_table(table: &DataTable) -> WitDataTable {
        let columns = table.columns.iter().map(|col| WitSchemaField {
            name: col.name.clone(),
            data_type: data_type_to_str(col.data_type),
        }).collect();

        let row_count = table.row_count();
        let col_count = table.column_count();

        let mut rows = Vec::with_capacity(row_count);
        for row_idx in 0..row_count {
            let mut row = Vec::with_capacity(col_count);
            for col in &table.columns {
                let value = col.get(row_idx).unwrap_or(&FieldValue::Null);
                row.push(field_value_to_wit_field_value(value.clone()));
            }
            rows.push(row);
        }

        WitDataTable { columns, rows }
    }

    /// 字符串 mark 类型转换为内部 Mark
    pub fn str_to_mark(s: &str) -> Result<Mark, ConvertError> {
        match s.to_lowercase().as_str() {
            "line" => Ok(Mark::Line),
            "bar" => Ok(Mark::Bar),
            "scatter" => Ok(Mark::Scatter),
            "area" => Ok(Mark::Area),
            _ => Err(ConvertError::UnsupportedMark(s.to_string())),
        }
    }

    /// WIT 图表规格转换为内部图表规格
    pub fn wit_chart_spec_to_chart_spec(wit_spec: WitChartSpec) -> Result<ChartSpec, ConvertError> {
        wit_chart_spec_with_table(&wit_spec, &deneb_core::DataTable::new())
    }

    /// 从数据表的列类型推断字段编码类型
    pub fn wit_chart_spec_with_table(
        wit_spec: &WitChartSpec,
        table: &deneb_core::DataTable,
    ) -> Result<ChartSpec, ConvertError> {
        let mark = str_to_mark(&wit_spec.mark)?;

        let x_field = table.get_column(&wit_spec.x_field)
            .map(|col| match col.data_type {
                deneb_core::DataType::Nominal | deneb_core::DataType::Ordinal => Field::nominal(&wit_spec.x_field),
                deneb_core::DataType::Temporal => Field::temporal(&wit_spec.x_field),
                _ => Field::quantitative(&wit_spec.x_field),
            })
            .unwrap_or_else(|| Field::quantitative(&wit_spec.x_field));

        let y_field = table.get_column(&wit_spec.y_field)
            .map(|col| match col.data_type {
                deneb_core::DataType::Nominal | deneb_core::DataType::Ordinal => Field::nominal(&wit_spec.y_field),
                deneb_core::DataType::Temporal => Field::temporal(&wit_spec.y_field),
                _ => Field::quantitative(&wit_spec.y_field),
            })
            .unwrap_or_else(|| Field::quantitative(&wit_spec.y_field));

        let mut encoding = Encoding::new()
            .x(x_field)
            .y(y_field);

        if let Some(color_field) = &wit_spec.color_field {
            encoding = encoding.color(Field::nominal(color_field));
        }

        let mut builder = deneb_component::ChartSpec::builder()
            .mark(mark)
            .encoding(encoding)
            .width(wit_spec.width)
            .height(wit_spec.height);

        if let Some(title) = &wit_spec.title {
            builder = builder.title(title);
        }

        builder.build().map_err(|e| ConvertError::TypeMismatch(e.to_string()))
    }

    /// 内部 DrawCmd 转换为展平的 WIT DrawCmd 列表
    pub fn draw_cmd_to_wit_draw_cmd_flat(cmd: DrawCmd, depth: u32) -> Vec<WitDrawCmd> {
        match cmd {
            DrawCmd::Rect { x, y, width, height, fill, stroke, corner_radius: _ } => {
                vec![WitDrawCmd {
                    cmd_type: "rect".to_string(),
                    params: vec![x, y, width, height],
                    fill: fill.and_then(|f| match f {
                        deneb_core::FillStyle::Color(c) => Some(c),
                        _ => None,
                    }),
                    stroke: stroke.and_then(|s| match s {
                        deneb_core::StrokeStyle::Color(c) => Some(c),
                        deneb_core::StrokeStyle::None => None,
                    }),
                    stroke_width: None,
                    text_content: None,
                    group_depth: depth,
                }]
            }
            DrawCmd::Circle { cx, cy, r, fill, stroke } => {
                vec![WitDrawCmd {
                    cmd_type: "circle".to_string(),
                    params: vec![cx, cy, r],
                    fill: fill.and_then(|f| match f {
                        deneb_core::FillStyle::Color(c) => Some(c),
                        _ => None,
                    }),
                    stroke: stroke.and_then(|s| match s {
                        deneb_core::StrokeStyle::Color(c) => Some(c),
                        deneb_core::StrokeStyle::None => None,
                    }),
                    stroke_width: None,
                    text_content: None,
                    group_depth: depth,
                }]
            }
            DrawCmd::Text { x, y, content, style, anchor, baseline } => {
                // params: [x, y, font_size, anchor(0=Start,1=Middle,2=End), baseline(0=Top,1=Middle,2=Bottom,3=Alphabetic)]
                let anchor_code = match anchor {
                    deneb_core::TextAnchor::Start => 0.0,
                    deneb_core::TextAnchor::Middle => 1.0,
                    deneb_core::TextAnchor::End => 2.0,
                };
                let baseline_code = match baseline {
                    deneb_core::TextBaseline::Top => 0.0,
                    deneb_core::TextBaseline::Middle => 1.0,
                    deneb_core::TextBaseline::Bottom => 2.0,
                    deneb_core::TextBaseline::Alphabetic => 3.0,
                };
                vec![WitDrawCmd {
                    cmd_type: "text".to_string(),
                    params: vec![x, y, style.font_size, anchor_code, baseline_code],
                    fill: Some(match style.fill {
                        deneb_core::FillStyle::Color(c) => c,
                        _ => "#000".to_string(),
                    }),
                    stroke: None,
                    stroke_width: None,
                    text_content: Some(content),
                    group_depth: depth,
                }]
            }
            DrawCmd::Path { segments, fill, stroke } => {
                // 编码 PathSegment 到 params 数组
                // 格式：[type_code, ...coords] 逐段拼接
                // 0=MoveTo(x,y), 1=LineTo(x,y), 2=BezierTo(cp1x,cp1y,cp2x,cp2y,x,y),
                // 3=QuadraticTo(cpx,cpy,x,y), 4=Arc(cx,cy,r,start,end,ccw), 5=Close
                let mut params = Vec::new();
                for seg in &segments {
                    match seg {
                        deneb_core::PathSegment::MoveTo(x, y) => params.extend_from_slice(&[0.0, *x, *y]),
                        deneb_core::PathSegment::LineTo(x, y) => params.extend_from_slice(&[1.0, *x, *y]),
                        deneb_core::PathSegment::BezierTo(cp1x, cp1y, cp2x, cp2y, x, y) => {
                            params.extend_from_slice(&[2.0, *cp1x, *cp1y, *cp2x, *cp2y, *x, *y]);
                        }
                        deneb_core::PathSegment::QuadraticTo(cpx, cpy, x, y) => {
                            params.extend_from_slice(&[3.0, *cpx, *cpy, *x, *y]);
                        }
                        deneb_core::PathSegment::Arc(cx, cy, r, start, end, ccw) => {
                            params.extend_from_slice(&[4.0, *cx, *cy, *r, *start, *end, if *ccw { 1.0 } else { 0.0 }]);
                        }
                        deneb_core::PathSegment::Close => params.push(5.0),
                    }
                }

                vec![WitDrawCmd {
                    cmd_type: "path".to_string(),
                    params,
                    fill: fill.and_then(|f| match f {
                        deneb_core::FillStyle::Color(c) => Some(c),
                        _ => None,
                    }),
                    stroke: stroke.and_then(|s| match s {
                        deneb_core::StrokeStyle::Color(c) => Some(c),
                        deneb_core::StrokeStyle::None => None,
                    }),
                    stroke_width: None,
                    text_content: None,
                    group_depth: depth,
                }]
            }
            DrawCmd::Group { label: _, items } => {
                items.into_iter()
                    .flat_map(|c| draw_cmd_to_wit_draw_cmd_flat(c, depth + 1))
                    .collect()
            }
        }
    }

    /// 内部 LayerKind 转换为字符串
    pub fn layer_kind_to_str(kind: LayerKind) -> String {
        match kind {
            LayerKind::Background => "background".to_string(),
            LayerKind::Grid => "grid".to_string(),
            LayerKind::Axis => "axis".to_string(),
            LayerKind::Data => "data".to_string(),
            LayerKind::Legend => "legend".to_string(),
            LayerKind::Title => "title".to_string(),
            LayerKind::Annotation => "annotation".to_string(),
        }
    }

    /// 内部 HitRegion 转换为 WIT HitRegion
    pub fn hit_region_to_wit_hit_region(region: HitRegion) -> WitHitRegion {
        WitHitRegion {
            index: region.index as u32,
            series: region.series.map(|s| s as u32),
            bounds_x: region.bounds.x,
            bounds_y: region.bounds.y,
            bounds_w: region.bounds.width,
            bounds_h: region.bounds.height,
        }
    }

    /// 内部 Layer 转换为 WIT Layer
    pub fn layer_to_wit_layer(layer: Layer) -> WitLayer {
        let commands: Vec<WitDrawCmd> = layer.commands.semantic.into_iter()
            .flat_map(|c| draw_cmd_to_wit_draw_cmd_flat(c, 0))
            .collect();

        let hit_regions: Vec<WitHitRegion> = Vec::new(); // 命中区域在 ChartOutput 层级处理

        WitLayer {
            kind: layer_kind_to_str(layer.kind),
            dirty: layer.dirty,
            z_index: layer.z_index,
            commands,
            hit_regions,
        }
    }

    /// 内部 ChartOutput 转换为 WIT RenderResult
    pub fn chart_output_to_wit_render_result(output: ChartOutput) -> WitRenderResult {
        let mut layers = Vec::new();

        for layer in output.layers.all() {
            let mut wit_layer = layer_to_wit_layer(layer.clone());

            // 将命中区域添加到对应的层
            for region in &output.hit_regions {
                let wit_region = hit_region_to_wit_hit_region(region.clone());
                wit_layer.hit_regions.push(wit_region);
            }

            layers.push(wit_layer);
        }

        WitRenderResult { layers }
    }
}

/// 库调用模式 API
pub mod lib_mode {
    use super::wit_types::*;
    use super::convert::*;

    /// 解析数据
    pub fn parse_data(data: &[u8], format: &str) -> Result<WitDataTable, String> {
        let table = match format.to_lowercase().as_str() {
            "csv" => {
                #[cfg(feature = "csv")]
                {
                    let data_str = std::str::from_utf8(data)
                        .map_err(|e| format!("Invalid UTF-8: {}", e))?;
                    deneb_core::parser::csv::parse_csv(data_str).map_err(|e| e.to_string())?
                }
                #[cfg(not(feature = "csv"))]
                {
                    return Err("CSV format not enabled".to_string());
                }
            }
            "json" => {
                #[cfg(feature = "json")]
                {
                    let data_str = std::str::from_utf8(data)
                        .map_err(|e| format!("Invalid UTF-8: {}", e))?;
                    deneb_core::parser::json::parse_json(data_str).map_err(|e| e.to_string())?
                }
                #[cfg(not(feature = "json"))]
                {
                    return Err("JSON format not enabled".to_string());
                }
            }
            "arrow" => {
                #[cfg(feature = "arrow-format")]
                {
                    deneb_core::parser::arrow::parse_arrow_ipc(data).map_err(|e| e.to_string())?
                }
                #[cfg(not(feature = "arrow-format"))]
                {
                    return Err("Arrow format not enabled".to_string());
                }
            }
            "parquet" => {
                #[cfg(feature = "parquet-format")]
                {
                    deneb_core::parser::parquet::parse_parquet(data).map_err(|e| e.to_string())?
                }
                #[cfg(not(feature = "parquet-format"))]
                {
                    return Err("Parquet format not enabled".to_string());
                }
            }
            _ => return Err(format!("Unsupported format: {}", format)),
        };

        Ok(data_table_to_wit_data_table(&table))
    }

    /// 使用预解析的 WitDataTable 渲染图表
    pub fn render_from_wit_table(wit_table: WitDataTable, spec: WitChartSpec) -> Result<WitRenderResult, String> {
        let table = wit_data_table_to_data_table(wit_table).map_err(|e| e.to_string())?;
        let chart_spec = wit_chart_spec_with_table(&spec, &table).map_err(|e| e.to_string())?;

        let theme = deneb_component::DefaultTheme;
        let output = match chart_spec.mark {
            deneb_component::Mark::Line => {
                deneb_component::LineChart::render(&chart_spec, &theme, &table)
                    .map_err(|e| e.to_string())?
            }
            deneb_component::Mark::Bar => {
                deneb_component::BarChart::render(&chart_spec, &theme, &table)
                    .map_err(|e| e.to_string())?
            }
            deneb_component::Mark::Scatter => {
                deneb_component::ScatterChart::render(&chart_spec, &theme, &table)
                    .map_err(|e| e.to_string())?
            }
            deneb_component::Mark::Area => {
                deneb_component::AreaChart::render(&chart_spec, &theme, &table)
                    .map_err(|e| e.to_string())?
            }
        };

        Ok(chart_output_to_wit_render_result(output))
    }

    /// 渲染图表
    pub fn render(data: &[u8], format: &str, spec: WitChartSpec) -> Result<WitRenderResult, String> {
        let wit_table = parse_data(data, format)?;
        render_from_wit_table(wit_table, spec)
    }

    /// 命中测试
    pub fn hit_test(result: &WitRenderResult, x: f64, y: f64, tolerance: f64) -> Option<u32> {
        for layer in &result.layers {
            for region in &layer.hit_regions {
                let bounds = deneb_core::BoundingBox::new(
                    region.bounds_x,
                    region.bounds_y,
                    region.bounds_w,
                    region.bounds_h,
                );

                // 检查点是否在包围盒内（考虑 tolerance）
                let expanded = bounds.expand(tolerance);
                if expanded.contains(x, y) {
                    return Some(region.index);
                }
            }
        }
        None
    }
}

/// 独立组件模式
pub mod component_mode {
    use super::wit_types::*;
    use super::lib_mode;
    use serde_json;

    /// 组件输入配置
    #[derive(Debug, serde::Deserialize)]
    pub struct ComponentInput {
        pub data: String,  // base64 编码的字节
        pub format: String,
        pub spec: WitChartSpec,
    }

    /// 组件输出结果
    #[derive(Debug, serde::Serialize)]
    pub struct ComponentOutput {
        pub result: WitRenderResult,
    }

    /// 运行组件（从 stdin 读取配置，输出到 stdout）
    pub fn run_component() -> Result<(), String> {
        use std::io::{self, Read};

        // 1. 从 stdin 读取 JSON 配置
        let mut input_str = String::new();
        io::stdin().read_to_string(&mut input_str)
            .map_err(|e| format!("Failed to read stdin: {}", e))?;

        // 2. 解析配置
        let input: ComponentInput = serde_json::from_str(&input_str)
            .map_err(|e| format!("Failed to parse input JSON: {}", e))?;

        // 3. 解码数据
        use base64::Engine;
        let data_bytes = base64::engine::general_purpose::STANDARD.decode(&input.data)
            .map_err(|e| format!("Failed to decode base64 data: {}", e))?;

        // 4. 调用渲染
        let result = lib_mode::render(&data_bytes, &input.format, input.spec)?;

        // 5. 输出结果到 stdout
        let output = ComponentOutput { result };
        let output_json = serde_json::to_string_pretty(&output)
            .map_err(|e| format!("Failed to serialize output: {}", e))?;

        println!("{}", output_json);
        Ok(())
    }
}

// 重新导出 WIT 类型
pub use wit_types::*;
pub use lib_mode::*;
