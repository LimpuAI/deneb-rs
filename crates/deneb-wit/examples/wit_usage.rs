//! deneb-wit 使用示例
//!
//! 展示 v2 有状态会话 API(ChartSession)的数据解析、渲染与交互。

use deneb_wit::session::ChartSession;
use deneb_wit::*;

fn main() -> Result<(), String> {
    // 示例 1: 解析 CSV 数据
    println!("=== 示例 1: 解析 CSV 数据 ===");
    let csv_data = b"category,value\nA,10\nB,20\nC,15\n4,25\nE,30";

    match parse_data(csv_data, "csv") {
        Ok(table) => {
            println!("成功解析 CSV 数据:");
            println!("  列数: {}", table.columns.len());
            println!("  行数: {}", table.rows.len());
            println!("  第一行数据: {:?}", table.rows.first());
        }
        Err(e) => println!("解析失败: {}", e),
    }

    // 示例 2: 创建会话(constructor + update-data)
    println!("\n=== 示例 2: 创建图表会话 ===");
    let spec = WitChartSpec {
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
        width: 800.0,
        height: 600.0,
        title: Some("示例柱状图".to_string()),
        animation: None,
    };
    let mut session = ChartSession::new(spec, None)?;
    session.update_data(csv_data, "csv")?;

    // 示例 3: 入场动画渲染(t 从 0 → 1)
    println!("\n=== 示例 3: 入场动画渲染 ===");
    let early = session.render(0.3)?;
    let settled = session.render(1.0)?;
    for layer in &settled.layers {
        println!("    - {} 层: {} 条指令 (dirty={})",
            layer.kind, layer.commands.len(), layer.dirty);
    }
    let early_height: f64 = early.layers.iter()
        .filter(|l| l.kind == "data")
        .flat_map(|l| l.commands.iter().map(|c| c.params[3]))
        .sum();
    let final_height: f64 = settled.layers.iter()
        .filter(|l| l.kind == "data")
        .flat_map(|l| l.commands.iter().map(|c| c.params[3]))
        .sum();
    println!("  柱高总和: t=0.3 时 {:.0}px → t=1.0 时 {:.0}px", early_height, final_height);

    // 示例 4: 命中区(datum 附着,宿主侧 AABB)
    println!("\n=== 示例 4: 命中区与交互 ===");
    let regions = session.hit_regions();
    println!("  命中区数量: {}", regions.len());
    if let Some(region) = regions.first() {
        let cx = region.bounds_x + region.bounds_w / 2.0;
        let cy = region.bounds_y + region.bounds_h / 2.0;
        match hit_test(&regions, cx, cy, 5.0) {
            Some(idx) => println!("  在 ({:.0}, {:.0}) 命中数据点 {} (datum={:?})", cx, cy, idx, region.datum.first()),
            None => println!("  未命中任何数据点"),
        }
    }

    // 示例 5: 选中交互(高亮 + 置灰过渡)
    println!("\n=== 示例 5: 选中交互 ===");
    session.set_state(WitInteractionState { hovered: None, selected: vec![1] });
    let _mid = session.render(0.5)?;   // 过渡中
    let done = session.render(1.0)?;   // 过渡完成:选中满色 + 其余 0.32
    let data = done.layers.iter().find(|l| l.kind == "data").unwrap();
    println!("  选中后 Data 层 {} 条指令(其中含 outline stroke)", data.commands.len());

    // 示例 6: JSON 序列化(协议类型可序列化)
    println!("\n=== 示例 6: JSON 序列化 ===");
    let spec_json = serde_json::to_string_pretty(&WitChartSpec {
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
        width: 600.0,
        height: 400.0,
        title: Some("示例折线图".to_string()),
        animation: None,
    }).unwrap();
    println!("图表规格 JSON(前 120 字符):\n{}", &spec_json[..120.min(spec_json.len())]);

    Ok(())
}
