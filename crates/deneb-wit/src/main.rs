//! deneb-wit 独立组件模式入口
//!
//! 作为独立 WASI 组件运行，通过 stdin/stdout 通信

fn main() {
    if let Err(e) = deneb_wit::component_mode::run_component() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
