//! WASM host — 通过 wasmtime 加载 deneb-viz WASI Component 并调用渲染

use deneb_wit::wit_types::*;

// wasmtime bindgen! 宏从 WIT 生成 host 端绑定
wasmtime::component::bindgen!({
    path: "../deneb-wit/wit",
    world: "deneb-viz",
});

use exports::deneb::viz::data_parser::{DataTable as BgDataTable, FieldValue as BgFieldValue};
use exports::deneb::viz::chart_renderer::{
    ChartSpec as BgChartSpec, DrawCmd as BgDrawCmd, HitRegion as BgHitRegion,
    Layer as BgLayer, RenderResult as BgRenderResult,
};
use limpuai::data::types::DataTable as LimpuDataTable;

/// WASM 组件加载或调用错误
#[derive(Debug)]
pub enum WasmHostError {
    Engine(String),
    ComponentLoad(String),
    Instantiate(String),
    Call(String),
}

impl std::fmt::Display for WasmHostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WasmHostError::Engine(s) => write!(f, "Engine error: {}", s),
            WasmHostError::ComponentLoad(s) => write!(f, "Component load error: {}", s),
            WasmHostError::Instantiate(s) => write!(f, "Instantiate error: {}", s),
            WasmHostError::Call(s) => write!(f, "Call error: {}", s),
        }
    }
}

impl std::error::Error for WasmHostError {}

/// WASI 状态 — 每个 store 的 WASI 上下文
struct WasiState {
    ctx: wasmtime_wasi::WasiCtx,
    table: wasmtime::component::ResourceTable,
}

impl wasmtime_wasi::WasiView for WasiState {
    fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
        wasmtime_wasi::WasiCtxView { ctx: &mut self.ctx, table: &mut self.table }
    }
}

/// 解析器组件路径（可选）
#[derive(Default)]
pub struct ParserPaths {
    pub arrow: Option<String>,
    pub parquet: Option<String>,
}

impl ParserPaths {
    /// 从目录自动发现解析器 .wasm 文件
    pub fn from_dir(dir: &str) -> Self {
        let arrow = std::path::Path::new(dir)
            .join("limpuai_wit_arrow.wasm")
            .to_str()
            .filter(|p| std::path::Path::new(p).exists())
            .map(String::from);
        let parquet = std::path::Path::new(dir)
            .join("limpuai_wit_parquet.wasm")
            .to_str()
            .filter(|p| std::path::Path::new(p).exists())
            .map(String::from);
        Self { arrow, parquet }
    }
}

/// WASM host — 管理 WASI Component 生命周期
pub struct WasmHost {
    engine: wasmtime::Engine,
    store: wasmtime::Store<WasiState>,
    bindings: DenebViz,
}

impl WasmHost {
    /// 从 .wasm 文件加载并实例化（无 Arrow/Parquet 解析器）
    pub fn from_file(wasm_path: &str) -> Result<Self, WasmHostError> {
        Self::from_file_with_parsers(wasm_path, ParserPaths {
            arrow: None,
            parquet: None,
        })
    }

    /// 从 .wasm 文件加载并实例化，可选提供解析器组件
    pub fn from_file_with_parsers(
        viz_wasm_path: &str,
        parser_paths: ParserPaths,
    ) -> Result<Self, WasmHostError> {
        let engine = wasmtime::Engine::new(
            wasmtime::Config::new().wasm_component_model(true),
        ).map_err(|e| WasmHostError::Engine(e.to_string()))?;

        let viz_component = wasmtime::component::Component::from_file(
            &engine, viz_wasm_path,
        ).map_err(|e| WasmHostError::ComponentLoad(e.to_string()))?;

        let mut linker = wasmtime::component::Linker::<WasiState>::new(&engine);

        wasmtime_wasi::p2::add_to_linker_sync(&mut linker)
            .map_err(|e| WasmHostError::Instantiate(format!("WASI linker: {}", e)))?;

        let wasi_ctx = wasmtime_wasi::WasiCtx::builder()
            .inherit_stdio()
            .build();

        let mut store = wasmtime::Store::new(&engine, WasiState {
            ctx: wasi_ctx,
            table: wasmtime::component::ResourceTable::new(),
        });

        match parser_paths.arrow {
            Some(ref path) => {
                let func = Self::load_parser_func(&engine, &mut store, &mut linker, path, "limpuai:data/arrow-parser")?;
                linker.instance("limpuai:data/arrow-parser")
                    .map_err(|e| WasmHostError::Instantiate(format!("arrow instance: {}", e)))?
                    .func_wrap("parse",
                        move |mut store: wasmtime::StoreContextMut<'_, WasiState>,
                              (data,): (Vec<u8>,)| {
                            let typed_fn = func.typed::<(Vec<u8>,), (Result<LimpuDataTable, String>,)>(&mut store)
                                .map_err(|e| wasmtime::Error::msg(e.to_string()))?;
                            let result = typed_fn.call(&mut store, (data,))
                                .map_err(|e| wasmtime::Error::msg(e.to_string()))?;
                            Ok(result)
                        }
                    )
                    .map_err(|e| WasmHostError::Instantiate(format!("arrow func: {}", e)))?;
            }
            None => {
                Self::register_stub(&mut linker, "limpuai:data/arrow-parser", "Arrow")?;
            }
        }

        match parser_paths.parquet {
            Some(ref path) => {
                let func = Self::load_parser_func(&engine, &mut store, &mut linker, path, "limpuai:data/parquet-parser")?;
                linker.instance("limpuai:data/parquet-parser")
                    .map_err(|e| WasmHostError::Instantiate(format!("parquet instance: {}", e)))?
                    .func_wrap("parse",
                        move |mut store: wasmtime::StoreContextMut<'_, WasiState>,
                              (data,): (Vec<u8>,)| {
                            let typed_fn = func.typed::<(Vec<u8>,), (Result<LimpuDataTable, String>,)>(&mut store)
                                .map_err(|e| wasmtime::Error::msg(e.to_string()))?;
                            let result = typed_fn.call(&mut store, (data,))
                                .map_err(|e| wasmtime::Error::msg(e.to_string()))?;
                            Ok(result)
                        }
                    )
                    .map_err(|e| WasmHostError::Instantiate(format!("parquet func: {}", e)))?;
            }
            None => {
                Self::register_stub(&mut linker, "limpuai:data/parquet-parser", "Parquet")?;
            }
        }

        let bindings = DenebViz::instantiate(&mut store, &viz_component, &linker)
            .map_err(|e| WasmHostError::Instantiate(format!("Instantiate: {}", e)))?;

        Ok(Self { engine, store, bindings })
    }

    /// 加载解析器组件，实例化并返回其 parse 导出函数
    fn load_parser_func(
        engine: &wasmtime::Engine,
        store: &mut wasmtime::Store<WasiState>,
        linker: &mut wasmtime::component::Linker<WasiState>,
        wasm_path: &str,
        instance_name: &str,
    ) -> Result<wasmtime::component::Func, WasmHostError> {
        let component = wasmtime::component::Component::from_file(engine, wasm_path)
            .map_err(|e| WasmHostError::ComponentLoad(format!("{}: {}", instance_name, e)))?;
        let root = linker.instantiate(&mut *store, &component)
            .map_err(|e| WasmHostError::Instantiate(format!("{} instantiate: {}", instance_name, e)))?;
        let instance_idx = root.get_export_index(&mut *store, None, instance_name)
            .ok_or_else(|| WasmHostError::Instantiate(format!("nested instance '{}' not found", instance_name)))?;
        let func_idx = root.get_export_index(&mut *store, Some(&instance_idx), "parse")
            .ok_or_else(|| WasmHostError::Instantiate(format!("{}: export 'parse' not found", instance_name)))?;
        root.get_func(&mut *store, &func_idx)
            .ok_or_else(|| WasmHostError::Instantiate(format!("{}: 'parse' is not a function", instance_name)))
    }

    /// 注册 stub 解析器（未提供 .wasm 时使用）
    fn register_stub(
        linker: &mut wasmtime::component::Linker<WasiState>,
        instance_name: &str,
        label: &str,
    ) -> Result<(), WasmHostError> {
        let msg = format!("{} parser not available (no .wasm provided)", label);
        linker.instance(instance_name)
            .map_err(|e| WasmHostError::Instantiate(format!("{} stub: {}", label, e)))?
            .func_wrap("parse",
                move |_: wasmtime::StoreContextMut<'_, WasiState>,
                      (_data,): (Vec<u8>,)| {
                    let r: Result<LimpuDataTable, String> = Err(msg.clone());
                    Ok((r,))
                }
            )
            .map_err(|e| WasmHostError::Instantiate(format!("{} stub func: {}", label, e)))?;
        Ok(())
    }

    /// 调用组件的 parse-csv 函数
    pub fn parse_csv(&mut self, data: &[u8]) -> Result<WitDataTable, WasmHostError> {
        let result = self.bindings
            .deneb_viz_data_parser()
            .call_parse_csv(&mut self.store, data)
            .map_err(|e: wasmtime::Error| WasmHostError::Call(e.to_string()))?
            .map_err(|e: String| WasmHostError::Call(format!("parse_csv: {}", e)))?;

        Ok(bg_to_wit_data_table(result))
    }

    /// 调用组件的 parse-json 函数
    pub fn parse_json(&mut self, data: &[u8]) -> Result<WitDataTable, WasmHostError> {
        let result = self.bindings
            .deneb_viz_data_parser()
            .call_parse_json(&mut self.store, data)
            .map_err(|e: wasmtime::Error| WasmHostError::Call(e.to_string()))?
            .map_err(|e: String| WasmHostError::Call(format!("parse_json: {}", e)))?;

        Ok(bg_to_wit_data_table(result))
    }

    /// 调用组件的 parse-arrow 函数
    pub fn parse_arrow(&mut self, data: &[u8]) -> Result<WitDataTable, WasmHostError> {
        let result = self.bindings
            .deneb_viz_data_parser()
            .call_parse_arrow(&mut self.store, data)
            .map_err(|e: wasmtime::Error| WasmHostError::Call(e.to_string()))?
            .map_err(|e: String| WasmHostError::Call(format!("parse_arrow: {}", e)))?;

        Ok(bg_to_wit_data_table(result))
    }

    /// 调用组件的 parse-parquet 函数
    pub fn parse_parquet(&mut self, data: &[u8]) -> Result<WitDataTable, WasmHostError> {
        let result = self.bindings
            .deneb_viz_data_parser()
            .call_parse_parquet(&mut self.store, data)
            .map_err(|e: wasmtime::Error| WasmHostError::Call(e.to_string()))?
            .map_err(|e: String| WasmHostError::Call(format!("parse_parquet: {}", e)))?;

        Ok(bg_to_wit_data_table(result))
    }

    /// 调用组件的 render 函数
    pub fn render(
        &mut self,
        data: &[u8],
        format: &str,
        spec: &WitChartSpec,
    ) -> Result<WitRenderResult, WasmHostError> {
        let bg_spec = wit_to_bg_chart_spec(spec);
        let format_str = format.to_string();

        let result = self.bindings
            .deneb_viz_chart_renderer()
            .call_render(&mut self.store, data, &format_str, &bg_spec)
            .map_err(|e: wasmtime::Error| WasmHostError::Call(e.to_string()))?
            .map_err(|e: String| WasmHostError::Call(format!("render: {}", e)))?;

        Ok(bg_to_wit_render_result(result))
    }

    /// 调用组件的 hit-test 函数
    pub fn hit_test(
        &mut self,
        result: &WitRenderResult,
        x: f64,
        y: f64,
        tolerance: f64,
    ) -> Result<Option<u32>, WasmHostError> {
        let bg_result = wit_to_bg_render_result(result);
        self.bindings
            .deneb_viz_chart_renderer()
            .call_hit_test(&mut self.store, &bg_result, x, y, tolerance)
            .map_err(|e: wasmtime::Error| WasmHostError::Call(e.to_string()))
    }

    /// 获取 engine 引用
    pub fn engine(&self) -> &wasmtime::Engine {
        &self.engine
    }
}

// ——— bindgen 生成类型 → WitXxx ———

fn bg_to_wit_data_table(t: BgDataTable) -> WitDataTable {
    WitDataTable {
        columns: t.columns.into_iter().map(|c| WitSchemaField {
            name: c.name,
            data_type: c.data_type,
        }).collect(),
        rows: t.rows.into_iter().map(|row: Vec<BgFieldValue>| {
            row.into_iter().map(bg_to_wit_field_value).collect()
        }).collect(),
    }
}

fn bg_to_wit_field_value(v: BgFieldValue) -> WitFieldValue {
    match v {
        BgFieldValue::Numeric(f) => WitFieldValue::Numeric(f),
        BgFieldValue::Text(s) => WitFieldValue::Text(s),
        BgFieldValue::Timestamp(f) => WitFieldValue::Timestamp(f),
        BgFieldValue::Boolean(b) => WitFieldValue::Boolean(b),
        BgFieldValue::Null => WitFieldValue::Null,
    }
}

fn bg_to_wit_render_result(r: BgRenderResult) -> WitRenderResult {
    WitRenderResult {
        layers: r.layers.into_iter().map(bg_to_wit_layer).collect(),
    }
}

fn bg_to_wit_layer(l: BgLayer) -> WitLayer {
    WitLayer {
        kind: l.kind,
        dirty: l.dirty,
        z_index: l.z_index,
        commands: l.commands.into_iter().map(bg_to_wit_draw_cmd).collect(),
        hit_regions: l.hit_regions.into_iter().map(|r| WitHitRegion {
            index: r.index,
            series: r.series,
            bounds_x: r.bounds_x,
            bounds_y: r.bounds_y,
            bounds_w: r.bounds_w,
            bounds_h: r.bounds_h,
        }).collect(),
    }
}

fn bg_to_wit_draw_cmd(c: BgDrawCmd) -> WitDrawCmd {
    WitDrawCmd {
        cmd_type: c.cmd_type,
        params: c.params,
        fill: c.fill,
        stroke: c.stroke,
        stroke_width: c.stroke_width,
        text_content: c.text_content,
        group_depth: c.group_depth,
    }
}

// ——— WitXxx → bindgen 生成类型 ———

fn wit_to_bg_chart_spec(spec: &WitChartSpec) -> BgChartSpec {
    BgChartSpec {
        mark: spec.mark.clone(),
        x_field: spec.x_field.clone(),
        y_field: spec.y_field.clone(),
        color_field: spec.color_field.clone(),
        width: spec.width,
        height: spec.height,
        title: spec.title.clone(),
        theme: spec.theme.clone(),
    }
}

fn wit_to_bg_render_result(r: &WitRenderResult) -> BgRenderResult {
    BgRenderResult {
        layers: r.layers.iter().map(wit_to_bg_layer).collect(),
    }
}

fn wit_to_bg_layer(l: &WitLayer) -> BgLayer {
    BgLayer {
        kind: l.kind.clone(),
        dirty: l.dirty,
        z_index: l.z_index,
        commands: l.commands.iter().map(wit_to_bg_draw_cmd).collect(),
        hit_regions: l.hit_regions.iter().map(|r| BgHitRegion {
            index: r.index,
            series: r.series,
            bounds_x: r.bounds_x,
            bounds_y: r.bounds_y,
            bounds_w: r.bounds_w,
            bounds_h: r.bounds_h,
        }).collect(),
    }
}

fn wit_to_bg_draw_cmd(c: &WitDrawCmd) -> BgDrawCmd {
    BgDrawCmd {
        cmd_type: c.cmd_type.clone(),
        params: c.params.clone(),
        fill: c.fill.clone(),
        stroke: c.stroke.clone(),
        stroke_width: c.stroke_width,
        text_content: c.text_content.clone(),
        group_depth: c.group_depth,
    }
}
