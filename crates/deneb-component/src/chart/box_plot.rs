//! 箱线图渲染器
//!
//! 将 ChartSpec 和 DataTable 渲染为箱线图的 Canvas 指令。
//! 支持五数概括（min, Q1, median, Q3, max）和 IQR 异常值检测。

use crate::layout::{compute_layout, PlotArea};
use crate::spec::ChartSpec;
use crate::theme::Theme;
use crate::chart::ChartOutput;
use crate::error::ComponentError;
use deneb_core::*;
use std::collections::HashMap;

/// BoxPlotChart 渲染器
pub struct BoxPlotChart;

/// 五数概括统计量
struct BoxStats {
    /// 最小值（whisker 下界）
    min: f64,
    /// Q1（25th percentile）
    q1: f64,
    /// 中位数（50th percentile）
    median: f64,
    /// Q3（75th percentile）
    q3: f64,
    /// 最大值（whisker 上界）
    max: f64,
    /// 异常值
    outliers: Vec<f64>,
}

impl BoxPlotChart {
    /// 渲染箱线图
    pub fn render<T: Theme>(
        spec: &ChartSpec,
        theme: &T,
        data: &DataTable,
    ) -> Result<ChartOutput, ComponentError> {
        // 1. 验证数据
        Self::validate_data(spec, data)?;

        // 空数据检查
        if data.is_empty() || data.row_count() == 0 {
            return Ok(Self::render_empty(spec, theme));
        }

        // 2. 计算布局
        let layout = compute_layout(spec, theme, data);
        let plot_area = &layout.plot_area;

        // 3. 构建 Scale
        let (x_scale, y_scale) = Self::build_scales(spec, data, plot_area)?;

        // 4. 按类别分组并计算统计量
        let groups = Self::group_and_compute_stats(spec, data)?;

        // 5. 生成渲染指令
        let mut layers = RenderLayers::new();
        let mut hit_regions = Vec::new();

        // 背景层
        layers.update_layer(LayerKind::Background, super::shared::render_background_with_border(spec, theme, plot_area));

        // 网格层
        if let Some(y_axis) = &layout.y_axis {
            layers.update_layer(LayerKind::Grid, super::shared::render_grid_horizontal(theme, &y_axis.tick_positions, plot_area));
        }

        // 数据层（箱体、须、异常值）
        let (data_commands, box_regions) = Self::render_boxes(
            theme,
            &x_scale,
            &y_scale,
            &groups,
            plot_area,
        )?;
        layers.update_layer(LayerKind::Data, data_commands);
        hit_regions.extend(box_regions);

        // 轴层
        layers.update_layer(LayerKind::Axis, super::shared::render_axes(spec, theme, &layout, plot_area, true));

        // 标题层
        if let Some(title) = &spec.title {
            layers.update_layer(LayerKind::Title, super::shared::render_title(theme, title, plot_area));
        }

        Ok(ChartOutput {
            layers,
            hit_regions,
        })
    }

    /// 验证数据
    fn validate_data(spec: &ChartSpec, data: &DataTable) -> Result<(), ComponentError> {
        if let Some(x_field) = &spec.encoding.x {
            if data.get_column(&x_field.name).is_none() {
                return Err(ComponentError::InvalidConfig {
                    reason: format!("x field '{}' not found in data", x_field.name),
                });
            }
        }

        if let Some(y_field) = &spec.encoding.y {
            if data.get_column(&y_field.name).is_none() {
                return Err(ComponentError::InvalidConfig {
                    reason: format!("y field '{}' not found in data", y_field.name),
                });
            }
        }

        Ok(())
    }

    /// 渲染空数据
    fn render_empty<T: Theme>(spec: &ChartSpec, theme: &T) -> ChartOutput {
        let mut layers = RenderLayers::new();
        let plot_area = PlotArea {
            x: theme.margin().left,
            y: theme.margin().top,
            width: spec.width - theme.margin().horizontal(),
            height: spec.height - theme.margin().vertical(),
        };

        layers.update_layer(LayerKind::Background, super::shared::render_background_with_border(spec, theme, &plot_area));

        if let Some(title) = &spec.title {
            layers.update_layer(LayerKind::Title, super::shared::render_title(theme, title, &plot_area));
        }

        ChartOutput {
            layers,
            hit_regions: Vec::new(),
        }
    }

    /// 构建比例尺
    fn build_scales(
        spec: &ChartSpec,
        data: &DataTable,
        plot_area: &PlotArea,
    ) -> Result<(BandScale, LinearScale), ComponentError> {
        let x_field = spec.encoding.x.as_ref().ok_or_else(|| {
            ComponentError::InvalidConfig {
                reason: "x encoding is required".to_string(),
            }
        })?;

        let x_column = data.get_column(&x_field.name).ok_or_else(|| {
            ComponentError::InvalidConfig {
                reason: format!("x field '{}' not found", x_field.name),
            }
        })?;

        let categories: Vec<String> = x_column
            .values
            .iter()
            .filter_map(|v| v.as_text().map(|s| s.to_string()))
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        let x_scale = BandScale::new(
            categories,
            plot_area.x,
            plot_area.x + plot_area.width,
            0.1,
        );

        let y_field = spec.encoding.y.as_ref().ok_or_else(|| {
            ComponentError::InvalidConfig {
                reason: "y encoding is required".to_string(),
            }
        })?;

        let y_column = data.get_column(&y_field.name).ok_or_else(|| {
            ComponentError::InvalidConfig {
                reason: format!("y field '{}' not found", y_field.name),
            }
        })?;

        let mut min: Option<f64> = None;
        let mut max: Option<f64> = None;

        for value in &y_column.values {
            if let Some(num) = value.as_numeric() {
                min = Some(min.map_or(num, |m| m.min(num)));
                max = Some(max.map_or(num, |m| m.max(num)));
            }
        }

        let (min, max) = match (min, max) {
            (Some(min), Some(max)) => (min, max),
            _ => (0.0, 100.0),
        };

        // BoxPlot Y 轴不从 0 开始（位置编码，非长度编码）
        let y_scale = LinearScale::new(
            min,
            max,
            plot_area.y + plot_area.height,
            plot_area.y,
        );

        Ok((x_scale, y_scale))
    }

    /// 按类别分组并计算五数概括
    fn group_and_compute_stats(
        spec: &ChartSpec,
        data: &DataTable,
    ) -> Result<HashMap<String, BoxStats>, ComponentError> {
        let x_field = spec.encoding.x.as_ref().ok_or_else(|| {
            ComponentError::InvalidConfig {
                reason: "x encoding is required".to_string(),
            }
        })?;

        let y_field = spec.encoding.y.as_ref().ok_or_else(|| {
            ComponentError::InvalidConfig {
                reason: "y encoding is required".to_string(),
            }
        })?;

        let x_column = data.get_column(&x_field.name).ok_or_else(|| {
            ComponentError::InvalidConfig {
                reason: format!("x field '{}' not found", x_field.name),
            }
        })?;

        let y_column = data.get_column(&y_field.name).ok_or_else(|| {
            ComponentError::InvalidConfig {
                reason: format!("y field '{}' not found", y_field.name),
            }
        })?;

        // 按类别分组收集数值
        let mut groups: HashMap<String, Vec<f64>> = HashMap::new();
        let row_count = data.row_count();

        for row_idx in 0..row_count {
            let x_value = x_column
                .get(row_idx)
                .and_then(|v| v.as_text())
                .unwrap_or("")
                .to_string();

            let y_value = y_column
                .get(row_idx)
                .and_then(|v| v.as_numeric())
                .unwrap_or(0.0);

            groups
                .entry(x_value)
                .or_insert_with(Vec::new)
                .push(y_value);
        }

        // 为每组计算统计量
        let mut result = HashMap::new();
        for (category, mut values) in groups {
            values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let stats = Self::compute_box_stats(&values);
            result.insert(category, stats);
        }

        Ok(result)
    }

    /// 计算五数概括和异常值
    fn compute_box_stats(sorted_values: &[f64]) -> BoxStats {
        let n = sorted_values.len();
        if n == 0 {
            return BoxStats {
                min: 0.0, q1: 0.0, median: 0.0, q3: 0.0, max: 0.0,
                outliers: Vec::new(),
            };
        }

        let min = sorted_values[0];
        let max = sorted_values[n - 1];
        let median = Self::percentile(sorted_values, 50.0);
        let q1 = Self::percentile(sorted_values, 25.0);
        let q3 = Self::percentile(sorted_values, 75.0);

        let iqr = q3 - q1;
        let lower_fence = q1 - 1.5 * iqr;
        let upper_fence = q3 + 1.5 * iqr;

        // Whiskers: 最后一个在 fence 内的数据点
        let whisker_min = sorted_values
            .iter()
            .find(|&&v| v >= lower_fence)
            .copied()
            .unwrap_or(min);
        let whisker_max = sorted_values
            .iter()
            .rev()
            .find(|&&v| v <= upper_fence)
            .copied()
            .unwrap_or(max);

        // 异常值
        let outliers: Vec<f64> = sorted_values
            .iter()
            .filter(|&&v| v < lower_fence || v > upper_fence)
            .copied()
            .collect();

        BoxStats {
            min: whisker_min,
            q1,
            median,
            q3,
            max: whisker_max,
            outliers,
        }
    }

    /// 线性插值百分位数
    fn percentile(sorted: &[f64], p: f64) -> f64 {
        let n = sorted.len();
        if n == 0 {
            return 0.0;
        }
        if n == 1 {
            return sorted[0];
        }

        let rank = (p / 100.0) * (n - 1) as f64;
        let lower = rank.floor() as usize;
        let upper = (lower + 1).min(n - 1);
        let frac = rank - lower as f64;

        sorted[lower] + frac * (sorted[upper] - sorted[lower])
    }

    /// 渲染箱体、须和异常值
    fn render_boxes<T: Theme>(
        theme: &T,
        x_scale: &BandScale,
        y_scale: &LinearScale,
        groups: &HashMap<String, BoxStats>,
        _plot_area: &PlotArea,
    ) -> Result<(RenderOutput, Vec<HitRegion>), ComponentError> {
        let mut output = RenderOutput::new();
        let mut hit_regions = Vec::new();

        let box_color = theme.series_color(0).to_string();
        let median_color = theme.foreground_color().to_string();
        let outlier_color = theme.series_color(1).to_string();
        let whisker_stroke = StrokeStyle::Color(theme.foreground_color().to_string());

        let mut group_idx = 0;
        for (category, stats) in groups {
            let band_center = x_scale.band_center(category).ok_or_else(|| {
                ComponentError::InvalidConfig {
                    reason: format!("category not found in x scale: {}", category),
                }
            })?;

            let band_width = x_scale.band_width();
            let box_half_width = band_width * 0.4; // 箱体占 band 的 80%

            let color = if groups.len() > 1 {
                theme.series_color(group_idx).to_string()
            } else {
                box_color.clone()
            };

            // 箱体 (Q1 to Q3)
            let q1_y = y_scale.map(stats.q1);
            let q3_y = y_scale.map(stats.q3);
            let box_x = band_center - box_half_width;
            let box_width = box_half_width * 2.0;
            let box_y = q3_y; // Q3 在上方（像素值更小）
            let box_height = (q1_y - q3_y).max(1.0); // 至少 1px

            // 下须：从 Q1 到 min
            let whisker_min_y = y_scale.map(stats.min);
            let whisker_max_y = y_scale.map(stats.max);

            // 须（竖线）
            output.add_command(DrawCmd::Path {
                segments: vec![
                    PathSegment::MoveTo(band_center, q1_y),
                    PathSegment::LineTo(band_center, whisker_min_y),
                ],
                fill: None,
                stroke: Some(whisker_stroke.clone()),
            });

            // 上须：从 Q3 到 max
            output.add_command(DrawCmd::Path {
                segments: vec![
                    PathSegment::MoveTo(band_center, q3_y),
                    PathSegment::LineTo(band_center, whisker_max_y),
                ],
                fill: None,
                stroke: Some(whisker_stroke.clone()),
            });

            // 须端（水平线）
            let cap_half = box_half_width * 0.5;
            output.add_command(DrawCmd::Path {
                segments: vec![
                    PathSegment::MoveTo(band_center - cap_half, whisker_min_y),
                    PathSegment::LineTo(band_center + cap_half, whisker_min_y),
                ],
                fill: None,
                stroke: Some(whisker_stroke.clone()),
            });
            output.add_command(DrawCmd::Path {
                segments: vec![
                    PathSegment::MoveTo(band_center - cap_half, whisker_max_y),
                    PathSegment::LineTo(band_center + cap_half, whisker_max_y),
                ],
                fill: None,
                stroke: Some(whisker_stroke.clone()),
            });

            // 箱体矩形
            output.add_command(DrawCmd::Rect {
                x: box_x,
                y: box_y,
                width: box_width,
                height: box_height,
                fill: Some(FillStyle::Color(color)),
                stroke: Some(StrokeStyle::Color(median_color.clone())),
                corner_radius: None,
            });

            // 中位数线
            let median_y = y_scale.map(stats.median);
            output.add_command(DrawCmd::Path {
                segments: vec![
                    PathSegment::MoveTo(box_x, median_y),
                    PathSegment::LineTo(box_x + box_width, median_y),
                ],
                fill: None,
                stroke: Some(StrokeStyle::Color(median_color.clone())),
            });

            // 命中区域（整个箱体+须的范围）
            let total_min_y = whisker_min_y.min(whisker_max_y);
            let total_max_y = whisker_min_y.max(whisker_max_y);
            let region = HitRegion::from_rect(
                box_x,
                total_min_y,
                box_width,
                total_max_y - total_min_y,
                group_idx,
                None,
                vec![
                    FieldValue::Text(category.clone()),
                    FieldValue::Numeric(stats.min),
                    FieldValue::Numeric(stats.q1),
                    FieldValue::Numeric(stats.median),
                    FieldValue::Numeric(stats.q3),
                    FieldValue::Numeric(stats.max),
                ],
            );
            hit_regions.push(region);

            // 异常值（圆点）
            let point_radius = theme.layout_config().point_radius;
            for &outlier_val in &stats.outliers {
                let outlier_y = y_scale.map(outlier_val);
                output.add_command(DrawCmd::Circle {
                    cx: band_center,
                    cy: outlier_y,
                    r: point_radius,
                    fill: Some(FillStyle::Color(outlier_color.clone())),
                    stroke: None,
                });

                let outlier_region = HitRegion::from_point(
                    band_center,
                    outlier_y,
                    point_radius,
                    group_idx,
                    None,
                    vec![
                        FieldValue::Text(category.clone()),
                        FieldValue::Numeric(outlier_val),
                    ],
                );
                hit_regions.push(outlier_region);
            }

            group_idx += 1;
        }

        Ok((output, hit_regions))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{Encoding, Mark, Field};
    use crate::theme::DefaultTheme;

    fn create_test_data() -> DataTable {
        // 三组数据：A 有异常值，B 正常，C 单值
        DataTable::with_columns(vec![
            Column::new(
                "category",
                DataType::Nominal,
                vec![
                    // A: 8 个值，含异常值
                    FieldValue::Text("A".to_string()),
                    FieldValue::Text("A".to_string()),
                    FieldValue::Text("A".to_string()),
                    FieldValue::Text("A".to_string()),
                    FieldValue::Text("A".to_string()),
                    FieldValue::Text("A".to_string()),
                    FieldValue::Text("A".to_string()),
                    FieldValue::Text("A".to_string()),
                    // B: 5 个值
                    FieldValue::Text("B".to_string()),
                    FieldValue::Text("B".to_string()),
                    FieldValue::Text("B".to_string()),
                    FieldValue::Text("B".to_string()),
                    FieldValue::Text("B".to_string()),
                ],
            ),
            Column::new(
                "value",
                DataType::Quantitative,
                vec![
                    // A: [1, 2, 3, 4, 5, 6, 7, 100] — 100 是异常值
                    FieldValue::Numeric(1.0),
                    FieldValue::Numeric(2.0),
                    FieldValue::Numeric(3.0),
                    FieldValue::Numeric(4.0),
                    FieldValue::Numeric(5.0),
                    FieldValue::Numeric(6.0),
                    FieldValue::Numeric(7.0),
                    FieldValue::Numeric(100.0),
                    // B: [10, 20, 30, 40, 50]
                    FieldValue::Numeric(10.0),
                    FieldValue::Numeric(20.0),
                    FieldValue::Numeric(30.0),
                    FieldValue::Numeric(40.0),
                    FieldValue::Numeric(50.0),
                ],
            ),
        ])
    }

    fn create_test_spec() -> ChartSpec {
        ChartSpec::builder()
            .mark(Mark::BoxPlot)
            .encoding(
                Encoding::new()
                    .x(Field::nominal("category"))
                    .y(Field::quantitative("value")),
            )
            .width(400.0)
            .height(300.0)
            .build()
            .unwrap()
    }

    #[test]
    fn test_box_plot_render_basic() {
        let spec = create_test_spec();
        let theme = DefaultTheme;
        let data = create_test_data();

        let result = BoxPlotChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        // 2 个箱体 + 1 个异常值(A 的 100)
        assert!(!output.hit_regions.is_empty());
        assert!(output.layers.get_layer(LayerKind::Background).is_some());
        assert!(output.layers.get_layer(LayerKind::Data).is_some());
        assert!(output.layers.get_layer(LayerKind::Axis).is_some());
    }

    #[test]
    fn test_box_plot_render_empty_data() {
        let spec = create_test_spec();
        let theme = DefaultTheme;
        let data = DataTable::with_columns(vec![
            Column::new("category", DataType::Nominal, vec![]),
            Column::new("value", DataType::Quantitative, vec![]),
        ]);

        let result = BoxPlotChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.hit_regions.is_empty());
        assert!(output.layers.get_layer(LayerKind::Background).is_some());
    }

    #[test]
    fn test_box_plot_validate_missing_field() {
        let spec = create_test_spec();
        let theme = DefaultTheme;
        let data = DataTable::with_columns(vec![
            Column::new("wrong", DataType::Nominal, vec![]),
        ]);

        let result = BoxPlotChart::render(&spec, &theme, &data);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_box_plot_compute_stats_basic() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 100.0];
        let stats = BoxPlotChart::compute_box_stats(&values);

        // Q1 ≈ 2.5, Q3 ≈ 6.5, median = 4.5
        // IQR = 4.0, lower fence = -3.5, upper fence = 12.5
        // Whisker min = 1, whisker max = 7
        // Outlier = 100
        assert_eq!(stats.outliers, vec![100.0]);
        assert_eq!(stats.min, 1.0);
        assert_eq!(stats.max, 7.0);
        assert!((stats.median - 4.5).abs() < 0.01);
    }

    #[test]
    fn test_box_plot_compute_stats_all_same() {
        let values = vec![5.0, 5.0, 5.0, 5.0, 5.0];
        let stats = BoxPlotChart::compute_box_stats(&values);

        assert!(stats.outliers.is_empty());
        assert_eq!(stats.min, 5.0);
        assert_eq!(stats.max, 5.0);
        assert!((stats.median - 5.0).abs() < f64::EPSILON);
        assert!((stats.q1 - 5.0).abs() < f64::EPSILON);
        assert!((stats.q3 - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_box_plot_with_title() {
        let spec = ChartSpec::builder()
            .mark(Mark::BoxPlot)
            .encoding(
                Encoding::new()
                    .x(Field::nominal("category"))
                    .y(Field::quantitative("value")),
            )
            .title("Box Plot Test")
            .width(400.0)
            .height(300.0)
            .build()
            .unwrap();

        let theme = DefaultTheme;
        let data = create_test_data();

        let result = BoxPlotChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        let title_layer = output.layers.get_layer(LayerKind::Title);
        assert!(title_layer.is_some());
        assert!(!title_layer.unwrap().commands.is_empty());
    }

    #[test]
    fn test_box_plot_single_category() {
        let data = DataTable::with_columns(vec![
            Column::new(
                "category",
                DataType::Nominal,
                vec![
                    FieldValue::Text("X".to_string()),
                    FieldValue::Text("X".to_string()),
                    FieldValue::Text("X".to_string()),
                ],
            ),
            Column::new(
                "value",
                DataType::Quantitative,
                vec![
                    FieldValue::Numeric(10.0),
                    FieldValue::Numeric(20.0),
                    FieldValue::Numeric(30.0),
                ],
            ),
        ]);

        let spec = create_test_spec();
        let theme = DefaultTheme;

        let result = BoxPlotChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        // 1 个箱体，无异常值
        assert_eq!(output.hit_regions.len(), 1);
    }

    #[test]
    fn test_box_plot_layers() {
        let spec = create_test_spec();
        let theme = DefaultTheme;
        let data = create_test_data();

        let result = BoxPlotChart::render(&spec, &theme, &data).unwrap();

        assert!(result.layers.get_layer(LayerKind::Background).is_some());
        assert!(result.layers.get_layer(LayerKind::Grid).is_some());
        assert!(result.layers.get_layer(LayerKind::Axis).is_some());
        assert!(result.layers.get_layer(LayerKind::Data).is_some());
    }
}
