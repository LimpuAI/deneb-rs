//! deneb-demo: 桌面演示应用
//!
//! 使用 tiny-skia 将 deneb-rs 的 Canvas 2D 指令渲染到桌面窗口。

pub mod renderer;
pub mod sample_data;
pub mod wasm_host;
pub mod window;

pub use renderer::TinySkiaRenderer;
pub use window::DemoApp;

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
