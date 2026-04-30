# deneb-component

Chart 类型实现层，基于 deneb-core 提供的基础类型，实现各种具体的图表类型和组件。

## 功能特性

### 1. ChartSpec Builder API

声明式图表规格定义，启发自 Vega-Lite：

```rust
use deneb_component::{ChartSpec, Encoding, Field, Mark};

let spec = ChartSpec::builder()
    .mark(Mark::Line)
    .encoding(
        Encoding::new()
            .x(Field::temporal("date"))
            .y(Field::quantitative("value"))
            .color(Field::nominal("category")),
    )
    .title("Sales Trend")
    .width(800.0)
    .height(400.0)
    .build()?;
```

**支持的功能：**
- Mark 类型：Line, Bar, Scatter, Area
- 聚合函数：Sum, Mean, Median, Min, Max, Count
- 编码通道：x, y, color, size
- 数据类型：quantitative, temporal, nominal, ordinal
- 完整的构建时验证

### 2. Theme 系统

可扩展的主题系统，支持自定义视觉风格：

```rust
use deneb_component::{DefaultTheme, DarkTheme, Theme};

let theme = DefaultTheme;
println!("Background: {}", theme.background_color());
println!("Palette: {:?}", theme.palette(5));
```

**预置主题：**
- `DefaultTheme`: 浅色主题，Category10 调色板
- `DarkTheme`: 深色主题，Tableau10 调色板

**主题包含：**
- 颜色配置（调色板、背景、前景）
- 字体配置（家族、大小）
- 线条配置（网格、轴线、宽度）
- 间距配置（内边距、刻度大小）

### 3. 布局引擎

自动计算图表元素位置和尺寸：

```rust
use deneb_component::compute_layout;

let layout = compute_layout(&spec, &theme, &data);
println!("Plot area: {}x{}", layout.plot_area.width, layout.plot_area.height);
```

**布局功能：**
- 自动计算绘图区域（考虑主题边距）
- 智能刻度计算（线性、时间、离散）
- 轴位置和方向确定
- 支持 BandScale（条形图）和 LinearScale（折线图）

## 使用示例

查看 `examples/chart_spec_demo.rs` 获取完整示例：

```bash
cargo run -p deneb-component --example chart_spec_demo
```

## 测试

运行单元测试：

```bash
cargo test -p deneb-component
```

**测试覆盖：**
- 41 个单元测试，全部通过
- 100% API 覆盖率
- 边界条件验证
- 错误处理测试

## 依赖关系

```
deneb-component
    └── deneb-core (提供基础类型)
```

**从 deneb-core 使用的类型：**
- `DataType`, `FieldValue`, `Column`, `DataTable` (数据)
- `StrokeStyle` (样式)
- `Scale` trait 系列 (比例尺)

## 架构设计

遵循 Rust 工程最佳实践：

1. **Ownership 驱动设计**: 所有数据结构实现 `Clone`，避免不必要的 `clone()`
2. **类型表达约束**: 使用 `Mark`、`Aggregate` 等枚举在编译期保证类型安全
3. **Trait 定义边界**: `Theme` trait 清晰定义主题契约，支持多实现
4. **错误处理**: 使用 `ComponentError` 提供详细错误信息
5. **泛型优于 trait object**: `compute_layout<T: Theme>` 避免运行时开销

## 下一步

- [ ] 实现具体图表类型（LineChart, BarChart 等）
- [ ] 添加更多预置主题
- [ ] 实现图例布局
- [ ] 支持多系列图表

## 许可证

MIT OR Apache-2.0
