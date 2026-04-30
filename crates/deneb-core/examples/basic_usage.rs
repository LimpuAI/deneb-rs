// deneb-rs 基础使用示例
//
// 展示 deneb-core 的基本功能，包括数据创建、比例尺使用和绘图指令生成。

use deneb_core::{
    Column, DataType, DataTable, FieldValue, LinearScale, BandScale, DrawCmd, Scale,
    FillStyle, StrokeStyle, TextStyle, TextAnchor, RenderOutput, RenderLayers, LayerKind,
};

fn main() {
    println!("🌟 deneb-rs 基础使用示例\n");

    // 1. 创建数据表
    println!("📊 创建数据表...");
    let mut table = DataTable::new();

    // 添加类别列
    let categories = vec![
        FieldValue::Text("A".to_string()),
        FieldValue::Text("B".to_string()),
        FieldValue::Text("C".to_string()),
        FieldValue::Text("D".to_string()),
    ];
    table.add_column(Column::new("category", DataType::Nominal, categories));

    // 添加数值列
    let values = vec![
        FieldValue::Numeric(10.0),
        FieldValue::Numeric(25.0),
        FieldValue::Numeric(15.0),
        FieldValue::Numeric(30.0),
    ];
    table.add_column(Column::new("value", DataType::Quantitative, values));

    println!("  ✓ 数据表创建成功: {} 行 x {} 列", table.row_count(), table.column_count());

    // 2. 创建比例尺
    println!("\n📏 创建比例尺...");

    // 线性比例尺 (用于数值)
    let y_scale = LinearScale::new(0.0, 40.0, 300.0, 0.0);
    println!("  ✓ Y 轴线性比例尺: domain {:?}, range {:?}", y_scale.domain(), y_scale.range());

    // 条形比例尺 (用于类别)
    let x_scale = BandScale::new(
        vec!["A".to_string(), "B".to_string(), "C".to_string(), "D".to_string()],
        50.0,
        450.0,
        0.1,
    );
    println!("  ✓ X 轴条形比例尺: step_width={}", x_scale.step_width());

    // 3. 绘制柱状图
    println!("\n🎨 绘制柱状图...");

    let mut commands = Vec::new();

    // 添加背景
    commands.push(DrawCmd::Rect {
        x: 0.0,
        y: 0.0,
        width: 500.0,
        height: 350.0,
        fill: Some(FillStyle::Color("#f8f9fa".to_string())),
        stroke: None,
        corner_radius: None,
    });

    // 为每个类别绘制柱子
    for (_i, row) in (0..table.row_count()).enumerate() {
        if let (Some(FieldValue::Text(category)), Some(FieldValue::Numeric(value))) = (
            table.get_column("category").and_then(|c| c.get(row)),
            table.get_column("value").and_then(|c| c.get(row)),
        ) {
            let cat_str = category.as_str();
            let val_num = *value;

            // 计算位置和大小
            let x = x_scale.band_start(cat_str).unwrap_or(0.0);
            let y = y_scale.map(val_num);
            let width = x_scale.band_width();
            let height = 300.0 - y;

            // 绘制柱子
            commands.push(DrawCmd::Rect {
                x,
                y,
                width,
                height,
                fill: Some(FillStyle::Color("#4c8bf5".to_string())),
                stroke: Some(StrokeStyle::Color("#2c5bb5".to_string())),
                corner_radius: Some(4.0),
            });
        }
    }

    // 添加标题
    let title = DrawCmd::Text {
        x: 250.0,
        y: 30.0,
        content: "deneb-rs 示例图表".to_string(),
        style: TextStyle::new()
            .with_font_size(20.0)
            .with_font_weight(deneb_core::FontWeight::Bold)
            .with_fill(FillStyle::Color("#333".to_string())),
        anchor: TextAnchor::Middle,
        baseline: deneb_core::TextBaseline::Alphabetic,
    };
    commands.push(title);

    println!("  ✓ 生成了 {} 个绘图指令", commands.len());

    // 4. 创建渲染输出
    let output = RenderOutput::from_commands(commands.clone());
    println!("\n🖼️  渲染输出:");
    println!("  ✓ 语义指令数: {}", output.semantic.len());
    println!("  ✓ Canvas 操作数: {}", output.canvas_ops.len());

    // 5. 使用图层系统
    println!("\n📚 使用图层系统...");
    let mut layers = RenderLayers::new();

    // 更新数据层
    layers.update_layer(LayerKind::Data, output.clone());
    println!("  ✓ 数据层已更新，包含 {} 个指令", layers.get_layer(LayerKind::Data).unwrap().commands.len());

    // 标记所有层为干净
    layers.mark_all_clean();
    println!("  ✓ 所有层已标记为干净");

    // 检查脏层
    let dirty_count = layers.dirty_count();
    println!("  ✓ 当前脏层数: {}", dirty_count);

    // 6. 展示一些示例数据
    println!("\n📋 示例数据:");
    for i in 0..table.row_count() {
        if let (Some(cat), Some(val)) = (
            table.get_column("category").and_then(|c| c.get(i)),
            table.get_column("value").and_then(|c| c.get(i)),
        ) {
            println!("  {} - {}", cat, val);
        }
    }

    println!("\n✨ 示例运行完成！");
    println!("\n💡 提示: 这个示例展示了 deneb-core 的基本功能。");
    println!("   在实际应用中，RenderOutput 可以被发送到不同的渲染后端");
    println!("   (如 Canvas、SVG、WebGL 等) 来绘制实际的图形。");
}
