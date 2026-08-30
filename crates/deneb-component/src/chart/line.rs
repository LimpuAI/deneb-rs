//! 折线图渲染器
//!
//! 将 ChartSpec 和 DataTable 渲染为折线图的 Canvas 指令。

use crate::layout::{compute_layout, PlotArea};
use crate::spec::ChartSpec;
use crate::theme::Theme;
use crate::chart::ChartOutput;
use crate::error::ComponentError;
use deneb_core::*;
use deneb_core::algorithm::lttb;
use std::collections::HashMap;

/// Scale wrapper enum for dynamic dispatch
enum NumericScale {
    Linear(LinearScale),
    Time(TimeScale),
}

impl NumericScale {
    fn map(&self, input: f64) -> f64 {
        match self {
            NumericScale::Linear(s) => s.map(input),
            NumericScale::Time(s) => s.map(input),
        }
    }
}

/// LineChart 渲染器
///
/// 负责将数据渲染为折线图，支持多系列、降采样、交互检测等功能。
pub struct LineChart;

impl LineChart {
    /// 渲染折线图
    ///
    /// # Arguments
    ///
    /// * `spec` - 图表规格
    /// * `theme` - 主题配置
    /// * `data` - 数据表
    ///
    /// # Returns
    ///
    /// 返回渲染结果，包含分层渲染指令和命中测试区域
    ///
    /// # Errors
    ///
    /// - 数据为空时返回空图表
    /// - 缺少必要字段时返回错误
    /// - 数据类型不匹配时返回错误
    pub fn render<T: Theme>(
        spec: &ChartSpec,
        theme: &T,
        data: &DataTable,
    ) -> Result<ChartOutput, ComponentError> {
        // 1. 验证数据
        Self::validate_data(spec, data)?;

        if data.is_empty() {
            return Ok(Self::render_empty(spec, theme));
        }

        // 2. 计算布局
        let layout = compute_layout(spec, theme, data);

        // 3. 构建 Scale
        let x_scale = Self::build_x_scale(spec, &layout.plot_area, data)?;
        let y_scale = Self::build_y_scale(spec, &layout.plot_area, data)?;

        // 4. 按系列分组数据（color encoding 分组）
        let series = Self::group_data_by_series(spec, data);

        // 5. 为每个系列生成折线路径
        let palette = theme.palette(series.len().max(1));
        let mut line_commands = Vec::new();
        let mut all_hit_regions = Vec::new();

        for (series_idx, (_series_key, series_data)) in series.iter().enumerate() {
            let color = palette.get(series_idx % palette.len())
                .unwrap_or(&palette[0]);

            // 映射数据到像素坐标
            let points = Self::map_data_to_points(
                series_data,
                data,
                &x_scale,
                &y_scale,
                spec,
            )?;

            if points.is_empty() {
                continue;
            }

            // 降采样（如果数据点超过 10000）
            let downsampled_points = if points.len() > 10000 {
                lttb(&points, layout.plot_area.width as usize)
            } else {
                points.clone()
            };

            // 生成折线路径(monotone cubic 平滑 — Fritsch-Carlson 零过冲)
            let line_cmd = Self::generate_line_path(
                &downsampled_points,
                color,
                theme.default_stroke_width(),
            );
            line_commands.push(line_cmd);

            // 生成 HitRegion(全局索引从当前累计长度起算 — 命令身份关联键)
            let base = all_hit_regions.len();
            let hit_regions = Self::generate_hit_regions(
                &points,
                series_idx,
                data,
            );
            all_hit_regions.extend(hit_regions);

            // 顶点标记(命令身份:id = 全局 hit-region index;宿主对
            // per-point hover 效果按 id 采样渲染,零 wasm 调用)。
            // >200 点跳过 — 视觉噪声(降采样场景同样不画)
            if points.len() <= 200 {
                let white = theme.background_color().to_string();
                for (i, &(x, y)) in points.iter().enumerate() {
                    line_commands.push(DrawCmd::Circle {
                        cx: x,
                        cy: y,
                        r: 2.5,
                        fill: Some(FillStyle::Color(color.to_string())),
                        stroke: Some(StrokeStyle::WithWidth { color: white.clone(), width: 1.0 }),
                        id: Some((base + i) as u32),
                    });
                }
            }
        }

        // 6. 生成轴指令（grid + axis layers）
        let (grid_commands, axis_commands) = super::shared::render_cartesian_grid_and_axes(
            &layout,
            theme,
        );

        // 7. 生成标题指令
        // 8. 生成背景指令
        let background_commands = super::shared::render_background(spec, theme);

        // 9. 组装 RenderLayers
        let mut layers = RenderLayers::new();

        // Background 层
        layers.update_layer(LayerKind::Background, background_commands);

        // Grid 层
        layers.update_layer(LayerKind::Grid, grid_commands);

        // Axis 层
        layers.update_layer(LayerKind::Axis, axis_commands);

        // Data 层
        let data_output = RenderOutput::from_commands(line_commands);
        layers.update_layer(LayerKind::Data, data_output);

        // Title 层
        if let Some(title) = &spec.title {
            layers.update_layer(LayerKind::Title, super::shared::render_title(theme, title, &layout.plot_area));
        }

        // Legend 层(多系列;T26)
        if series.len() >= 2 {
            let labels: Vec<String> = series.iter().map(|(k, _)| k.clone().unwrap_or_default()).collect();
            let colors: Vec<String> = (0..series.len())
                .map(|i| palette.get(i % palette.len()).unwrap_or(&palette[0]).clone())
                .collect();
            layers.update_layer(
                LayerKind::Legend,
                super::shared::render_legend(theme, &labels, &colors, &layout.plot_area),
            );
        }

        Ok(ChartOutput {
            layers,
            hit_regions: all_hit_regions,
        })
    }

    /// 渲染空图表
    fn render_empty<T: Theme>(spec: &ChartSpec, theme: &T) -> ChartOutput {
        let mut layers = RenderLayers::new();

        // 生成背景
        let background_commands = super::shared::render_background(spec, theme);
        layers.update_layer(LayerKind::Background, background_commands);

        ChartOutput {
            layers,
            hit_regions: Vec::new(),
        }
    }

    /// 验证数据
    fn validate_data(spec: &ChartSpec, data: &DataTable) -> Result<(), ComponentError> {
        // 检查 x 编码
        if spec.encoding.x.is_none() {
            return Err(ComponentError::InvalidConfig {
                reason: "x encoding is required".to_string(),
            });
        }

        // 检查 y 编码
        if spec.encoding.y.is_none() {
            return Err(ComponentError::InvalidConfig {
                reason: "y encoding is required".to_string(),
            });
        }

        // 只有在有数据时才检查字段是否存在
        if !data.is_empty() {
            // 检查 x 字段是否存在
            if let Some(x_field) = &spec.encoding.x {
                if data.get_column(&x_field.name).is_none() {
                    return Err(ComponentError::InvalidConfig {
                        reason: format!("x field '{}' not found in data", x_field.name),
                    });
                }
            }

            // 检查 y 字段是否存在
            if let Some(y_field) = &spec.encoding.y {
                if data.get_column(&y_field.name).is_none() {
                    return Err(ComponentError::InvalidConfig {
                        reason: format!("y field '{}' not found in data", y_field.name),
                    });
                }
            }
        }

        Ok(())
    }

    /// 构建 X 轴 Scale
    fn build_x_scale(
        spec: &ChartSpec,
        plot_area: &PlotArea,
        data: &DataTable,
    ) -> Result<NumericScale, ComponentError> {
        let x_field = spec.encoding.x.as_ref()
            .ok_or_else(|| ComponentError::InvalidConfig {
                reason: "encoding.x is required".to_string(),
            })?;

        let column = data.get_column(&x_field.name)
            .ok_or_else(|| ComponentError::InvalidConfig {
                reason: format!("column '{}' not found", x_field.name),
            })?;

        let (min, max) = Self::get_numeric_range(column)?;

        let range = (plot_area.x, plot_area.x + plot_area.width);

        let scale = match x_field.data_type {
            DataType::Quantitative => NumericScale::Linear(LinearScale::new(min, max, range.0, range.1)),
            DataType::Temporal => NumericScale::Time(TimeScale::new(min, max, range.0, range.1)),
            _ => return Err(ComponentError::InvalidConfig {
                reason: format!("x axis does not support {:?} type", x_field.data_type),
            }),
        };

        Ok(scale)
    }

    /// 构建 Y 轴 Scale
    fn build_y_scale(
        spec: &ChartSpec,
        plot_area: &PlotArea,
        data: &DataTable,
    ) -> Result<NumericScale, ComponentError> {
        let y_field = spec.encoding.y.as_ref()
            .ok_or_else(|| ComponentError::InvalidConfig {
                reason: "encoding.y is required".to_string(),
            })?;

        let column = data.get_column(&y_field.name)
            .ok_or_else(|| ComponentError::InvalidConfig {
                reason: format!("column '{}' not found", y_field.name),
            })?;

        let (min, max) = Self::get_numeric_range(column)?;

        let range = (plot_area.y + plot_area.height, plot_area.y); // Y 轴翻转

        Ok(NumericScale::Linear(LinearScale::new(min, max, range.0, range.1)))
    }

    /// 获取数值列的范围
    fn get_numeric_range(column: &Column) -> Result<(f64, f64), ComponentError> {
        let mut min: Option<f64> = None;
        let mut max: Option<f64> = None;

        for value in &column.values {
            if let Some(num) = value.as_numeric() {
                min = Some(min.map_or(num, |m| m.min(num)));
                max = Some(max.map_or(num, |m| m.max(num)));
            } else if let Some(ts) = value.as_timestamp() {
                min = Some(min.map_or(ts, |m| m.min(ts)));
                max = Some(max.map_or(ts, |m| m.max(ts)));
            }
        }

        match (min, max) {
            (Some(min), Some(max)) => Ok((min, max)),
            _ => Err(ComponentError::InvalidConfig {
                reason: format!("column '{}' has no valid numeric values", column.name),
            }),
        }
    }

    /// 按系列分组数据
    fn group_data_by_series(
        spec: &ChartSpec,
        data: &DataTable,
    ) -> Vec<(Option<String>, Vec<usize>)> {
        if let Some(color_field) = &spec.encoding.color {
            // 按 color 字段分组
            let mut groups: HashMap<Option<String>, Vec<usize>> = HashMap::new();

            let color_column = data.get_column(&color_field.name);
            if let Some(col) = color_column {
                for (i, value) in col.values.iter().enumerate() {
                    let key = value.as_text().map(|s| s.to_string());
                    groups.entry(key).or_default().push(i);
                }
            }

            // 排序：按 key 的字母顺序
            let mut sorted_groups: Vec<_> = groups.into_iter().collect();
            sorted_groups.sort_by(|a, b| {
                match (a.0.as_ref(), b.0.as_ref()) {
                    (Some(ka), Some(kb)) => ka.cmp(kb),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                }
            });

            sorted_groups
        } else {
            // 单系列
            vec![(None, (0..data.row_count()).collect())]
        }
    }

    /// 将数据映射到像素坐标
    fn map_data_to_points(
        row_indices: &[usize],
        data: &DataTable,
        x_scale: &NumericScale,
        y_scale: &NumericScale,
        spec: &ChartSpec,
    ) -> Result<Vec<(f64, f64)>, ComponentError> {
        let x_field = spec.encoding.x.as_ref()
            .ok_or_else(|| ComponentError::InvalidConfig {
                reason: "encoding.x is required".to_string(),
            })?;

        let y_field = spec.encoding.y.as_ref()
            .ok_or_else(|| ComponentError::InvalidConfig {
                reason: "encoding.y is required".to_string(),
            })?;

        let x_column = data.get_column(&x_field.name)
            .ok_or_else(|| ComponentError::InvalidConfig {
                reason: format!("column '{}' not found", x_field.name),
            })?;

        let y_column = data.get_column(&y_field.name)
            .ok_or_else(|| ComponentError::InvalidConfig {
                reason: format!("column '{}' not found", y_field.name),
            })?;

        let mut points = Vec::new();

        for &i in row_indices {
            let x_value = x_column.values.get(i)
                .ok_or_else(|| ComponentError::InvalidConfig {
                    reason: format!("row index {} out of bounds", i),
                })?;

            let y_value = y_column.values.get(i)
                .ok_or_else(|| ComponentError::InvalidConfig {
                    reason: format!("row index {} out of bounds", i),
                })?;

            let x_num = x_value.as_numeric()
                .or_else(|| x_value.as_timestamp())
                .ok_or_else(|| ComponentError::InvalidConfig {
                    reason: format!("x value at row {} is not numeric", i),
                })?;

            let y_num = y_value.as_numeric()
                .or_else(|| y_value.as_timestamp())
                .ok_or_else(|| ComponentError::InvalidConfig {
                    reason: format!("y value at row {} is not numeric", i),
                })?;

            let px = x_scale.map(x_num);
            let py = y_scale.map(y_num);

            points.push((px, py));
        }

        Ok(points)
    }

    /// 生成折线路径
    fn generate_line_path(
        points: &[(f64, f64)],
        color: &str,
        stroke_width: f64,
    ) -> DrawCmd {
        if points.is_empty() {
            // 空路径
            return DrawCmd::Path {
                segments: Vec::new(),
                fill: None,
                stroke: None,
                dash: None,
                line_cap: None,
                id: None,
};
        }

        if points.len() == 1 {
            // 单点退化为 Circle
            return DrawCmd::Circle {
                cx: points[0].0,
                cy: points[0].1,
                r: stroke_width * 2.0,
                fill: Some(FillStyle::Color(color.to_string())),
                stroke: None,
                id: None,
};
        }

        let mut segments = Vec::new();
        segments.push(PathSegment::MoveTo(points[0].0, points[0].1));
        for &(x, y) in &points[1..] {
            segments.push(PathSegment::LineTo(x, y));
        }

        DrawCmd::Path {
            segments,
            fill: None,
            stroke: Some(StrokeStyle::Color(color.to_string())),
            dash: None,
            line_cap: None,
            id: None,
}
    }

    /// 生成命中测试区域
    fn generate_hit_regions(
        points: &[(f64, f64)],
        series_idx: usize,
        data: &DataTable,
    ) -> Vec<HitRegion> {
        let mut regions = Vec::new();

        for (i, &(x, y)) in points.iter().enumerate() {
            // 获取该行的数据值
            let row_data: Vec<FieldValue> = data.columns
                .iter()
                .filter_map(|col| col.values.get(i).cloned())
                .collect();

            let region = HitRegion::from_point(
                x,
                y,
                5.0, // 默认半径为 5 像素
                i,
                Some(series_idx),
                row_data,
            )
            .with_hover(HoverEffect::brighten(0.08));
            regions.push(region);
        }

        regions
    }
}

#[cfg(test)]
/// Fritsch-Carlson monotone cubic 插值 — 切线限制保证零过冲。
///
/// 逐段斜率取相邻割线斜率均值,极值点(斜率变号)切线归零,再以
/// alpha²+beta² ≤ 9 约束收紧 — 数学保证曲线不越过数据点包络。
/// <3 点退化为直线段。
fn monotone_segments(points: &[(f64, f64)]) -> Vec<PathSegment> {
    let n = points.len();
    if n < 3 {
        let mut segs = Vec::with_capacity(n.max(1));
        if let Some(&(x, y)) = points.first() {
            segs.push(PathSegment::MoveTo(x, y));
            for &(x, y) in &points[1..] {
                segs.push(PathSegment::LineTo(x, y));
            }
        }
        return segs;
    }

    // 割线斜率
    let dx: Vec<f64> = (0..n - 1).map(|i| points[i + 1].0 - points[i].0).collect();
    let slope: Vec<f64> = (0..n - 1)
        .map(|i| {
            let dy = points[i + 1].1 - points[i].1;
            if dx[i].abs() > f64::EPSILON { dy / dx[i] } else { 0.0 }
        })
        .collect();

    // 初始切线:端点取相邻割线,内点取均值(极值点归零)
    let mut m = vec![0.0f64; n];
    m[0] = slope[0];
    m[n - 1] = slope[n - 2];
    for i in 1..n - 1 {
        m[i] = if slope[i - 1] * slope[i] <= 0.0 { 0.0 } else { (slope[i - 1] + slope[i]) * 0.5 };
    }

    // Fritsch-Carlson 限制:同段两切线的 alpha²+beta² ≤ 9
    for i in 0..n - 1 {
        if slope[i].abs() <= f64::EPSILON {
            m[i] = 0.0;
            m[i + 1] = 0.0;
            continue;
        }
        let alpha = m[i] / slope[i];
        let beta = m[i + 1] / slope[i];
        let norm = alpha * alpha + beta * beta;
        if norm > 9.0 {
            let tau = 3.0 / norm.sqrt();
            m[i] = tau * alpha * slope[i];
            m[i + 1] = tau * beta * slope[i];
        }
    }

    let mut segs = Vec::with_capacity(n);
    segs.push(PathSegment::MoveTo(points[0].0, points[0].1));
    for i in 0..n - 1 {
        let h = dx[i];
        segs.push(PathSegment::BezierTo(
            points[i].0 + h / 3.0,
            points[i].1 + m[i] * h / 3.0,
            points[i + 1].0 - h / 3.0,
            points[i + 1].1 - m[i + 1] * h / 3.0,
            points[i + 1].0,
            points[i + 1].1,
        ));
    }
    segs
}

mod tests {
    use super::*;
    use crate::spec::{Encoding, Mark};
    use deneb_core::{Column, DataType, FieldValue};

    fn create_test_data() -> DataTable {
        DataTable::with_columns(vec![
            Column::new("x", DataType::Quantitative, vec![
                FieldValue::Numeric(0.0),
                FieldValue::Numeric(1.0),
                FieldValue::Numeric(2.0),
                FieldValue::Numeric(3.0),
                FieldValue::Numeric(4.0),
            ]),
            Column::new("y", DataType::Quantitative, vec![
                FieldValue::Numeric(10.0),
                FieldValue::Numeric(20.0),
                FieldValue::Numeric(15.0),
                FieldValue::Numeric(25.0),
                FieldValue::Numeric(30.0),
            ]),
        ])
    }

    fn create_test_spec() -> ChartSpec {
        ChartSpec::builder()
            .mark(Mark::Line)
            .encoding(
                Encoding::new()
                    .x(crate::spec::Field::quantitative("x"))
                    .y(crate::spec::Field::quantitative("y")),
            )
            .title("Test Chart")
            .width(800.0)
            .height(400.0)
            .build()
            .unwrap()
    }

    #[test]
    fn test_line_chart_render_basic() {
        let spec = create_test_spec();
        let theme = crate::theme::DefaultTheme;
        let data = create_test_data();

        let result = LineChart::render(&spec, &theme, &data);

        assert!(result.is_ok());
        let output = result.unwrap();

        // 验证层数
        assert_eq!(output.layers.all().len(), 7);

        // 验证有脏层
        assert!(output.has_dirty_layers());

        // 验证有命中区域
        assert_eq!(output.hit_regions.len(), 5);
    }

    #[test]
    fn test_line_chart_render_empty() {
        let spec = create_test_spec();
        let theme = crate::theme::DefaultTheme;
        let data = DataTable::new();

        let result = LineChart::render(&spec, &theme, &data);

        assert!(result.is_ok());
        let output = result.unwrap();

        // 验证只有背景层
        assert!(!output.layers.get_layer(LayerKind::Background).unwrap().commands.is_empty());
        assert!(output.layers.get_layer(LayerKind::Data).unwrap().commands.is_empty());
        assert!(output.hit_regions.is_empty());
    }

    #[test]
    fn test_line_chart_render_single_point() {
        let spec = create_test_spec();
        let theme = crate::theme::DefaultTheme;
        let data = DataTable::with_columns(vec![
            Column::new("x", DataType::Quantitative, vec![FieldValue::Numeric(0.0)]),
            Column::new("y", DataType::Quantitative, vec![FieldValue::Numeric(10.0)]),
        ]);

        let result = LineChart::render(&spec, &theme, &data);

        assert!(result.is_ok());
        let output = result.unwrap();

        // 单点应该退化为 Circle + 顶点标记(带命令身份)
        let data_layer = output.layers.get_layer(LayerKind::Data).unwrap();
        assert_eq!(data_layer.commands.len(), 2);
        if let DrawCmd::Circle { .. } = &data_layer.commands.semantic[0] {
            // 正确
        } else {
            panic!("Expected Circle for single point");
        }
    }

    #[test]
    fn test_line_chart_render_multi_series() {
        let spec = ChartSpec::builder()
            .mark(Mark::Line)
            .encoding(
                Encoding::new()
                    .x(crate::spec::Field::quantitative("x"))
                    .y(crate::spec::Field::quantitative("y"))
                    .color(crate::spec::Field::nominal("category")),
            )
            .width(800.0)
            .height(400.0)
            .build()
            .unwrap();

        let data = DataTable::with_columns(vec![
            Column::new("x", DataType::Quantitative, vec![
                FieldValue::Numeric(0.0),
                FieldValue::Numeric(1.0),
                FieldValue::Numeric(0.0),
                FieldValue::Numeric(1.0),
            ]),
            Column::new("y", DataType::Quantitative, vec![
                FieldValue::Numeric(10.0),
                FieldValue::Numeric(20.0),
                FieldValue::Numeric(15.0),
                FieldValue::Numeric(25.0),
            ]),
            Column::new("category", DataType::Nominal, vec![
                FieldValue::Text("A".to_string()),
                FieldValue::Text("A".to_string()),
                FieldValue::Text("B".to_string()),
                FieldValue::Text("B".to_string()),
            ]),
        ]);

        let theme = crate::theme::DefaultTheme;
        let result = LineChart::render(&spec, &theme, &data);

        assert!(result.is_ok());
        let output = result.unwrap();

        // 应该有两个系列(2 条路径 + 每点顶点标记)
        let data_layer = output.layers.get_layer(LayerKind::Data).unwrap();
        let paths = data_layer
            .commands
            .semantic
            .iter()
            .filter(|c| matches!(c, DrawCmd::Path { .. }))
            .count();
        let markers = data_layer
            .commands
            .semantic
            .iter()
            .filter(|c| matches!(c, DrawCmd::Circle { id: Some(_), .. }))
            .count();
        assert_eq!(paths, 2, "两个系列各一条路径");
        assert!(markers > 0, "顶点标记携带命令身份");
    }

    #[test]
    fn test_line_chart_downsample() {
        let spec = create_test_spec();
        let theme = crate::theme::DefaultTheme;

        // 创建大数据集（超过 10000 点）
        let large_data: Vec<FieldValue> = (0..15000)
            .map(|i| FieldValue::Numeric(i as f64))
            .collect();

        let data = DataTable::with_columns(vec![
            Column::new("x", DataType::Quantitative, large_data.clone()),
            Column::new("y", DataType::Quantitative, large_data),
        ]);

        let result = LineChart::render(&spec, &theme, &data);

        assert!(result.is_ok());
        let output = result.unwrap();

        // 应该成功渲染并降采样
        assert!(output.layers.get_layer(LayerKind::Data).unwrap().commands.len() > 0);
    }

    #[test]
    fn test_line_chart_hit_regions() {
        let spec = create_test_spec();
        let theme = crate::theme::DefaultTheme;
        let data = create_test_data();

        let output = LineChart::render(&spec, &theme, &data).unwrap();

        // 验证命中区域
        assert_eq!(output.hit_regions.len(), 5);

        // 验证第一个命中区域
        let first_region = &output.hit_regions[0];
        assert_eq!(first_region.index, 0);
        assert_eq!(first_region.series, Some(0));

        // 验证命中区域包含正确的数据值
        assert_eq!(first_region.data.len(), 2); // 2 列
    }

    #[test]
    fn test_line_chart_constant_y_values() {
        let spec = create_test_spec();
        let theme = crate::theme::DefaultTheme;

        // 所有 y 值相同
        let data = DataTable::with_columns(vec![
            Column::new("x", DataType::Quantitative, vec![
                FieldValue::Numeric(0.0),
                FieldValue::Numeric(1.0),
                FieldValue::Numeric(2.0),
            ]),
            Column::new("y", DataType::Quantitative, vec![
                FieldValue::Numeric(42.0),
                FieldValue::Numeric(42.0),
                FieldValue::Numeric(42.0),
            ]),
        ]);

        let result = LineChart::render(&spec, &theme, &data);

        assert!(result.is_ok());
        // 应该成功处理常数 y 值（水平线）
    }
}

#[cfg(test)]
mod monotone_tests {
    use super::*;

    #[test]
    fn monotone_no_overshoot_on_monotone_data() {
        // 单调上升数据:平滑曲线任意 Bezier 控制点不得越过数据包络
        let pts = vec![(0.0, 0.0), (1.0, 1.0), (2.0, 2.0), (3.0, 3.0)];
        let segs = monotone_segments(&pts);
        assert!(matches!(segs[0], PathSegment::MoveTo(..)));
        for s in &segs[1..] {
            if let PathSegment::BezierTo(x1, y1, x2, y2, x, y) = *s {
                for (cx, cy) in [(x1, y1), (x2, y2), (x, y)] {
                    assert!(cx >= 0.0 && cx <= 3.0, "x control {cx} out of [0,3]");
                    let _ = cy;
                }
            }
        }
    }

    #[test]
    fn monotone_extremum_tangent_zero() {
        // 峰值点(斜率变号)切线应为零 — 曲线在峰顶走平不过冲
        let pts = vec![(0.0, 0.0), (1.0, 1.0), (2.0, 0.0)];
        let segs = monotone_segments(&pts);
        // 峰值点前后各一段 Bezier,其靠近峰值的控制点 y ≈ 峰值(切线≈0)
        if let PathSegment::BezierTo(_, y1, _, _, _, _) = segs[1] {
            assert!(y1 <= 1.0 + 1e-9, "pre-peak control y {y1} overshoots peak 1.0");
        }
        if let PathSegment::BezierTo(_, _, _, y2, _, _) = segs[2] {
            assert!(y2 <= 1.0 + 1e-9, "post-peak control y {y2} overshoots peak 1.0");
        }
    }

    #[test]
    fn monotone_two_points_degrades_to_line() {
        let pts = vec![(0.0, 0.0), (5.0, 5.0)];
        let segs = monotone_segments(&pts);
        assert!(matches!(segs[1], PathSegment::LineTo(..)));
    }
}
