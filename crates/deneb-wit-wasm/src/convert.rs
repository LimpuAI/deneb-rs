//! guest 侧类型转换 — wit-bindgen 类型 ↔ deneb-wit Wit* 类型 ↔ limpuai:data 类型

use deneb_wit::wit_types::*;
use crate::limpuai::data::types::{
    DataTable as LimpuDataTable, FieldValue as LimpuFieldValue,
};

use crate::exports::deneb::viz::chart_renderer as cr;
use crate::exports::deneb::viz::data_parser as dp;

// ---------- data-parser:WitDataTable ↔ bindgen / limpuai → Wit ----------

pub fn wit_data_table_to_bindgen(t: WitDataTable) -> dp::DataTable {
    dp::DataTable {
        columns: t
            .columns
            .into_iter()
            .map(|c| dp::SchemaField {
                name: c.name,
                data_type: c.data_type,
            })
            .collect(),
        rows: t
            .rows
            .into_iter()
            .map(|row| row.into_iter().map(wit_field_to_bindgen).collect())
            .collect(),
    }
}

fn wit_field_to_bindgen(v: WitFieldValue) -> dp::FieldValue {
    match v {
        WitFieldValue::Numeric(f) => dp::FieldValue::Numeric(f),
        WitFieldValue::Text(s) => dp::FieldValue::Text(s),
        WitFieldValue::Timestamp(f) => dp::FieldValue::Timestamp(f),
        WitFieldValue::Boolean(b) => dp::FieldValue::Boolean(b),
        WitFieldValue::Null => dp::FieldValue::Null,
    }
}

/// Arrow 物理类型 → deneb 语义类型
pub fn arrow_type_to_semantic(ty: &str) -> &'static str {
    match ty {
        "Int8" | "Int16" | "Int32" | "Int64" | "UInt8" | "UInt16" | "UInt32" | "UInt64"
        | "Float16" | "Float32" | "Float64" | "Decimal128" | "Decimal256" => "quantitative",
        "Date32" | "Date64" | "Timestamp" | "Time32" | "Time64" | "Duration" => "temporal",
        "Utf8" | "LargeUtf8" | "Binary" | "LargeBinary" => "nominal",
        "Boolean" => "nominal",
        _ => "nominal",
    }
}

pub fn limpuai_dt_to_bindgen(dt: LimpuDataTable) -> dp::DataTable {
    dp::DataTable {
        columns: dt
            .columns
            .into_iter()
            .map(|c| dp::SchemaField {
                name: c.name,
                data_type: arrow_type_to_semantic(&c.data_type).to_string(),
            })
            .collect(),
        rows: dt
            .rows
            .into_iter()
            .map(|row| row.into_iter().map(limpuai_field_to_bindgen).collect())
            .collect(),
    }
}

fn limpuai_field_to_bindgen(v: LimpuFieldValue) -> dp::FieldValue {
    match v {
        LimpuFieldValue::Numeric(f) => dp::FieldValue::Numeric(f),
        LimpuFieldValue::Text(s) => dp::FieldValue::Text(s),
        LimpuFieldValue::Timestamp(f) => dp::FieldValue::Timestamp(f),
        LimpuFieldValue::Boolean(b) => dp::FieldValue::Boolean(b),
        LimpuFieldValue::Null => dp::FieldValue::Null,
    }
}

pub fn limpuai_dt_to_wit(dt: LimpuDataTable) -> WitDataTable {
    WitDataTable {
        columns: dt
            .columns
            .into_iter()
            .map(|c| WitSchemaField {
                name: c.name,
                data_type: arrow_type_to_semantic(&c.data_type).to_string(),
            })
            .collect(),
        rows: dt
            .rows
            .into_iter()
            .map(|row| row.into_iter().map(limpuai_field_to_wit).collect())
            .collect(),
    }
}

fn limpuai_field_to_wit(v: LimpuFieldValue) -> WitFieldValue {
    match v {
        LimpuFieldValue::Numeric(f) => WitFieldValue::Numeric(f),
        LimpuFieldValue::Text(s) => WitFieldValue::Text(s),
        LimpuFieldValue::Timestamp(f) => WitFieldValue::Timestamp(f),
        LimpuFieldValue::Boolean(b) => WitFieldValue::Boolean(b),
        LimpuFieldValue::Null => WitFieldValue::Null,
    }
}

// ---------- chart-renderer:bindgen → Wit(输入方向) ----------

pub fn bindgen_to_wit_chart_spec(spec: cr::ChartSpec) -> WitChartSpec {
    WitChartSpec {
        mark: spec.mark,
        x_field: spec.x_field,
        y_field: spec.y_field,
        color_field: spec.color_field,
        open_field: spec.open_field,
        high_field: spec.high_field,
        low_field: spec.low_field,
        close_field: spec.close_field,
        theta_field: spec.theta_field,
        size_field: spec.size_field,
        width: spec.width,
        height: spec.height,
        title: spec.title,
        animation: spec.animation.map(|a| WitAnimationConfig {
            enter_duration_ms: a.enter_duration_ms,
            stagger_ms: a.stagger_ms,
            easing: a.easing,
            disable: a.disable,
        }),
    }
}

pub fn bindgen_to_wit_theme(theme: cr::Theme) -> WitTheme {
    WitTheme {
        palette: theme.palette,
        background: theme.background,
        foreground: theme.foreground,
        grid: theme.grid,
        axis: theme.axis,
        title_color: theme.title_color,
        focus_color: theme.focus_color,
        font_family: theme.font_family,
        base_font_size: theme.base_font_size,
        title_font_size: theme.title_font_size,
        label_font_size: theme.label_font_size,
        tick_font_size: theme.tick_font_size,
        margin: (
            theme.margin.top,
            theme.margin.right,
            theme.margin.bottom,
            theme.margin.left,
        ),
    }
}

// ---------- chart-renderer:Wit → bindgen(输出方向) ----------

pub fn wit_render_result_to_bindgen(r: WitRenderResult) -> cr::RenderResult {
    cr::RenderResult {
        layers: r.layers.into_iter().map(wit_layer_to_bindgen).collect(),
    }
}

fn wit_layer_to_bindgen(l: WitLayer) -> cr::Layer {
    cr::Layer {
        kind: l.kind,
        dirty: l.dirty,
        z_index: l.z_index,
        commands: l.commands.into_iter().map(wit_draw_cmd_to_bindgen).collect(),
    }
}

fn wit_paint_to_bindgen(p: WitPaint) -> cr::Paint {
    match p {
        WitPaint::Solid(c) => cr::Paint::Solid(c),
        WitPaint::Gradient(g) => cr::Paint::Gradient(cr::LinearGradient {
            x0: g.x0,
            y0: g.y0,
            x1: g.x1,
            y1: g.y1,
            stops: g
                .stops
                .into_iter()
                .map(|s| cr::GradientStop { pos: s.pos, color: s.color })
                .collect(),
        }),
    }
}

fn wit_anim_to_bindgen(a: WitAnimDesc) -> cr::AnimDesc {
    cr::AnimDesc {
        property: match a.property {
            WitAnimProperty::Opacity => cr::AnimProperty::Opacity,
        },
        keyframes: a
            .keyframes
            .into_iter()
            .map(|k| cr::Keyframe { t: k.t, value: k.value, easing: k.easing })
            .collect(),
        duration_ms: a.duration_ms,
        delay_ms: a.delay_ms,
        loop_: match a.loop_mode {
            WitLoopMode::Once => cr::LoopMode::Once,
            WitLoopMode::Loop => cr::LoopMode::Loop,
            WitLoopMode::PingPong => cr::LoopMode::PingPong,
        },
    }
}

fn wit_draw_cmd_to_bindgen(c: WitDrawCmd) -> cr::DrawCmd {
    cr::DrawCmd {
        cmd_type: c.cmd_type,
        params: c.params,
        fill: c.fill.map(wit_paint_to_bindgen),
        stroke: c.stroke.map(wit_paint_to_bindgen),
        stroke_width: c.stroke_width,
        corner_radius: c.corner_radius,
        text_content: c.text_content,
        font: c.font.map(|f| cr::FontDesc {
            family: f.family,
            weight: f.weight,
            italic: f.italic,
        }),
        group_depth: c.group_depth,
        anim: c.anim.map(wit_anim_to_bindgen),
    }
}

pub fn wit_hit_region_to_bindgen(r: WitHitRegion) -> cr::HitRegion {
    cr::HitRegion {
        index: r.index,
        series: r.series,
        bounds_x: r.bounds_x,
        bounds_y: r.bounds_y,
        bounds_w: r.bounds_w,
        bounds_h: r.bounds_h,
        datum: r.datum.into_iter().map(wit_field_to_bindgen).collect(),
    }
}
