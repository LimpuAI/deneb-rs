//! deneb-wit 使用示例
//!
//! 展示如何使用 deneb-wit 的库模式 API 进行数据解析和图表渲染

use deneb_wit::*;

fn main() -> Result<(), String> {
    // 示例 1: 解析 CSV 数据
    println!("=== 示例 1: 解析 CSV 数据 ===");
    let csv_data = b"x,y\n1,10\n2,20\n3,15\n4,25\n5,30";

    match parse_data(csv_data, "csv") {
        Ok(table) => {
            println!("成功解析 CSV 数据:");
            println!("  列数: {}", table.columns.len());
            println!("  行数: {}", table.rows.len());
            println!("  第一行数据: {:?}", table.rows.first());
        }
        Err(e) => println!("解析失败: {}", e),
    }

    // 示例 2: 创建图表规格
    println!("\n=== 示例 2: 创建图表规格 ===");
    let spec = WitChartSpec {
        mark: "line".to_string(),
        x_field: "x".to_string(),
        y_field: "y".to_string(),
        color_field: None,
        open_field: None,
        high_field: None,
        low_field: None,
        close_field: None,
        theta_field: None,
        size_field: None,
        width: 800.0,
        height: 600.0,
        title: Some("示例折线图".to_string()),
        theme: None,
    };
    println!("图表规格: mark={}, 尺寸={}x{}", spec.mark, spec.width, spec.height);

    // 示例 3: 渲染图表
    println!("\n=== 示例 3: 渲染图表 ===");
    match render(csv_data, "csv", spec) {
        Ok(result) => {
            println!("成功渲染图表:");
            println!("  层数: {}", result.layers.len());
            for layer in &result.layers {
                println!("    - {} 层: {} 条指令, {} 个命中区域",
                    layer.kind, layer.commands.len(), layer.hit_regions.len());
            }

            // 示例 4: 命中测试
            println!("\n=== 示例 4: 命中测试 ===");
            if let Some(layer) = result.layers.iter().find(|l| l.kind == "data") {
                if let Some(region) = layer.hit_regions.first() {
                    let cx = region.bounds_x + region.bounds_w / 2.0;
                    let cy = region.bounds_y + region.bounds_h / 2.0;

                    match hit_test(&result, cx, cy, 5.0) {
                        Some(idx) => println!("  在中心点 ({}, {}) 命中数据点: {}", cx, cy, idx),
                        None => println!("  未命中任何数据点"),
                    }
                }
            }
        }
        Err(e) => println!("渲染失败: {}", e),
    }

    // 示例 5: JSON 序列化
    println!("\n=== 示例 5: JSON 序列化 ===");
    let spec_json = serde_json::to_string_pretty(&WitChartSpec {
        mark: "bar".to_string(),
        x_field: "category".to_string(),
        y_field: "value".to_string(),
        color_field: None,
        open_field: None,
        high_field: None,
        low_field: None,
        close_field: None,
        theta_field: None,
        size_field: None,
        width: 600.0,
        height: 400.0,
        title: Some("示例柱状图".to_string()),
        theme: None,
    }).unwrap();
    println!("图表规格 JSON:\n{}", spec_json);

    Ok(())
}
