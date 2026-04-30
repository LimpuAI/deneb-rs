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

/// WASM host — 管理 WASI Component 生命周期
pub struct WasmHost {
    engine: wasmtime::Engine,
    store: wasmtime::Store<WasiState>,
    bindings: DenebViz,
}

impl WasmHost {
    /// 从 .wasm 文件加载并实例化 WASI Component
    pub fn from_file(wasm_path: &str) -> Result<Self, WasmHostError> {
        let engine = wasmtime::Engine::new(
            wasmtime::Config::new().wasm_component_model(true),
        ).map_err(|e| WasmHostError::Engine(e.to_string()))?;

        let component = wasmtime::component::Component::from_file(
            &engine, wasm_path,
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

        let bindings = DenebViz::instantiate(&mut store, &component, &linker)
            .map_err(|e| WasmHostError::Instantiate(format!("Instantiate: {}", e)))?;

        Ok(Self { engine, store, bindings })
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
