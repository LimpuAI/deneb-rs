//! deneb-demo: 桌面演示应用
//!
//! 使用 tiny-skia 将 deneb-rs 的 Canvas 2D 指令渲染到桌面窗口。

pub mod renderer;
pub mod sample_data;
pub mod wasm_host;
pub mod window;

pub use renderer::TinySkiaRenderer;
pub use window::DemoApp;

use deneb_component::{ChartOutput, ComponentError, Theme};

/// WASM 模式 CLI 参数
pub struct WasmArgs {
    pub wasm_path: String,
    pub deps_dir: Option<String>,
}

/// 从命令行参数解析 --wasm 和 --deps
pub fn parse_wasm_args() -> Option<WasmArgs> {
    let args: Vec<String> = std::env::args().collect();
    let wasm_path = args.windows(2)
        .find(|w| w[0] == "--wasm")
        .map(|w| w[1].clone())?;
    let deps_dir = args.windows(2)
        .find(|w| w[0] == "--deps")
        .map(|w| w[1].clone());
    Some(WasmArgs { wasm_path, deps_dir })
}

/// 从命令行参数解析 --theme
pub fn parse_theme_name() -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    args.windows(2)
        .find(|w| w[0] == "--theme")
        .map(|w| w[1].clone())
}

/// 通用渲染 + 展示辅助函数，接受任意 Theme 实现
pub fn render_and_show<T: Theme, F>(
    theme: T,
    render_fn: F,
    title: &str,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: FnOnce(&T) -> Result<ChartOutput, ComponentError>,
{
    let output = render_fn(&theme)?;
    let mut renderer = TinySkiaRenderer::new(800, 600)?;
    renderer.render_layers(&output.layers);
    let app = DemoApp::new(title, 800, 600);
    app.run(renderer.pixmap().clone())
}
