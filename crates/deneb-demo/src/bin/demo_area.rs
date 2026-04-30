//! 面积图演示

use deneb_component::{AreaChart, ChartSpec, DefaultTheme, Encoding, Field, Mark};
use deneb_core::parser::csv::parse_csv;
use deneb_demo::{DemoApp, TinySkiaRenderer};
use deneb_demo::wasm_host::WasmHost;
use deneb_wit::wit_types::WitChartSpec;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let csv = "x,y\n0,20\n1,35\n2,28\n3,45\n4,38\n5,55\n6,48\n7,65\n8,58\n9,72\n10,62\n11,78";
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
        .mark(Mark::Area)
        .encoding(
            Encoding::new()
                .x(Field::quantitative("x"))
                .y(Field::quantitative("y")),
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

fn run_wasm(wasm_path: &str, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let mut host = WasmHost::from_file(wasm_path)?;

    let wit_spec = WitChartSpec {
        mark: "area".to_string(),
        x_field: "x".to_string(),
        y_field: "y".to_string(),
        color_field: None,
        width: 800.0,
        height: 600.0,
        title: Some("Area Chart Demo (WASM)".to_string()),
        theme: None,
    };

    let wit_result = host.render(data, "csv", &wit_spec)?;

    let mut renderer = TinySkiaRenderer::new(800, 600)?;
    renderer.render_wit_layers(&wit_result.layers);

    let app = DemoApp::new("Deneb - Area Chart (WASM)", 800, 600);
    app.run(renderer.pixmap().clone())
}
