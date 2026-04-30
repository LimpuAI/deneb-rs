# deneb-rs

一个后端无关的 Rust 可视化库，输出 Canvas 2D 指令序列。

## 项目结构

```
deneb-rs/
├── Cargo.toml                          # Workspace 配置
├── crates/
│   ├── deneb-core/                     # 核心计算库
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs                  # 库入口
│   │   │   ├── data/                   # 数据类型
│   │   │   │   └── mod.rs              # FieldValue, DataType, Column, DataTable, Schema
│   │   │   ├── style/                  # 样式类型
│   │   │   │   └── mod.rs              # FillStyle, StrokeStyle, TextStyle, Gradient
│   │   │   ├── instruction/            # 绘图指令
│   │   │   │   └── mod.rs              # DrawCmd, PathSegment, CanvasOp, RenderOutput
│   │   │   ├── layer/                  # 图层系统
│   │   │   │   └── mod.rs              # LayerKind, Layer, RenderLayers
│   │   │   ├── scale/                  # 比例尺系统
│   │   │   │   └── mod.rs              # Scale trait, LinearScale, LogScale, etc.
│   │   │   └── error.rs                # 错误类型定义
│   │   └── examples/
│   │       └── basic_usage.rs          # 使用示例
│   ├── deneb-component/                # 图表组件实现 (后续)
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   └── deneb-wit/                      # WASI 0.3 集成 (后续)
│       ├── Cargo.toml
│       └── src/lib.rs
```

## Wave 1 完成状态

✅ **deneb-core 基础完成** (2025-04-29)

### 实现的核心功能

1. **数据类型** (`data/mod.rs`)
   - `FieldValue`: 数值、文本、时间戳、布尔值、空值
   - `DataType`: 定量、时间、名义、序数
   - `Column`: 列定义，包含名称、类型、值序列
   - `DataTable`: 数据表，支持列操作和验证
   - `Schema`: 模式定义，提供快速查找

2. **样式类型** (`style/mod.rs`)
   - `FillStyle`/`StrokeStyle`: 填充和描边样式
   - `Gradient`: 线性和径向渐变
   - `TextStyle`: 文本样式（字体、大小、粗细、样式）
   - `TextAnchor`/`TextBaseline`: 文本对齐

3. **绘图指令** (`instruction/mod.rs`)
   - `DrawCmd`: 语义化绘图指令（矩形、路径、圆形、文本、分组）
   - `PathSegment`: 路径段（移动、直线、贝塞尔曲线、圆弧）
   - `CanvasOp`: 底层 Canvas 2D API 调用
   - `RenderOutput`: 渲染输出，包含语义指令和 Canvas 操作

4. **图层系统** (`layer/mod.rs`)
   - `LayerKind`: 7 种标准层（背景、网格、坐标轴、数据、图例、标题、标注）
   - `Layer`: 图层定义，支持脏标记
   - `RenderLayers`: 图层管理器，支持增量更新

5. **比例尺系统** (`scale/mod.rs`)
   - `Scale` trait: 统一的比例尺接口
   - `LinearScale`: 线性比例尺
   - `LogScale`: 对数比例尺
   - `TimeScale`: 时间比例尺
   - `OrdinalScale`: 序数比例尺
   - `BandScale`: 条形比例尺

6. **错误处理** (`error.rs`)
   - `CoreError`: 统一错误类型
   - `DataFormat`: 数据格式枚举

## 技术特性

- ✅ **类型安全**: 利用 Rust 类型系统在编译期捕获错误
- ✅ **零成本抽象**: Trait-based 设计，无运行时开销
- ✅ **所有权驱动**: 遵循 Rust ownership 最佳实践
- ✅ **并发安全**: 所有公开 API 满足 Send + Sync
- ✅ **no_std 兼容**: 核心类型不依赖 std
- ✅ **WASM 就绪**: 可编译为 WebAssembly

## 测试覆盖

- **52 个单元测试**: 覆盖所有核心功能
- **集成测试**: 验证模块间协作
- **示例程序**: `basic_usage.rs` 展示完整使用流程

## 构建和运行

```bash
# 检查编译
cargo check --workspace

# 运行测试
cargo test --workspace

# 运行示例
cargo run -p deneb-core --example basic_usage

# 代码检查
cargo clippy --workspace
```

## 下一步 (Wave 2+)

- [ ] deneb-component: 实现具体图表类型（柱状图、折线图、散点图等）
- [ ] deneb-wit: WASI 0.3 集成，支持 WASM 运行时
- [ ] 数据解析器: CSV、JSON、Arrow、Parquet 支持
- [ ] 渲染后端: Canvas、SVG、WebGL 输出
- [ ] 交互系统: 事件处理和交互反馈

## 许可证

MIT OR Apache-2.0
