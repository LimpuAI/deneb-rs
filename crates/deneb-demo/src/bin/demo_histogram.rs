//! 直方图演示

use deneb_component::{
    HistogramChart, ChartSpec, DarkTheme, DefaultTheme, Encoding, Field, ForestTheme, Mark,
    NordicTheme, CappuccinoTheme,
};
use deneb_core::parser::csv::parse_csv;
use deneb_demo::{parse_theme_name, parse_wasm_args, render_and_show, sample_data, DemoApp,
    TinySkiaRenderer};
use deneb_demo::wasm_host::{ParserPaths, WasmHost};
use deneb_wit::wit_types::WitChartSpec;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let csv = sample_data::histogram_chart_csv();

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
        .mark(Mark::Histogram)
        .encoding(
            Encoding::new()
                .x(Field::quantitative("value"))
                .y(Field::quantitative("value")),
        )
        .title("Histogram Demo")
        .width(800.0)
        .height(600.0)
        .build()?;

    let theme_name = parse_theme_name();
    match theme_name.as_deref() {
        Some("dark") => render_and_show(DarkTheme, |t| HistogramChart::render(&spec, t, &table), "Deneb - Histogram"),
        Some("forest") => render_and_show(ForestTheme, |t| HistogramChart::render(&spec, t, &table), "Deneb - Histogram"),
        Some("nordic") => render_and_show(NordicTheme, |t| HistogramChart::render(&spec, t, &table), "Deneb - Histogram"),
        Some("cappuccino") => render_and_show(CappuccinoTheme, |t| HistogramChart::render(&spec, t, &table), "Deneb - Histogram"),
        _ => render_and_show(DefaultTheme, |t| HistogramChart::render(&spec, t, &table), "Deneb - Histogram"),
    }
}

fn run_wasm(host: &mut WasmHost, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let wit_spec = WitChartSpec {
        mark: "histogram".to_string(),
        x_field: "value".to_string(),
        y_field: "value".to_string(),
        color_field: None,
        open_field: None,
        high_field: None,
        low_field: None,
        close_field: None,
        theta_field: None,
        size_field: None,
        width: 800.0,
        height: 600.0,
        title: Some("Histogram Demo (WASM)".to_string()),
        theme: parse_theme_name(),
    };

    let wit_result = host.render(data, "csv", &wit_spec)?;

    let mut renderer = TinySkiaRenderer::new(800, 600)?;
    renderer.render_wit_layers(&wit_result.layers);

    let app = DemoApp::new("Deneb - Histogram (WASM)", 800, 600);
    app.run(renderer.pixmap().clone())
}
