//! 直方图渲染器
//!
//! 将 ChartSpec 和 DataTable 渲染为直方图的 Canvas 指令。
//! 使用 Sturges' rule 计算分箱数量。

use crate::layout::{compute_layout, PlotArea};
use crate::spec::ChartSpec;
use crate::theme::Theme;
use crate::chart::ChartOutput;
use crate::error::ComponentError;
use deneb_core::*;

/// HistogramChart 渲染器
pub struct HistogramChart;

/// 分箱结果
struct Bin {
    /// 分箱左边界
    left: f64,
    /// 分箱右边界
    right: f64,
    /// 计数
    count: usize,
}

impl HistogramChart {
    /// 渲染直方图
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

        // 2. 提取数值数据
        let x_field = spec.encoding.x.as_ref().ok_or_else(|| {
            ComponentError::InvalidConfig {
                reason: "x encoding is required".to_string(),
            }
        })?;

        let x_column = data.get_column(&x_field.name).ok_or_else(|| {
            ComponentError::InvalidConfig {
                reason: format!("x field '{}' not found in data", x_field.name),
            }
        })?;

        let values: Vec<f64> = x_column
            .values
            .iter()
            .filter_map(|v| v.as_numeric())
            .collect();

        if values.is_empty() {
            return Ok(Self::render_empty(spec, theme));
        }

        // 3. 计算分箱
        let bins = Self::compute_bins(&values);

        // 4. 计算布局
        let layout = compute_layout(spec, theme, data);
        let plot_area = &layout.plot_area;

        // 5. 构建 Scale
        let x_scale = Self::build_x_scale(&bins, plot_area);
        let y_scale = Self::build_y_scale(&bins, plot_area);

        // 6. 生成渲染指令
        let mut layers = RenderLayers::new();
        let mut hit_regions = Vec::new();

        // 背景层
        layers.update_layer(LayerKind::Background, super::shared::render_background_with_border(spec, theme, plot_area));

        // 网格层
        if let Some(y_axis) = &layout.y_axis {
            layers.update_layer(LayerKind::Grid, super::shared::render_grid_horizontal(theme, &y_axis.tick_positions, plot_area));
        }

        // 数据层（柱子）
        let (data_commands, bin_regions) = Self::render_bins(
            theme, &x_scale, &y_scale, &bins, plot_area,
        );
        layers.update_layer(LayerKind::Data, data_commands);
        hit_regions.extend(bin_regions);

        // 轴层
        layers.update_layer(LayerKind::Axis, super::shared::render_axes(spec, theme, &layout, plot_area, false));

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

    /// Sturges' rule: bin count = ceil(1 + log2(n))
    fn compute_bins(values: &[f64]) -> Vec<Bin> {
        let n = values.len();
        if n == 0 {
            return Vec::new();
        }

        let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        // 所有值相同 → 单个分箱
        if (max - min).abs() < f64::EPSILON {
            return vec![Bin {
                left: min - 0.5,
                right: max + 0.5,
                count: n,
            }];
        }

        // Sturges' rule
        let bin_count = (1.0 + (n as f64).log2()).ceil() as usize;
        let bin_count = bin_count.max(1);
        let bin_width = (max - min) / bin_count as f64;

        let mut bins: Vec<Bin> = (0..bin_count)
            .map(|i| Bin {
                left: min + i as f64 * bin_width,
                right: min + (i + 1) as f64 * bin_width,
                count: 0,
            })
            .collect();

        for &v in values {
            // 最后一个 bin 包含右端点
            let idx = if v >= max {
                bin_count - 1
            } else {
                ((v - min) / bin_width).floor() as usize
            };
            let idx = idx.min(bin_count - 1);
            bins[idx].count += 1;
        }

        bins
    }

    /// X 轴 LinearScale（基于分箱边界）
    fn build_x_scale(bins: &[Bin], plot_area: &PlotArea) -> LinearScale {
        if bins.is_empty() {
            return LinearScale::new(0.0, 1.0, plot_area.x, plot_area.x + plot_area.width);
        }

        let min = bins.first().unwrap().left;
        let max = bins.last().unwrap().right;

        LinearScale::new(min, max, plot_area.x, plot_area.x + plot_area.width)
    }

    /// Y 轴 LinearScale，从 0 开始（直方图必须）
    fn build_y_scale(bins: &[Bin], plot_area: &PlotArea) -> LinearScale {
        let max_count = bins.iter().map(|b| b.count).max().unwrap_or(0) as f64;
        let max_count = max_count.max(1.0);

        LinearScale::new(
            0.0,
            max_count,
            plot_area.y + plot_area.height,
            plot_area.y,
        )
    }

    /// 渲染分箱柱子
    fn render_bins<T: Theme>(
        theme: &T,
        x_scale: &LinearScale,
        y_scale: &LinearScale,
        bins: &[Bin],
        _plot_area: &PlotArea,
    ) -> (RenderOutput, Vec<HitRegion>) {
        let mut output = RenderOutput::new();
        let mut hit_regions = Vec::new();

        let baseline = y_scale.map(0.0);

        for (i, bin) in bins.iter().enumerate() {
            let x_left = x_scale.map(bin.left);
            let x_right = x_scale.map(bin.right);
            let bin_width = x_right - x_left;

            let count = bin.count as f64;
            let y_top = y_scale.map(count);

            let bar_y = y_top;
            let bar_height = (baseline - y_top).max(1.0);

            let color = theme.series_color(i);

            output.add_command(DrawCmd::Rect {
                x: x_left,
                y: bar_y,
                width: bin_width,
                height: bar_height,
                fill: Some(FillStyle::Color(color.to_string())),
                stroke: None,
                corner_radius: None,
            });

            let region = HitRegion::from_rect(
                x_left,
                bar_y,
                bin_width,
                bar_height,
                i,
                None,
                vec![
                    FieldValue::Numeric(bin.left),
                    FieldValue::Numeric(bin.right),
                    FieldValue::Numeric(count),
                ],
            );
            hit_regions.push(region);
        }

        (output, hit_regions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{Encoding, Mark, Field};
    use crate::theme::DefaultTheme;
    use deneb_core::{Column, DataType, FieldValue};

    fn create_test_data() -> DataTable {
        DataTable::with_columns(vec![
            Column::new(
                "value",
                DataType::Quantitative,
                (0..50).map(|i| FieldValue::Numeric(i as f64)).collect(),
            ),
            Column::new(
                "dummy",
                DataType::Nominal,
                (0..50).map(|_| FieldValue::Text("x".to_string())).collect(),
            ),
        ])
    }

    fn create_test_spec() -> ChartSpec {
        ChartSpec::builder()
            .mark(Mark::Histogram)
            .encoding(
                Encoding::new()
                    .x(Field::quantitative("value"))
                    .y(Field::quantitative("dummy")),
            )
            .width(400.0)
            .height(300.0)
            .build()
            .unwrap()
    }

    #[test]
    fn test_histogram_render_basic() {
        let spec = create_test_spec();
        let theme = DefaultTheme;
        let data = create_test_data();

        let result = HistogramChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(!output.hit_regions.is_empty());
        assert!(output.layers.get_layer(LayerKind::Background).is_some());
        assert!(output.layers.get_layer(LayerKind::Data).is_some());
    }

    #[test]
    fn test_histogram_render_empty_data() {
        let spec = create_test_spec();
        let theme = DefaultTheme;
        let data = DataTable::with_columns(vec![
            Column::new("value", DataType::Quantitative, vec![]),
            Column::new("dummy", DataType::Nominal, vec![]),
        ]);

        let result = HistogramChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.hit_regions.is_empty());
        assert!(output.layers.get_layer(LayerKind::Background).is_some());
    }

    #[test]
    fn test_histogram_all_same_values() {
        let spec = create_test_spec();
        let theme = DefaultTheme;
        let data = DataTable::with_columns(vec![
            Column::new(
                "value",
                DataType::Quantitative,
                vec![
                    FieldValue::Numeric(5.0),
                    FieldValue::Numeric(5.0),
                    FieldValue::Numeric(5.0),
                ],
            ),
            Column::new(
                "dummy",
                DataType::Nominal,
                vec![
                    FieldValue::Text("x".to_string()),
                    FieldValue::Text("x".to_string()),
                    FieldValue::Text("x".to_string()),
                ],
            ),
        ]);

        let result = HistogramChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        // 单一分箱
        assert_eq!(output.hit_regions.len(), 1);
    }

    #[test]
    fn test_histogram_with_title() {
        let spec = ChartSpec::builder()
            .mark(Mark::Histogram)
            .encoding(
                Encoding::new()
                    .x(Field::quantitative("value"))
                    .y(Field::quantitative("dummy")),
            )
            .title("Histogram Test")
            .width(400.0)
            .height(300.0)
            .build()
            .unwrap();

        let theme = DefaultTheme;
        let data = create_test_data();

        let result = HistogramChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        let title_layer = output.layers.get_layer(LayerKind::Title);
        assert!(title_layer.is_some());
        assert!(!title_layer.unwrap().commands.is_empty());
    }

    #[test]
    fn test_histogram_validate_data_missing_field() {
        let spec = create_test_spec();
        let theme = DefaultTheme;
        let data = DataTable::with_columns(vec![
            Column::new("wrong_field", DataType::Quantitative, vec![]),
            Column::new("dummy", DataType::Nominal, vec![]),
        ]);

        let result = HistogramChart::render(&spec, &theme, &data);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_histogram_layers() {
        let spec = create_test_spec();
        let theme = DefaultTheme;
        let data = create_test_data();

        let result = HistogramChart::render(&spec, &theme, &data).unwrap();

        assert!(result.layers.get_layer(LayerKind::Background).is_some());
        assert!(result.layers.get_layer(LayerKind::Grid).is_some());
        assert!(result.layers.get_layer(LayerKind::Axis).is_some());
        assert!(result.layers.get_layer(LayerKind::Data).is_some());
    }
}
