//! deneb-wit-wasm: WASI Component Model 导出层
//!
//! 使用 wit-bindgen 0.51 从 world.wit 生成 guest 绑定，
//! 将 deneb-wit 的功能导出为标准 WASI Component。

wit_bindgen::generate!({
    world: "deneb-viz",
    path: "../deneb-wit/wit",
});

use deneb_wit::lib_mode;
use deneb_wit::wit_types::*;

use exports::deneb::viz::chart_renderer::{Guest as ChartRendererGuest, ChartSpec as Cs, DrawCmd as Dc, HitRegion as Hr, Layer as Ly, RenderResult as Rr};
use exports::deneb::viz::data_parser::{Guest as DataParserGuest, SchemaField as Sf};

struct DenebVizComponent;

impl DataParserGuest for DenebVizComponent {
    fn parse_csv(data: Vec<u8>) -> Result<exports::deneb::viz::data_parser::DataTable, String> {
        let wit = lib_mode::parse_data(&data, "csv")?;
        Ok(wit_data_table_to_bindgen(wit))
    }

    fn parse_json(data: Vec<u8>) -> Result<exports::deneb::viz::data_parser::DataTable, String> {
        let wit = lib_mode::parse_data(&data, "json")?;
        Ok(wit_data_table_to_bindgen(wit))
    }

    fn parse_arrow(data: Vec<u8>) -> Result<exports::deneb::viz::data_parser::DataTable, String> {
        let _ = data;
        Err("Arrow format not available in WASM component".to_string())
    }

    fn parse_parquet(data: Vec<u8>) -> Result<exports::deneb::viz::data_parser::DataTable, String> {
        let _ = data;
        Err("Parquet format not available in WASM component".to_string())
    }
}

impl ChartRendererGuest for DenebVizComponent {
    fn render(
        data: Vec<u8>,
        format: String,
        spec: Cs,
    ) -> Result<Rr, String> {
        let wit_spec = WitChartSpec {
            mark: spec.mark,
            x_field: spec.x_field,
            y_field: spec.y_field,
            color_field: spec.color_field,
            width: spec.width,
            height: spec.height,
            title: spec.title,
            theme: spec.theme,
        };

        let wit_result = lib_mode::render(&data, &format, wit_spec)?;
        Ok(wit_render_result_to_bindgen(wit_result))
    }

    fn hit_test(render_data: Rr, x: f64, y: f64, tolerance: f64) -> Option<u32> {
        let wit = bindgen_render_result_to_wit(render_data);
        lib_mode::hit_test(&wit, x, y, tolerance)
    }
}

// WitXxx → wit-bindgen 类型

fn wit_data_table_to_bindgen(t: WitDataTable) -> exports::deneb::viz::data_parser::DataTable {
    exports::deneb::viz::data_parser::DataTable {
        columns: t.columns.into_iter().map(|c| Sf {
            name: c.name,
            data_type: c.data_type,
        }).collect(),
        rows: t.rows.into_iter().map(|row| {
            row.into_iter().map(wit_field_to_bindgen).collect()
        }).collect(),
    }
}

fn wit_field_to_bindgen(v: WitFieldValue) -> exports::deneb::viz::data_parser::FieldValue {
    match v {
        WitFieldValue::Numeric(f) => exports::deneb::viz::data_parser::FieldValue::Numeric(f),
        WitFieldValue::Text(s) => exports::deneb::viz::data_parser::FieldValue::Text(s),
        WitFieldValue::Timestamp(f) => exports::deneb::viz::data_parser::FieldValue::Timestamp(f),
        WitFieldValue::Boolean(b) => exports::deneb::viz::data_parser::FieldValue::Boolean(b),
        WitFieldValue::Null => exports::deneb::viz::data_parser::FieldValue::Null,
    }
}

fn wit_render_result_to_bindgen(r: WitRenderResult) -> Rr {
    Rr {
        layers: r.layers.into_iter().map(wit_layer_to_bindgen).collect(),
    }
}

fn wit_layer_to_bindgen(l: WitLayer) -> Ly {
    Ly {
        kind: l.kind,
        dirty: l.dirty,
        z_index: l.z_index,
        commands: l.commands.into_iter().map(wit_draw_cmd_to_bindgen).collect(),
        hit_regions: l.hit_regions.into_iter().map(|r| Hr {
            index: r.index,
            series: r.series,
            bounds_x: r.bounds_x,
            bounds_y: r.bounds_y,
            bounds_w: r.bounds_w,
            bounds_h: r.bounds_h,
        }).collect(),
    }
}

fn wit_draw_cmd_to_bindgen(c: WitDrawCmd) -> Dc {
    Dc {
        cmd_type: c.cmd_type,
        params: c.params,
        fill: c.fill,
        stroke: c.stroke,
        stroke_width: c.stroke_width,
        text_content: c.text_content,
        group_depth: c.group_depth,
    }
}

// wit-bindgen 类型 → WitXxx

fn bindgen_render_result_to_wit(r: Rr) -> WitRenderResult {
    WitRenderResult {
        layers: r.layers.into_iter().map(|l| WitLayer {
            kind: l.kind,
            dirty: l.dirty,
            z_index: l.z_index,
            commands: l.commands.into_iter().map(bindgen_draw_cmd_to_wit).collect(),
            hit_regions: l.hit_regions.into_iter().map(|r| WitHitRegion {
                index: r.index,
                series: r.series,
                bounds_x: r.bounds_x,
                bounds_y: r.bounds_y,
                bounds_w: r.bounds_w,
                bounds_h: r.bounds_h,
            }).collect(),
        }).collect(),
    }
}

fn bindgen_draw_cmd_to_wit(c: Dc) -> WitDrawCmd {
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

export!(DenebVizComponent);
