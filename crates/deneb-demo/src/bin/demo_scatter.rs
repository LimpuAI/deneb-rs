//! 散点图演示

use deneb_component::{ChartSpec, DefaultTheme, Encoding, Field, Mark, ScatterChart};
use deneb_core::parser::csv::parse_csv;
use deneb_demo::{sample_data, DemoApp, TinySkiaRenderer};
use deneb_demo::wasm_host::WasmHost;
use deneb_wit::wit_types::WitChartSpec;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let csv = sample_data::scatter_chart_csv();
    let args: Vec<String> = std::env::args().collect();

    let wasm_path = args.windows(2)
        .find(|w| w[0] == "--wasm")
        .map(|w| w[1].clone());

    if let Some(path) = wasm_path {
        run_wasm(&path, csv.as_bytes())?;
    } else {
        run_direct(csv)?;
    }

    Ok(())
}

fn run_direct(csv: &str) -> Result<(), Box<dyn std::error::Error>> {
    let table = parse_csv(csv)?;

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

    let theme = DefaultTheme;
    let output = ScatterChart::render(&spec, &theme, &table)?;

    let mut renderer = TinySkiaRenderer::new(800, 600)?;
    renderer.render_layers(&output.layers);

    let app = DemoApp::new("Deneb - Scatter Chart", 800, 600);
    app.run(renderer.pixmap().clone())
}

fn run_wasm(wasm_path: &str, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let mut host = WasmHost::from_file(wasm_path)?;

    let wit_spec = WitChartSpec {
        mark: "scatter".to_string(),
        x_field: "x".to_string(),
        y_field: "y".to_string(),
        color_field: Some("group".to_string()),
        width: 800.0,
        height: 600.0,
        title: Some("Scatter Chart Demo (WASM)".to_string()),
        theme: None,
    };

    let wit_result = host.render(data, "csv", &wit_spec)?;

    let mut renderer = TinySkiaRenderer::new(800, 600)?;
    renderer.render_wit_layers(&wit_result.layers);

    let app = DemoApp::new("Deneb - Scatter Chart (WASM)", 800, 600);
    app.run(renderer.pixmap().clone())
}
