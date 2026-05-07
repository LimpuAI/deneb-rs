//! 桑基图演示

use deneb_component::{
    SankeyChart, ChartSpec, DarkTheme, DefaultTheme, Encoding, Field, ForestTheme, Mark,
    NordicTheme, CappuccinoTheme,
};
use deneb_core::parser::csv::parse_csv;
use deneb_demo::{parse_theme_name, parse_wasm_args, render_and_show, sample_data, DemoApp,
    TinySkiaRenderer};
use deneb_demo::wasm_host::{ParserPaths, WasmHost};
use deneb_wit::wit_types::WitChartSpec;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let csv = sample_data::sankey_chart_csv();

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
        .mark(Mark::Sankey)
        .encoding(
            Encoding::new()
                .x(Field::nominal("source"))
                .y(Field::quantitative("flow")),
        )
        .title("Sankey Chart Demo")
        .width(800.0)
        .height(600.0)
        .build()?;

    let theme_name = parse_theme_name();
    match theme_name.as_deref() {
        Some("dark") => render_and_show(DarkTheme, |t| SankeyChart::render(&spec, t, &table), "Deneb - Sankey"),
        Some("forest") => render_and_show(ForestTheme, |t| SankeyChart::render(&spec, t, &table), "Deneb - Sankey"),
        Some("nordic") => render_and_show(NordicTheme, |t| SankeyChart::render(&spec, t, &table), "Deneb - Sankey"),
        Some("cappuccino") => render_and_show(CappuccinoTheme, |t| SankeyChart::render(&spec, t, &table), "Deneb - Sankey"),
        _ => render_and_show(DefaultTheme, |t| SankeyChart::render(&spec, t, &table), "Deneb - Sankey"),
    }
}

fn run_wasm(host: &mut WasmHost, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let wit_spec = WitChartSpec {
        mark: "sankey".to_string(),
        x_field: "source".to_string(),
        y_field: "flow".to_string(),
        color_field: None,
        open_field: None,
        high_field: None,
        low_field: None,
        close_field: None,
        theta_field: None,
        size_field: Some("flow".to_string()),
        width: 800.0,
        height: 600.0,
        title: Some("Sankey Chart Demo (WASM)".to_string()),
        theme: parse_theme_name(),
    };

    let wit_result = host.render(data, "csv", &wit_spec)?;

    let mut renderer = TinySkiaRenderer::new(800, 600)?;
    renderer.render_wit_layers(&wit_result.layers);

    let app = DemoApp::new("Deneb - Sankey (WASM)", 800, 600);
    app.run(renderer.pixmap().clone())
}
