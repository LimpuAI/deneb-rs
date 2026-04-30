use crate::layout::{compute_layout, LayoutResult, PlotArea};
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

/// 堆叠系列数据
struct StackedSeries {
    /// 行索引
    indices: Vec<usize>,
    /// 堆叠后的 y 值
    y_values: Vec<f64>,
}

/// AreaChart 渲染器
///
/// 负责将数据渲染为面积图，支持多系列堆叠、降采样、交互检测等功能。
pub struct AreaChart;

impl AreaChart {
    /// 渲染面积图
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

        // 5. 计算堆叠（如果有多系列）
        let stacked_series = Self::compute_stacked_values(&series, data, spec);

        // 6. 为每个系列生成面积路径
        let palette = theme.palette(stacked_series.len().max(1));
        let mut area_commands = Vec::new();
        let mut all_hit_regions = Vec::new();

        for (series_idx, stacked_data) in stacked_series.iter().enumerate() {
            let color = palette.get(series_idx % palette.len())
                .unwrap_or(&palette[0]);

            // 映射数据到像素坐标
            let points = Self::map_data_to_points(
                &stacked_data.indices,
                stacked_data.y_values.as_slice(),
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

            // 生成面积路径
            let baseline = y_scale.map(0.0); // Y 轴的 0 值位置
            let area_cmd = Self::generate_area_path(
                &downsampled_points,
                baseline,
                color,
                theme.default_stroke_width(),
            );
            area_commands.push(area_cmd);

            // 生成 HitRegion
            let hit_regions = Self::generate_hit_regions(
                &points,
                baseline,
                series_idx,
                &stacked_data.indices,
                data,
            );
            all_hit_regions.extend(hit_regions);
        }

        // 7. 生成轴指令（grid + axis layers）
        let (grid_commands, axis_commands) = Self::generate_axis_commands(
            &layout,
            theme,
        );

        // 8. 生成标题指令
        let title_commands = Self::generate_title_commands(spec, &layout.plot_area, theme);

        // 9. 生成背景指令
        let background_commands = Self::generate_background_commands(spec, theme);

        // 10. 组装 RenderLayers
        let mut layers = RenderLayers::new();

        // Background 层
        layers.update_layer(LayerKind::Background, background_commands);

        // Grid 层
        layers.update_layer(LayerKind::Grid, grid_commands);

        // Axis 层
        layers.update_layer(LayerKind::Axis, axis_commands);

        // Data 层
        let data_output = RenderOutput::from_commands(area_commands);
        layers.update_layer(LayerKind::Data, data_output);

        // Title 层
        layers.update_layer(LayerKind::Title, title_commands);

        Ok(ChartOutput {
            layers,
            hit_regions: all_hit_regions,
        })
    }

    /// 渲染空图表
    fn render_empty<T: Theme>(spec: &ChartSpec, theme: &T) -> ChartOutput {
        let mut layers = RenderLayers::new();

        // 生成背景
        let background_commands = Self::generate_background_commands(spec, theme);
        layers.update_layer(LayerKind::Background, background_commands);

        ChartOutput {
            layers,
            hit_regions: Vec::new(),
        }
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

        let scale: NumericScale = match x_field.data_type {
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
    ) -> Result<LinearScale, ComponentError> {
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

        Ok(LinearScale::new(min, max, range.0, range.1))
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

    /// 堆叠数据
    ///
    /// 返回每个系列的堆叠后的 y 值
    fn compute_stacked_values(
        series: &[(Option<String>, Vec<usize>)],
        data: &DataTable,
        spec: &ChartSpec,
    ) -> Vec<StackedSeries> {
        if series.len() <= 1 {
            // 单系列，不需要堆叠
            return series.iter().map(|(_, indices)| StackedSeries {
                indices: indices.clone(),
                y_values: indices.iter().map(|&i| {
                    Self::get_y_value(i, data, spec)
                }).collect(),
            }).collect();
        }

        // 多系列，计算堆叠
        let y_field = spec.encoding.y.as_ref().unwrap();
        let y_column = data.get_column(&y_field.name).unwrap();

        // 初始化累积值
        let mut accumulated: Vec<f64> = vec![0.0; data.row_count()];

        series.iter().map(|(_, indices)| {
            let mut stacked_y = Vec::new();

            for &i in indices {
                let y_value = y_column.values.get(i)
                    .and_then(|v| v.as_numeric())
                    .unwrap_or(0.0);

                let stacked = accumulated[i] + y_value;
                accumulated[i] = stacked;
                stacked_y.push(stacked);
            }

            StackedSeries {
                indices: indices.clone(),
                y_values: stacked_y,
            }
        }).collect()
    }

    /// 获取指定行的 y 值
    fn get_y_value(row_index: usize, data: &DataTable, spec: &ChartSpec) -> f64 {
        if let Some(y_field) = &spec.encoding.y {
            if let Some(column) = data.get_column(&y_field.name) {
                if let Some(value) = column.values.get(row_index) {
                    if let Some(num) = value.as_numeric() {
                        return num;
                    }
                }
            }
        }
        0.0
    }

    /// 将数据映射到像素坐标
    fn map_data_to_points(
        row_indices: &[usize],
        y_values: &[f64],
        data: &DataTable,
        x_scale: &NumericScale,
        y_scale: &LinearScale,
        spec: &ChartSpec,
    ) -> Result<Vec<(f64, f64)>, ComponentError> {
        let x_field = spec.encoding.x.as_ref()
            .ok_or_else(|| ComponentError::InvalidConfig {
                reason: "encoding.x is required".to_string(),
            })?;

        let x_column = data.get_column(&x_field.name)
            .ok_or_else(|| ComponentError::InvalidConfig {
                reason: format!("column '{}' not found", x_field.name),
            })?;

        let mut points = Vec::new();

        for (idx, &i) in row_indices.iter().enumerate() {
            let x_value = x_column.values.get(i)
                .ok_or_else(|| ComponentError::InvalidConfig {
                    reason: format!("row index {} out of bounds", i),
                })?;

            let x_num = x_value.as_numeric()
                .or_else(|| x_value.as_timestamp())
                .ok_or_else(|| ComponentError::InvalidConfig {
                    reason: format!("x value at row {} is not numeric", i),
                })?;

            let y_num = y_values.get(idx)
                .ok_or_else(|| ComponentError::InvalidConfig {
                    reason: format!("y value at index {} out of bounds", idx),
                })?;

            let px = x_scale.map(x_num);
            let py = y_scale.map(*y_num);

            points.push((px, py));
        }

        Ok(points)
    }

    /// 生成面积路径
    fn generate_area_path(
        points: &[(f64, f64)],
        baseline: f64,
        color: &str,
        stroke_width: f64,
    ) -> DrawCmd {
        if points.is_empty() {
            // 空路径
            return DrawCmd::Path {
                segments: Vec::new(),
                fill: None,
                stroke: None,
            };
        }

        if points.len() == 1 {
            // 单点退化为矩形
            let (x, y) = points[0];
            let width = stroke_width * 2.0;
            return DrawCmd::Rect {
                x: x - width / 2.0,
                y: y - width / 2.0,
                width,
                height: width,
                fill: Some(FillStyle::Color(color.to_string())),
                stroke: None,
                corner_radius: None,
            };
        }

        // 生成面积路径：上边界（数据点）→ 右下角基线 → 左下角基线 → 闭合
        let mut segments = Vec::new();

        // 上边界
        segments.push(PathSegment::MoveTo(points[0].0, points[0].1));
        for &(x, y) in &points[1..] {
            segments.push(PathSegment::LineTo(x, y));
        }

        // 右下角基线
        let last_x = points.last().unwrap().0;
        segments.push(PathSegment::LineTo(last_x, baseline));

        // 左下角基线
        let first_x = points[0].0;
        segments.push(PathSegment::LineTo(first_x, baseline));

        // 闭合
        segments.push(PathSegment::Close);

        // 生成半透明填充颜色
        let fill_color = Self::add_alpha(color, 0.3);

        DrawCmd::Path {
            segments,
            fill: Some(FillStyle::Color(fill_color)),
            stroke: Some(StrokeStyle::Color(color.to_string())),
        }
    }

    /// 为颜色添加 alpha 通道
    fn add_alpha(hex_color: &str, alpha: f64) -> String {
        // 解析 hex 颜色
        let hex = hex_color.trim_start_matches('#');

        if hex.len() == 6 {
            // RGB 格式
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
            format!("rgba({}, {}, {}, {:.2})", r, g, b, alpha)
        } else {
            // 其他格式（如已有 alpha），返回原色
            hex_color.to_string()
        }
    }

    /// 生成网格和坐标轴指令
    fn generate_axis_commands<T: Theme>(
        layout: &LayoutResult,
        theme: &T,
    ) -> (RenderOutput, RenderOutput) {
        let mut grid_commands = Vec::new();
        let mut axis_commands = Vec::new();

        // X 轴网格和刻度
        if let Some(x_axis) = &layout.x_axis {
            // 网格线（垂直线）
            for &pos in &x_axis.tick_positions {
                grid_commands.push(DrawCmd::Path {
                    segments: vec![
                        PathSegment::MoveTo(pos, layout.plot_area.y),
                        PathSegment::LineTo(pos, layout.plot_area.y + layout.plot_area.height),
                    ],
                    fill: None,
                    stroke: Some(theme.grid_stroke().clone()),
                });
            }

            // 轴线
            axis_commands.push(DrawCmd::Path {
                segments: vec![
                    PathSegment::MoveTo(layout.plot_area.x, x_axis.position),
                    PathSegment::LineTo(
                        layout.plot_area.x + layout.plot_area.width,
                        x_axis.position,
                    ),
                ],
                fill: None,
                stroke: Some(theme.axis_stroke().clone()),
            });

            // 刻度标签
            let tick_size = theme.tick_size();
            for (i, &pos) in x_axis.tick_positions.iter().enumerate() {
                if let Some(label) = x_axis.tick_labels.get(i) {
                    let text = DrawCmd::Text {
                        x: pos,
                        y: x_axis.position + tick_size + 2.0,
                        content: label.clone(),
                        style: TextStyle::new()
                            .with_font_size(theme.tick_font_size())
                            .with_font_family(theme.font_family())
                            .with_fill(FillStyle::Color(theme.foreground_color())),
                        anchor: TextAnchor::Middle,
                        baseline: TextBaseline::Top,
                    };
                    axis_commands.push(text);
                }
            }
        }

        // Y 轴网格和刻度
        if let Some(y_axis) = &layout.y_axis {
            // 网格线（水平线）
            for &pos in &y_axis.tick_positions {
                grid_commands.push(DrawCmd::Path {
                    segments: vec![
                        PathSegment::MoveTo(layout.plot_area.x, pos),
                        PathSegment::LineTo(layout.plot_area.x + layout.plot_area.width, pos),
                    ],
                    fill: None,
                    stroke: Some(theme.grid_stroke().clone()),
                });
            }

            // 轴线
            axis_commands.push(DrawCmd::Path {
                segments: vec![
                    PathSegment::MoveTo(y_axis.position, layout.plot_area.y),
                    PathSegment::LineTo(
                        y_axis.position,
                        layout.plot_area.y + layout.plot_area.height,
                    ),
                ],
                fill: None,
                stroke: Some(theme.axis_stroke().clone()),
            });

            // 刻度标签
            let tick_size = theme.tick_size();
            for (i, &pos) in y_axis.tick_positions.iter().enumerate() {
                if let Some(label) = y_axis.tick_labels.get(i) {
                    let text = DrawCmd::Text {
                        x: y_axis.position - tick_size - 2.0,
                        y: pos,
                        content: label.clone(),
                        style: TextStyle::new()
                            .with_font_size(theme.tick_font_size())
                            .with_font_family(theme.font_family())
                            .with_fill(FillStyle::Color(theme.foreground_color())),
                        anchor: TextAnchor::End,
                        baseline: TextBaseline::Middle,
                    };
                    axis_commands.push(text);
                }
            }
        }

        (
            RenderOutput::from_commands(grid_commands),
            RenderOutput::from_commands(axis_commands),
        )
    }

    /// 生成标题指令
    fn generate_title_commands<T: Theme>(
        spec: &ChartSpec,
        plot_area: &PlotArea,
        theme: &T,
    ) -> RenderOutput {
        if let Some(title) = &spec.title {
            let title_cmd = DrawCmd::Text {
                x: plot_area.x + plot_area.width / 2.0,
                y: plot_area.y - 10.0,
                content: title.clone(),
                style: TextStyle::new()
                    .with_font_size(theme.title_font_size())
                    .with_font_family(theme.font_family())
                    .with_font_weight(FontWeight::Bold)
                    .with_fill(FillStyle::Color(theme.foreground_color())),
                anchor: TextAnchor::Middle,
                baseline: TextBaseline::Bottom,
            };
            RenderOutput::from_commands(vec![title_cmd])
        } else {
            RenderOutput::new()
        }
    }

    /// 生成背景指令
    fn generate_background_commands<T: Theme>(
        spec: &ChartSpec,
        theme: &T,
    ) -> RenderOutput {
        let bg_cmd = DrawCmd::Rect {
            x: 0.0,
            y: 0.0,
            width: spec.width,
            height: spec.height,
            fill: Some(FillStyle::Color(theme.background_color())),
            stroke: None,
            corner_radius: None,
        };
        RenderOutput::from_commands(vec![bg_cmd])
    }

    /// 生成命中测试区域
    fn generate_hit_regions(
        points: &[(f64, f64)],
        baseline: f64,
        series_idx: usize,
        row_indices: &[usize],
        data: &DataTable,
    ) -> Vec<HitRegion> {
        let mut regions = Vec::new();

        for ((x, y), &row_idx) in points.iter().zip(row_indices.iter()) {
            // 获取该行的数据值
            let row_data: Vec<FieldValue> = data.columns
                .iter()
                .filter_map(|col| col.values.get(row_idx).cloned())
                .collect();

            // 使用矩形包围盒（从点到基线的区域）
            let height = (baseline - y).abs();
            let region = HitRegion::from_rect(
                x - 2.0, // 左右各扩展 2px
                y.min(baseline),
                4.0,
                height,
                row_idx,
                Some(series_idx),
                row_data,
            );
            regions.push(region);
        }

        regions
    }
}

#[cfg(test)]
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
            .mark(Mark::Area)
            .encoding(
                Encoding::new()
                    .x(crate::spec::Field::quantitative("x"))
                    .y(crate::spec::Field::quantitative("y")),
            )
            .title("Test Area Chart")
            .width(800.0)
            .height(400.0)
            .build()
            .unwrap()
    }

    #[test]
    fn test_area_chart_render_basic() {
        let spec = create_test_spec();
        let theme = crate::theme::DefaultTheme;
        let data = create_test_data();

        let result = AreaChart::render(&spec, &theme, &data);

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
    fn test_area_chart_render_empty() {
        let spec = create_test_spec();
        let theme = crate::theme::DefaultTheme;
        let data = DataTable::new();

        let result = AreaChart::render(&spec, &theme, &data);

        assert!(result.is_ok());
        let output = result.unwrap();

        // 验证只有背景层
        assert!(!output.layers.get_layer(LayerKind::Background).unwrap().commands.is_empty());
        assert!(output.layers.get_layer(LayerKind::Data).unwrap().commands.is_empty());
        assert!(output.hit_regions.is_empty());
    }

    #[test]
    fn test_area_chart_render_single_point() {
        let spec = create_test_spec();
        let theme = crate::theme::DefaultTheme;
        let data = DataTable::with_columns(vec![
            Column::new("x", DataType::Quantitative, vec![FieldValue::Numeric(0.0)]),
            Column::new("y", DataType::Quantitative, vec![FieldValue::Numeric(10.0)]),
        ]);

        let result = AreaChart::render(&spec, &theme, &data);

        assert!(result.is_ok());
        let output = result.unwrap();

        // 单点应该退化为矩形
        let data_layer = output.layers.get_layer(LayerKind::Data).unwrap();
        assert_eq!(data_layer.commands.len(), 1);
        if let DrawCmd::Rect { .. } = &data_layer.commands.semantic[0] {
            // 正确
        } else {
            panic!("Expected Rect for single point");
        }
    }

    #[test]
    fn test_area_chart_render_multi_series_stacked() {
        let spec = ChartSpec::builder()
            .mark(Mark::Area)
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
        let result = AreaChart::render(&spec, &theme, &data);

        assert!(result.is_ok());
        let output = result.unwrap();

        // 应该有两个堆叠的面积
        let data_layer = output.layers.get_layer(LayerKind::Data).unwrap();
        assert_eq!(data_layer.commands.len(), 2);
    }

    #[test]
    fn test_area_chart_add_alpha() {
        let color = "#4e79a7";
        let with_alpha = AreaChart::add_alpha(color, 0.3);
        assert_eq!(with_alpha, "rgba(78, 121, 167, 0.30)");

        let color2 = "#ff0000";
        let with_alpha2 = AreaChart::add_alpha(color2, 0.5);
        assert_eq!(with_alpha2, "rgba(255, 0, 0, 0.50)");
    }

    #[test]
    fn test_area_chart_downsample() {
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

        let result = AreaChart::render(&spec, &theme, &data);

        assert!(result.is_ok());
        let output = result.unwrap();

        // 应该成功渲染并降采样
        assert!(output.layers.get_layer(LayerKind::Data).unwrap().commands.len() > 0);
    }

    #[test]
    fn test_area_chart_hit_regions() {
        let spec = create_test_spec();
        let theme = crate::theme::DefaultTheme;
        let data = create_test_data();

        let result = AreaChart::render(&spec, &theme, &data).unwrap();

        // 验证命中区域
        assert_eq!(result.hit_regions.len(), 5);

        // 验证第一个命中区域
        let first_region = &result.hit_regions[0];
        assert_eq!(first_region.index, 0);
        assert_eq!(first_region.series, Some(0));

        // 验证命中区域包含正确的数据值
        assert_eq!(first_region.data.len(), 2); // 2 列
    }
}
