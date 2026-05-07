//! K 线图演示

use deneb_component::{
    CandlestickChart, ChartSpec, DarkTheme, DefaultTheme, Encoding, Field, ForestTheme, Mark,
    NordicTheme, CappuccinoTheme,
};
use deneb_core::parser::csv::parse_csv;
use deneb_demo::{parse_theme_name, parse_wasm_args, render_and_show, sample_data, DemoApp,
    TinySkiaRenderer};
use deneb_demo::wasm_host::{ParserPaths, WasmHost};
use deneb_wit::wit_types::WitChartSpec;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let csv = sample_data::candlestick_chart_csv();

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
        .mark(Mark::Candlestick)
        .encoding(
            Encoding::new()
                .x(Field::nominal("date"))
                .y(Field::quantitative("close"))
                .open(Field::quantitative("open"))
                .high(Field::quantitative("high"))
                .low(Field::quantitative("low"))
                .close(Field::quantitative("close")),
        )
        .title("Candlestick Chart Demo")
        .width(800.0)
        .height(600.0)
        .build()?;

    let theme_name = parse_theme_name();
    match theme_name.as_deref() {
        Some("dark") => render_and_show(DarkTheme, |t| CandlestickChart::render(&spec, t, &table), "Deneb - Candlestick"),
        Some("forest") => render_and_show(ForestTheme, |t| CandlestickChart::render(&spec, t, &table), "Deneb - Candlestick"),
        Some("nordic") => render_and_show(NordicTheme, |t| CandlestickChart::render(&spec, t, &table), "Deneb - Candlestick"),
        Some("cappuccino") => render_and_show(CappuccinoTheme, |t| CandlestickChart::render(&spec, t, &table), "Deneb - Candlestick"),
        _ => render_and_show(DefaultTheme, |t| CandlestickChart::render(&spec, t, &table), "Deneb - Candlestick"),
    }
}

fn run_wasm(host: &mut WasmHost, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let wit_spec = WitChartSpec {
        mark: "candlestick".to_string(),
        x_field: "date".to_string(),
        y_field: "close".to_string(),
        color_field: None,
        open_field: Some("open".to_string()),
        high_field: Some("high".to_string()),
        low_field: Some("low".to_string()),
        close_field: Some("close".to_string()),
        theta_field: None,
        size_field: None,
        width: 800.0,
        height: 600.0,
        title: Some("Candlestick Chart Demo (WASM)".to_string()),
        theme: parse_theme_name(),
    };

    let wit_result = host.render(data, "csv", &wit_spec)?;

    let mut renderer = TinySkiaRenderer::new(800, 600)?;
    renderer.render_wit_layers(&wit_result.layers);

    let app = DemoApp::new("Deneb - Candlestick (WASM)", 800, 600);
    app.run(renderer.pixmap().clone())
}
