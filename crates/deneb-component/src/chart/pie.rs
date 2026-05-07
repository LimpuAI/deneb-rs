//! 饼图渲染器
//!
//! 将 ChartSpec 和 DataTable 渲染为饼图的 Canvas 指令。
//! 使用极坐标系，通过 DrawCmd::Arc 绘制扇形。

use crate::layout::PlotArea;
use crate::spec::ChartSpec;
use crate::theme::Theme;
use crate::chart::ChartOutput;
use crate::error::ComponentError;
use deneb_core::*;

/// PieChart 渲染器
pub struct PieChart;

impl PieChart {
    /// 渲染饼图
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

        // 2. 计算布局（饼图使用自己的布局逻辑，不用 compute_layout）
        let plot_area = PlotArea {
            x: theme.margin().left,
            y: theme.margin().top,
            width: spec.width - theme.margin().horizontal(),
            height: spec.height - theme.margin().vertical(),
        };

        // 3. 提取数据和角度
        let slices = Self::build_slices(spec, data)?;

        // 4. 生成渲染指令
        let mut layers = RenderLayers::new();
        let mut hit_regions = Vec::new();

        // 背景层
        layers.update_layer(LayerKind::Background, super::shared::render_background(spec, theme));

        // 数据层（扇形 + 标签）
        let (data_commands, slice_regions) = Self::render_slices(
            theme,
            &slices,
            &plot_area,
            data,
        );
        layers.update_layer(LayerKind::Data, data_commands);
        hit_regions.extend(slice_regions);

        // 标题层
        if let Some(title) = &spec.title {
            layers.update_layer(LayerKind::Title, Self::render_title(spec, theme, title, &plot_area));
        }

        Ok(ChartOutput {
            layers,
            hit_regions,
        })
    }

    /// 验证数据
    fn validate_data(spec: &ChartSpec, data: &DataTable) -> Result<(), ComponentError> {
        if let Some(color_field) = &spec.encoding.color {
            if data.get_column(&color_field.name).is_none() {
                return Err(ComponentError::InvalidConfig {
                    reason: format!("color field '{}' not found in data", color_field.name),
                });
            }
        }

        if let Some(theta_field) = &spec.encoding.theta {
            if data.get_column(&theta_field.name).is_none() {
                return Err(ComponentError::InvalidConfig {
                    reason: format!("theta field '{}' not found in data", theta_field.name),
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

        layers.update_layer(LayerKind::Background, super::shared::render_background(spec, theme));

        if let Some(title) = &spec.title {
            layers.update_layer(LayerKind::Title, Self::render_title(spec, theme, title, &plot_area));
        }

        ChartOutput {
            layers,
            hit_regions: Vec::new(),
        }
    }

    /// 构建切片数据
    fn build_slices(
        spec: &ChartSpec,
        data: &DataTable,
    ) -> Result<Vec<SliceData>, ComponentError> {
        let row_count = data.row_count();

        // 获取标签列（使用 color 编码或 x 编码）
        let label_column = spec.encoding.color.as_ref()
            .or(spec.encoding.x.as_ref())
            .and_then(|field| data.get_column(&field.name));

        // 获取数值列（theta 或 y）
        let value_column = spec.encoding.theta.as_ref()
            .or(spec.encoding.y.as_ref())
            .and_then(|field| data.get_column(&field.name));

        let mut slices = Vec::new();

        if let Some(values) = value_column {
            let total: f64 = (0..row_count)
                .filter_map(|i| values.get(i).and_then(|v| v.as_numeric()))
                .sum();

            for row_idx in 0..row_count {
                let value = values
                    .get(row_idx)
                    .and_then(|v| v.as_numeric())
                    .unwrap_or(0.0);

                let label = label_column
                    .and_then(|col| col.get(row_idx))
                    .and_then(|v| v.as_text())
                    .unwrap_or("")
                    .to_string();

                // 全零时等分
                let fraction = if total == 0.0 {
                    1.0 / row_count as f64
                } else {
                    (value / total).abs()
                };

                slices.push(SliceData {
                    label,
                    value,
                    fraction,
                    row_idx,
                });
            }
        } else {
            // 无 theta 字段：等分
            for row_idx in 0..row_count {
                let label = label_column
                    .and_then(|col| col.get(row_idx))
                    .and_then(|v| v.as_text())
                    .unwrap_or("")
                    .to_string();

                slices.push(SliceData {
                    label,
                    value: 1.0,
                    fraction: 1.0 / row_count as f64,
                    row_idx,
                });
            }
        }

        Ok(slices)
    }

    /// 渲染扇形
    fn render_slices<T: Theme>(
        theme: &T,
        slices: &[SliceData],
        plot_area: &PlotArea,
        data: &DataTable,
    ) -> (RenderOutput, Vec<HitRegion>) {
        let mut output = RenderOutput::new();
        let mut hit_regions = Vec::new();

        let cx = plot_area.x + plot_area.width / 2.0;
        let cy = plot_area.y + plot_area.height / 2.0;
        let radius = (plot_area.width.min(plot_area.height) / 2.0) - 10.0;
        let radius = radius.max(1.0);

        let start_offset = -std::f64::consts::FRAC_PI_2; // 从 12 点方向开始
        let mut current_angle = start_offset;

        for (idx, slice) in slices.iter().enumerate() {
            let slice_angle = slice.fraction * 2.0 * std::f64::consts::PI;
            let end_angle = current_angle + slice_angle;

            let color = theme.series_color(idx).to_string();

            // 绘制扇形
            output.add_command(DrawCmd::Arc {
                cx,
                cy,
                r: radius,
                start_angle: current_angle,
                end_angle,
                fill: Some(FillStyle::Color(color)),
                stroke: Some(StrokeStyle::Color("#ffffff".to_string())),
            });

            // 计算标签位置（扇形中点角度方向）
            let mid_angle = current_angle + slice_angle / 2.0;
            let label_r = radius + 20.0;
            let label_x = cx + label_r * mid_angle.cos();
            let label_y = cy + label_r * mid_angle.sin();

            // 标签文本
            let percentage = (slice.fraction * 100.0).round() as i32;
            let label_text = if slice.label.is_empty() {
                format!("{}%", percentage)
            } else {
                format!("{} ({}%)", slice.label, percentage)
            };

            let text_style = TextStyle::new()
                .with_font_size(theme.tick_font_size())
                .with_font_family(theme.font_family())
                .with_fill(FillStyle::Color(theme.foreground_color().to_string()));

            output.add_command(DrawCmd::Text {
                x: label_x,
                y: label_y,
                content: label_text,
                style: text_style,
                anchor: TextAnchor::Middle,
                baseline: TextBaseline::Middle,
            });

            // HitRegion：用包围盒覆盖扇形区域
            let region = HitRegion::from_rect(
                cx - radius,
                cy - radius,
                radius * 2.0,
                radius * 2.0,
                slice.row_idx,
                Some(idx),
                Self::collect_row_data(data, slice.row_idx),
            );
            hit_regions.push(region);

            current_angle = end_angle;
        }

        (output, hit_regions)
    }

    /// 收集行数据
    fn collect_row_data(data: &DataTable, row_idx: usize) -> Vec<FieldValue> {
        let mut row_data = Vec::new();
        for column in &data.columns {
            if let Some(value) = column.get(row_idx) {
                row_data.push(value.clone());
            }
        }
        row_data
    }

    /// 渲染背景
    /// 渲染标题
    fn render_title<T: Theme>(
        _spec: &ChartSpec,
        theme: &T,
        title: &str,
        _plot_area: &PlotArea,
    ) -> RenderOutput {
        let mut output = RenderOutput::new();

        let title_style = TextStyle::new()
            .with_font_size(theme.title_font_size())
            .with_font_family(theme.font_family())
            .with_font_weight(FontWeight::Bold)
            .with_fill(FillStyle::Color(theme.title_color().to_string()));

        output.add_command(DrawCmd::Text {
            x: _spec.width / 2.0,
            y: theme.margin().top / 2.0,
            content: title.to_string(),
            style: title_style,
            anchor: TextAnchor::Middle,
            baseline: TextBaseline::Middle,
        });

        output
    }
}

/// 切片数据
struct SliceData {
    /// 标签
    label: String,
    /// 原始数值
    value: f64,
    /// 占比 (0-1)
    fraction: f64,
    /// 行索引
    row_idx: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{Encoding, Field, Mark};
    use crate::theme::DefaultTheme;
    use deneb_core::{Column, DataType, FieldValue};

    fn create_pie_data() -> DataTable {
        DataTable::with_columns(vec![
            Column::new(
                "category",
                DataType::Nominal,
                vec![
                    FieldValue::Text("A".to_string()),
                    FieldValue::Text("B".to_string()),
                    FieldValue::Text("C".to_string()),
                ],
            ),
            Column::new(
                "value",
                DataType::Quantitative,
                vec![
                    FieldValue::Numeric(30.0),
                    FieldValue::Numeric(50.0),
                    FieldValue::Numeric(20.0),
                ],
            ),
        ])
    }

    fn create_pie_spec() -> ChartSpec {
        ChartSpec {
            mark: Mark::Pie,
            encoding: Encoding::new()
                .x(Field::nominal("category"))
                .y(Field::quantitative("value"))
                .color(Field::nominal("category")),
            title: None,
            width: 400.0,
            height: 300.0,
        }
    }

    #[test]
    fn test_pie_chart_render_basic() {
        let spec = create_pie_spec();
        let theme = DefaultTheme;
        let data = create_pie_data();

        let result = PieChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.hit_regions.len(), 3);
    }

    #[test]
    fn test_pie_chart_render_empty() {
        let spec = create_pie_spec();
        let theme = DefaultTheme;
        let data = DataTable::with_columns(vec![
            Column::new("category", DataType::Nominal, vec![]),
            Column::new("value", DataType::Quantitative, vec![]),
        ]);

        let result = PieChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.hit_regions.is_empty());
    }

    #[test]
    fn test_pie_chart_single_slice() {
        let spec = create_pie_spec();
        let theme = DefaultTheme;
        let data = DataTable::with_columns(vec![
            Column::new(
                "category",
                DataType::Nominal,
                vec![FieldValue::Text("A".to_string())],
            ),
            Column::new(
                "value",
                DataType::Quantitative,
                vec![FieldValue::Numeric(100.0)],
            ),
        ]);

        let result = PieChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.hit_regions.len(), 1);
    }

    #[test]
    fn test_pie_chart_all_zeros() {
        let spec = create_pie_spec();
        let theme = DefaultTheme;
        let data = DataTable::with_columns(vec![
            Column::new(
                "category",
                DataType::Nominal,
                vec![
                    FieldValue::Text("A".to_string()),
                    FieldValue::Text("B".to_string()),
                ],
            ),
            Column::new(
                "value",
                DataType::Quantitative,
                vec![
                    FieldValue::Numeric(0.0),
                    FieldValue::Numeric(0.0),
                ],
            ),
        ]);

        let result = PieChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        // 全零时等分
        assert_eq!(output.hit_regions.len(), 2);
    }

    #[test]
    fn test_pie_chart_with_title() {
        let spec = ChartSpec {
            mark: Mark::Pie,
            encoding: Encoding::new()
                .x(Field::nominal("category"))
                .y(Field::quantitative("value"))
                .color(Field::nominal("category")),
            title: Some("Pie Chart".to_string()),
            width: 400.0,
            height: 300.0,
        };

        let theme = DefaultTheme;
        let data = create_pie_data();

        let result = PieChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        let title_layer = output.layers.get_layer(LayerKind::Title);
        assert!(title_layer.is_some());
        assert!(!title_layer.unwrap().commands.is_empty());
    }

    #[test]
    fn test_pie_chart_layers() {
        let spec = create_pie_spec();
        let theme = DefaultTheme;
        let data = create_pie_data();

        let result = PieChart::render(&spec, &theme, &data).unwrap();

        assert!(result.layers.get_layer(LayerKind::Background).is_some());
        assert!(result.layers.get_layer(LayerKind::Data).is_some());
    }
}
