//! ChartSpec builder API 使用示例
//!
//! 展示如何使用 deneb-component 的声明式 API 创建图表规格

use deneb_component::{
    ChartSpec, DarkTheme, DefaultTheme, Encoding, Field, Mark, Theme,
};

fn main() {
    // 示例 1: 创建简单的折线图规格
    let line_chart_spec = ChartSpec::builder()
        .mark(Mark::Line)
        .encoding(
            Encoding::new()
                .x(Field::temporal("date"))
                .y(Field::quantitative("value")),
        )
        .title("Sales Trend")
        .width(800.0)
        .height(400.0)
        .build()
        .expect("Failed to build chart spec");

    println!("=== Line Chart Spec ===");
    println!("Mark: {}", line_chart_spec.mark);
    println!("Title: {:?}", line_chart_spec.title);
    println!("Size: {} x {}", line_chart_spec.width, line_chart_spec.height);
    println!(
        "X Field: {} ({})",
        line_chart_spec.encoding.x.as_ref().unwrap().name,
        line_chart_spec.encoding.x.as_ref().unwrap().data_type
    );
    println!(
        "Y Field: {} ({})",
        line_chart_spec.encoding.y.as_ref().unwrap().name,
        line_chart_spec.encoding.y.as_ref().unwrap().data_type
    );

    // 示例 2: 带聚合的柱状图
    let bar_chart_spec = ChartSpec::builder()
        .mark(Mark::Bar)
        .encoding(
            Encoding::new()
                .x(Field::nominal("category"))
                .y(Field::quantitative("sales").with_aggregate(deneb_component::Aggregate::Sum)),
        )
        .title("Sales by Category")
        .width(600.0)
        .height(400.0)
        .build()
        .expect("Failed to build chart spec");

    println!("\n=== Bar Chart Spec ===");
    println!("Mark: {}", bar_chart_spec.mark);
    println!(
        "Y Aggregate: {:?}",
        bar_chart_spec.encoding.y.as_ref().unwrap().aggregate
    );

    // 示例 3: 使用主题
    let default_theme = DefaultTheme;
    let dark_theme = DarkTheme;

    println!("\n=== Theme Comparison ===");
    println!("Default Theme Background: {}", default_theme.background_color());
    println!("Dark Theme Background: {}", dark_theme.background_color());
    println!(
        "Default Palette (5): {:?}",
        default_theme.palette(5)
    );
    println!("Dark Palette (5): {:?}", dark_theme.palette(5));

    // 示例 4: 边距计算
    let padding = default_theme.margin();
    println!("\n=== Layout ===");
    println!("Padding: top={}, right={}, bottom={}, left={}",
             padding.top, padding.right, padding.bottom, padding.left);
    println!("Horizontal padding: {}", padding.horizontal());
    println!("Vertical padding: {}", padding.vertical());

    // 示例 5: 错误处理
    let invalid_spec = ChartSpec::builder()
        .encoding(
            Encoding::new()
                .x(Field::temporal("date"))
                .y(Field::quantitative("value")),
        )
        .build();

    match invalid_spec {
        Ok(_) => println!("Unexpected success"),
        Err(e) => println!("\n=== Expected Error ===\n{}", e),
    }

    // 示例 6: 尺寸验证
    let invalid_size = ChartSpec::builder()
        .mark(Mark::Scatter)
        .encoding(
            Encoding::new()
                .x(Field::quantitative("x"))
                .y(Field::quantitative("y")),
        )
        .width(-100.0)
        .build();

    match invalid_size {
        Ok(_) => println!("Unexpected success"),
        Err(e) => println!("=== Size Error ===\n{}", e),
    }

    println!("\n=== All examples completed successfully! ===");
}
