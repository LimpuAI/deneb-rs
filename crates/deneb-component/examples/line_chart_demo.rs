use deneb_component::{ChartSpec, Encoding, Field, Mark, LineChart, DefaultTheme};
use deneb_core::{DataTable, Column, DataType, FieldValue};

fn main() {
    // 创建测试数据
    let data = DataTable::with_columns(vec![
        Column::new("x", DataType::Quantitative, vec![
            FieldValue::Numeric(0.0),
            FieldValue::Numeric(1.0),
            FieldValue::Numeric(2.0),
            FieldValue::Numeric(3.0),
            FieldValue::Numeric(4.0),
            FieldValue::Numeric(5.0),
        ]),
        Column::new("y", DataType::Quantitative, vec![
            FieldValue::Numeric(10.0),
            FieldValue::Numeric(25.0),
            FieldValue::Numeric(15.0),
            FieldValue::Numeric(30.0),
            FieldValue::Numeric(20.0),
            FieldValue::Numeric(35.0),
        ]),
    ]);

    // 创建图表规格
    let spec = ChartSpec::builder()
        .mark(Mark::Line)
        .encoding(
            Encoding::new()
                .x(Field::quantitative("x"))
                .y(Field::quantitative("y")),
        )
        .title("Line Chart Demo")
        .width(800.0)
        .height(400.0)
        .build()
        .expect("Failed to build chart spec");

    // 创建主题
    let theme = DefaultTheme;

    // 渲染折线图
    match LineChart::render(&spec, &theme, &data) {
        Ok(output) => {
            println!("Successfully rendered line chart!");
            println!("Total layers: {}", output.layers.all().len());
            println!("Dirty layers: {}", output.dirty_count());
            println!("Hit regions: {}", output.hit_regions.len());

            // 打印各层的指令数量
            for layer in output.layers.all() {
                println!("  Layer {:?}: {} commands", layer.kind, layer.commands.len());
            }
        }
        Err(e) => {
            eprintln!("Error rendering line chart: {}", e);
        }
    }
}
