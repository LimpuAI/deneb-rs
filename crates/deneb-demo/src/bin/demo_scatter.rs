//! 散点图演示（Parquet 格式）

use deneb_component::{
    ChartSpec, DarkTheme, DefaultTheme, Encoding, Field, ForestTheme, Mark, NordicTheme,
    CappuccinoTheme, ScatterChart,
};
use deneb_core::parser::parquet::parse_parquet;
use deneb_demo::{parse_theme_name, parse_wasm_args, render_and_show, sample_data, DemoApp,
    TinySkiaRenderer};
use deneb_demo::wasm_host::{ParserPaths, WasmHost};
use deneb_wit::wit_types::WitChartSpec;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let parquet_data = sample_data::scatter_chart_parquet();

    if let Some(args) = parse_wasm_args() {
        let parsers = args.deps_dir.as_deref()
            .map(ParserPaths::from_dir)
            .unwrap_or_default();
        let mut host = WasmHost::from_file_with_parsers(&args.wasm_path, parsers)?;
        run_wasm(&mut host, &parquet_data)?;
    } else {
        run_direct(&parquet_data)?;
    }

    Ok(())
}

fn run_direct(data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let table = parse_parquet(data)?;

    let spec = ChartSpec::builder()
        .mark(Mark::Scatter)
        .encoding(
            Encoding::new()
                .x(Field::quantitative("x"))
                .y(Field::quantitative("y"))
                .color(Field::nominal("group")),
        )
        .title("Scatter Chart Demo")
        .width(800.0)
        .height(600.0)
        .build()?;

    let theme_name = parse_theme_name();
    match theme_name.as_deref() {
        Some("dark") => render_and_show(DarkTheme, |t| ScatterChart::render(&spec, t, &table), "Deneb - Scatter Chart"),
        Some("forest") => render_and_show(ForestTheme, |t| ScatterChart::render(&spec, t, &table), "Deneb - Scatter Chart"),
        Some("nordic") => render_and_show(NordicTheme, |t| ScatterChart::render(&spec, t, &table), "Deneb - Scatter Chart"),
        Some("cappuccino") => render_and_show(CappuccinoTheme, |t| ScatterChart::render(&spec, t, &table), "Deneb - Scatter Chart"),
        _ => render_and_show(DefaultTheme, |t| ScatterChart::render(&spec, t, &table), "Deneb - Scatter Chart"),
    }
}

fn run_wasm(host: &mut WasmHost, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let wit_spec = WitChartSpec {
        mark: "scatter".to_string(),
        x_field: "x".to_string(),
        y_field: "y".to_string(),
        color_field: Some("group".to_string()),
        open_field: None,
        high_field: None,
        low_field: None,
        close_field: None,
        theta_field: None,
        size_field: None,
        width: 800.0,
        height: 600.0,
        title: Some("Scatter Chart Demo (WASM)".to_string()),
        theme: parse_theme_name(),
    };

    let wit_result = host.render(data, "parquet", &wit_spec)?;

    let mut renderer = TinySkiaRenderer::new(800, 600)?;
    renderer.render_wit_layers(&wit_result.layers);

    let app = DemoApp::new("Deneb - Scatter Chart (WASM)", 800, 600);
    app.run(renderer.pixmap().clone())
}
