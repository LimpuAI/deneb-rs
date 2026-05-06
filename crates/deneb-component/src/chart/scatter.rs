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

/// ScatterChart 渲染器
///
/// 负责将数据渲染为散点图，支持多系列、大小编码、降采样、交互检测等功能。
pub struct ScatterChart;

impl ScatterChart {
    /// 渲染散点图
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

        // 5. 构建 size scale（如果有 size encoding）
        let size_scale = Self::build_size_scale(spec, data)?;

        // 6. 为每个系列生成散点
        let palette = theme.palette(series.len().max(1));
        let mut scatter_commands = Vec::new();
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

            // 生成散点
            for ((x, y), original_idx) in downsampled_points.iter().zip(series_data.iter()) {
                let radius = Self::get_point_radius(*original_idx, spec, data, &size_scale);

                let circle = DrawCmd::Circle {
                    cx: *x,
                    cy: *y,
                    r: radius,
                    fill: Some(FillStyle::Color(color.to_string())),
                    stroke: None,
                };
                scatter_commands.push(circle);

                // 生成 HitRegion
                let row_data: Vec<FieldValue> = data.columns
                    .iter()
                    .filter_map(|col| col.values.get(*original_idx).cloned())
                    .collect();

                let region = HitRegion::from_point(
                    *x,
                    *y,
                    radius,
                    *original_idx,
                    Some(series_idx),
                    row_data,
                );
                all_hit_regions.push(region);
            }
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
        let data_output = RenderOutput::from_commands(scatter_commands);
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

    /// 构建 Size Scale（如果有 size encoding）
    fn build_size_scale(
        spec: &ChartSpec,
        data: &DataTable,
    ) -> Result<Option<LinearScale>, ComponentError> {
        if let Some(size_field) = &spec.encoding.size {
            let column = data.get_column(&size_field.name)
                .ok_or_else(|| ComponentError::InvalidConfig {
                    reason: format!("column '{}' not found", size_field.name),
                })?;

            let (min, max) = Self::get_numeric_range(column)?;

            // 半径范围：2px 到 10px
            Ok(Some(LinearScale::new(min, max, 2.0, 10.0)))
        } else {
            Ok(None)
        }
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
        y_scale: &LinearScale,
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

    /// 获取点的半径
    fn get_point_radius(
        row_index: usize,
        spec: &ChartSpec,
        data: &DataTable,
        size_scale: &Option<LinearScale>,
    ) -> f64 {
        if let Some(scale) = size_scale {
            if let Some(size_field) = &spec.encoding.size {
                if let Some(column) = data.get_column(&size_field.name) {
                    if let Some(value) = column.values.get(row_index) {
                        if let Some(num) = value.as_numeric() {
                            return scale.map(num);
                        }
                    }
                }
            }
        }

        // 默认半径 4px
        4.0
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
            let tick_size = theme.layout_config().tick_length;
            for (i, &pos) in x_axis.tick_positions.iter().enumerate() {
                if let Some(label) = x_axis.tick_labels.get(i) {
                    let text = DrawCmd::Text {
                        x: pos,
                        y: x_axis.position + tick_size + 2.0,
                        content: label.clone(),
                        style: TextStyle::new()
                            .with_font_size(theme.tick_font_size())
                            .with_font_family(theme.font_family())
                            .with_fill(FillStyle::Color(theme.foreground_color().to_string())),
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
            let tick_size = theme.layout_config().tick_length;
            for (i, &pos) in y_axis.tick_positions.iter().enumerate() {
                if let Some(label) = y_axis.tick_labels.get(i) {
                    let text = DrawCmd::Text {
                        x: y_axis.position - tick_size - 2.0,
                        y: pos,
                        content: label.clone(),
                        style: TextStyle::new()
                            .with_font_size(theme.tick_font_size())
                            .with_font_family(theme.font_family())
                            .with_fill(FillStyle::Color(theme.foreground_color().to_string())),
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
                    .with_fill(FillStyle::Color(theme.title_color().to_string())),
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
            fill: Some(FillStyle::Color(theme.background_color().to_string())),
            stroke: None,
            corner_radius: None,
        };
        RenderOutput::from_commands(vec![bg_cmd])
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
            .mark(Mark::Scatter)
            .encoding(
                Encoding::new()
                    .x(crate::spec::Field::quantitative("x"))
                    .y(crate::spec::Field::quantitative("y")),
            )
            .title("Test Scatter Chart")
            .width(800.0)
            .height(400.0)
            .build()
            .unwrap()
    }

    #[test]
    fn test_scatter_chart_render_basic() {
        let spec = create_test_spec();
        let theme = crate::theme::DefaultTheme;
        let data = create_test_data();

        let result = ScatterChart::render(&spec, &theme, &data);

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
    fn test_scatter_chart_render_empty() {
        let spec = create_test_spec();
        let theme = crate::theme::DefaultTheme;
        let data = DataTable::new();

        let result = ScatterChart::render(&spec, &theme, &data);

        assert!(result.is_ok());
        let output = result.unwrap();

        // 验证只有背景层
        assert!(!output.layers.get_layer(LayerKind::Background).unwrap().commands.is_empty());
        assert!(output.layers.get_layer(LayerKind::Data).unwrap().commands.is_empty());
        assert!(output.hit_regions.is_empty());
    }

    #[test]
    fn test_scatter_chart_render_single_point() {
        let spec = create_test_spec();
        let theme = crate::theme::DefaultTheme;
        let data = DataTable::with_columns(vec![
            Column::new("x", DataType::Quantitative, vec![FieldValue::Numeric(0.0)]),
            Column::new("y", DataType::Quantitative, vec![FieldValue::Numeric(10.0)]),
        ]);

        let result = ScatterChart::render(&spec, &theme, &data);

        assert!(result.is_ok());
        let output = result.unwrap();

        // 单点应该生成一个 Circle
        let data_layer = output.layers.get_layer(LayerKind::Data).unwrap();
        assert_eq!(data_layer.commands.len(), 1);
        if let DrawCmd::Circle { .. } = &data_layer.commands.semantic[0] {
            // 正确
        } else {
            panic!("Expected Circle for single point");
        }
    }

    #[test]
    fn test_scatter_chart_render_multi_series() {
        let spec = ChartSpec::builder()
            .mark(Mark::Scatter)
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
        let result = ScatterChart::render(&spec, &theme, &data);

        assert!(result.is_ok());
        let output = result.unwrap();

        // 应该有两个系列的点
        let data_layer = output.layers.get_layer(LayerKind::Data).unwrap();
        assert_eq!(data_layer.commands.len(), 4); // 2 个系列 × 2 个点
    }

    #[test]
    fn test_scatter_chart_with_size_encoding() {
        let spec = ChartSpec::builder()
            .mark(Mark::Scatter)
            .encoding(
                Encoding::new()
                    .x(crate::spec::Field::quantitative("x"))
                    .y(crate::spec::Field::quantitative("y"))
                    .size(crate::spec::Field::quantitative("size")),
            )
            .width(800.0)
            .height(400.0)
            .build()
            .unwrap();

        let data = DataTable::with_columns(vec![
            Column::new("x", DataType::Quantitative, vec![
                FieldValue::Numeric(0.0),
                FieldValue::Numeric(1.0),
            ]),
            Column::new("y", DataType::Quantitative, vec![
                FieldValue::Numeric(10.0),
                FieldValue::Numeric(20.0),
            ]),
            Column::new("size", DataType::Quantitative, vec![
                FieldValue::Numeric(1.0),
                FieldValue::Numeric(10.0),
            ]),
        ]);

        let theme = crate::theme::DefaultTheme;
        let result = ScatterChart::render(&spec, &theme, &data);

        assert!(result.is_ok());
        let output = result.unwrap();

        // 验证两个点的半径不同
        let data_layer = output.layers.get_layer(LayerKind::Data).unwrap();
        assert_eq!(data_layer.commands.len(), 2);

        if let (DrawCmd::Circle { r: r1, .. }, DrawCmd::Circle { r: r2, .. }) = (
            &data_layer.commands.semantic[0],
            &data_layer.commands.semantic[1],
        ) {
            assert!(r1 < r2); // 第一个点的半径应该更小
        } else {
            panic!("Expected Circle commands");
        }
    }

    #[test]
    fn test_scatter_chart_downsample() {
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

        let result = ScatterChart::render(&spec, &theme, &data);

        assert!(result.is_ok());
        let output = result.unwrap();

        // 应该成功渲染并降采样
        assert!(output.layers.get_layer(LayerKind::Data).unwrap().commands.len() > 0);
        // 降采样后的点数应该少于原始数据
        assert!(output.layers.get_layer(LayerKind::Data).unwrap().commands.len() < 15000);
    }

    #[test]
    fn test_scatter_chart_hit_regions() {
        let spec = create_test_spec();
        let theme = crate::theme::DefaultTheme;
        let data = create_test_data();

        let result = ScatterChart::render(&spec, &theme, &data).unwrap();

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
