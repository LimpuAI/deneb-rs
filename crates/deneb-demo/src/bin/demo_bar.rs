//! 柱状图演示

use deneb_component::{BarChart, ChartSpec, DefaultTheme, Encoding, Field, Mark};
use deneb_core::parser::csv::parse_csv;
use deneb_demo::{sample_data, DemoApp, TinySkiaRenderer, parse_wasm_args};
use deneb_demo::wasm_host::{ParserPaths, WasmHost};
use deneb_wit::wit_types::WitChartSpec;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let csv = sample_data::bar_chart_csv();

    if let Some(args) = parse_wasm_args() {
        let parsers = args.deps_dir.as_deref()
            .map(ParserPaths::from_dir)
            .unwrap_or_default();
        let mut host = WasmHost::from_file_with_parsers(&args.wasm_path, parsers)?;
        run_wasm(&mut host, csv.as_bytes())?;
    } else {
        run_direct(csv)?;
    }

    Ok(())
}

fn run_direct(csv: &str) -> Result<(), Box<dyn std::error::Error>> {
    let table = parse_csv(csv)?;

    let spec = ChartSpec::builder()
        .mark(Mark::Bar)
        .encoding(
            Encoding::new()
                .x(Field::nominal("category"))
                .y(Field::quantitative("value")),
        )
        .title("Bar Chart Demo")
        .width(800.0)
        .height(600.0)
        .build()?;

    let theme = DefaultTheme;
    let output = BarChart::render(&spec, &theme, &table)?;

    let mut renderer = TinySkiaRenderer::new(800, 600)?;
    renderer.render_layers(&output.layers);

    let app = DemoApp::new("Deneb - Bar Chart", 800, 600);
    app.run(renderer.pixmap().clone())
}

fn run_wasm(host: &mut WasmHost, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let wit_spec = WitChartSpec {
        mark: "bar".to_string(),
        x_field: "category".to_string(),
        y_field: "value".to_string(),
        color_field: None,
        width: 800.0,
        height: 600.0,
        title: Some("Bar Chart Demo (WASM)".to_string()),
        theme: None,
    };

    let wit_result = host.render(data, "csv", &wit_spec)?;

    let mut renderer = TinySkiaRenderer::new(800, 600)?;
    renderer.render_wit_layers(&wit_result.layers);

    let app = DemoApp::new("Deneb - Bar Chart (WASM)", 800, 600);
    app.run(renderer.pixmap().clone())
}
