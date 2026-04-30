//! deneb-demo: 桌面演示应用
//!
//! 使用 tiny-skia 将 deneb-rs 的 Canvas 2D 指令渲染到桌面窗口。

pub mod renderer;
pub mod sample_data;
pub mod wasm_host;
pub mod window;

pub use renderer::TinySkiaRenderer;
pub use window::DemoApp;
