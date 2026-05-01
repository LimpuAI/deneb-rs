//! 面积图演示（Parquet 格式）

use deneb_component::{AreaChart, ChartSpec, DefaultTheme, Encoding, Field, Mark};
use deneb_core::parser::parquet::parse_parquet;
use deneb_demo::{sample_data, DemoApp, TinySkiaRenderer, parse_wasm_args};
use deneb_demo::wasm_host::{ParserPaths, WasmHost};
use deneb_wit::wit_types::WitChartSpec;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let parquet_data = sample_data::area_chart_parquet();

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
        .mark(Mark::Area)
        .encoding(
            Encoding::new()
                .x(Field::quantitative("x"))
                .y(Field::quantitative("y1")),
        )
        .title("Area Chart Demo")
        .width(800.0)
        .height(600.0)
        .build()?;

    let theme = DefaultTheme;
    let output = AreaChart::render(&spec, &theme, &table)?;

    let mut renderer = TinySkiaRenderer::new(800, 600)?;
    renderer.render_layers(&output.layers);

    let app = DemoApp::new("Deneb - Area Chart", 800, 600);
    app.run(renderer.pixmap().clone())
}

fn run_wasm(host: &mut WasmHost, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let wit_spec = WitChartSpec {
        mark: "area".to_string(),
        x_field: "x".to_string(),
        y_field: "y1".to_string(),
        color_field: None,
        width: 800.0,
        height: 600.0,
        title: Some("Area Chart Demo (WASM)".to_string()),
        theme: None,
    };

    let wit_result = host.render(data, "parquet", &wit_spec)?;

    let mut renderer = TinySkiaRenderer::new(800, 600)?;
    renderer.render_wit_layers(&wit_result.layers);

    let app = DemoApp::new("Deneb - Area Chart (WASM)", 800, 600);
    app.run(renderer.pixmap().clone())
}
