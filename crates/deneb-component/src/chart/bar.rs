//! 柱状图渲染器
//!
//! 将 ChartSpec 和 DataTable 渲染为柱状图的 Canvas 指令。

use crate::layout::{compute_layout, PlotArea};
use crate::spec::ChartSpec;
use crate::theme::Theme;
use crate::chart::ChartOutput;
use crate::error::ComponentError;
use deneb_core::*;
use std::collections::HashMap;

/// BarChart 渲染器
pub struct BarChart;

/// 柱体圆角半径(逻辑像素)— §9.2 美学规格(统一圆角;顶角专属圆角待内部类型支持 per-corner 后跟进)
const BAR_CORNER_RADIUS: f64 = 4.0;

impl BarChart {
    /// 渲染柱状图(稳态,无动画/交互)
    ///
    /// # Arguments
    ///
    /// * `spec` - 图表规格
    /// * `theme` - 主题
    /// * `data` - 数据表
    ///
    /// # Returns
    ///
    /// 返回渲染结果和命中区域
    pub fn render<T: Theme>(
        spec: &ChartSpec,
        theme: &T,
        data: &DataTable,
    ) -> Result<ChartOutput, ComponentError> {
        Self::render_with_anim(spec, theme, data, &ChartAnim::steady())
    }

    /// 渲染柱状图(Tier 1 语义动画输入)
    ///
    /// `anim.enter_t` 驱动柱高生长(stagger 级联),`anim.state`/`state_t` 驱动
    /// hover 提亮与选中高亮/其余置灰。稳态等价于 `render`。
    pub fn render_with_anim<T: Theme>(
        spec: &ChartSpec,
        theme: &T,
        data: &DataTable,
        anim: &ChartAnim,
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

        // 4. 按系列分组（如果有 color encoding）
        let series_data = Self::group_by_series(spec, data)?;

        // 5. 生成渲染指令
        let mut layers = RenderLayers::new();
        let mut hit_regions = Vec::new();

        // 背景层
        layers.update_layer(LayerKind::Background, super::shared::render_background_with_border(spec, theme, plot_area));

        // 网格层
        if let Some(y_axis) = &layout.y_axis {
            layers.update_layer(LayerKind::Grid, super::shared::render_grid_horizontal(theme, &y_axis.tick_positions, plot_area));
        }

        // 数据层（柱子）
        let (data_commands, bar_regions) = Self::render_bars(
            spec,
            theme,
            &x_scale,
            &y_scale,
            &series_data,
            plot_area,
            anim,
        )?;
        layers.update_layer(LayerKind::Data, data_commands);
        hit_regions.extend(bar_regions);

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

    /// 仅重算数据层 + 命中区(状态过渡路径:静态层走会话缓存)
    ///
    /// 重新计算 scales/分组是廉价的 O(n);命中区返回稳态几何(与全量渲染一致)。
    pub fn render_data_layer<T: Theme>(
        spec: &ChartSpec,
        theme: &T,
        data: &DataTable,
        anim: &ChartAnim,
    ) -> Result<(RenderOutput, Vec<HitRegion>), ComponentError> {
        Self::validate_data(spec, data)?;
        if data.is_empty() || data.row_count() == 0 {
            return Ok((RenderOutput::new(), Vec::new()));
        }
        let layout = compute_layout(spec, theme, data);
        let plot_area = &layout.plot_area;
        let (x_scale, y_scale) = Self::build_scales(spec, data, plot_area)?;
        let series_data = Self::group_by_series(spec, data)?;
        Self::render_bars(spec, theme, &x_scale, &y_scale, &series_data, plot_area, anim)
    }

    /// 验证数据
    fn validate_data(spec: &ChartSpec, data: &DataTable) -> Result<(), ComponentError> {
        // 检查必需的字段是否存在
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

        if let Some(color_field) = &spec.encoding.color {
            if data.get_column(&color_field.name).is_none() {
                return Err(ComponentError::InvalidConfig {
                    reason: format!("color field '{}' not found in data", color_field.name),
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

        // 背景层
        layers.update_layer(LayerKind::Background, super::shared::render_background_with_border(spec, theme, &plot_area));

        // 标题层
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
        // X 轴：BandScale（类别列）
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

        // 获取唯一类别(保持数据首现顺序 — 确定性:HashSet 随机序会导致
        // 每次重渲染柱位洗牌,hover/click 重渲染时图表"乱跳")
        let mut seen = std::collections::HashSet::new();
        let categories: Vec<String> = x_column
            .values
            .iter()
            .filter_map(|v| v.as_text().map(|s| s.to_string()))
            .filter(|s| seen.insert(s.clone()))
            .collect();

        let x_scale = BandScale::new(
            categories,
            plot_area.x,
            plot_area.x + plot_area.width,
            0.3, // 类目间隙比(§9.2:ECharts barCategoryGap 校准)
        );

        // Y 轴：LinearScale（数值列）
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

        // 获取数值范围
        let mut min: Option<f64> = None;
        let mut max: Option<f64> = None;

        for value in &y_column.values {
            if let Some(num) = value.as_numeric() {
                min = Some(min.map_or(num, |m| m.min(num)));
                max = Some(max.map_or(num, |m| m.max(num)));
            }
        }

        // 如果全是空值，使用默认范围
        let (min, max) = match (min, max) {
            (Some(min), Some(max)) => (min, max),
            _ => (0.0, 100.0),
        };

        // 确保 0 在范围内（处理负值）
        let min = min.min(0.0);
        let max = max.max(0.0);

        // Y 轴 range 是反向的（底部=max_y，顶部=min_y）
        let y_scale = LinearScale::new(
            min,
            max,
            plot_area.y + plot_area.height,
            plot_area.y,
        );

        Ok((x_scale, y_scale))
    }

    /// 按系列分组数据(保持系列首现顺序 — 确定性,见 build_scales 注释)
    fn group_by_series(
        spec: &ChartSpec,
        data: &DataTable,
    ) -> Result<Vec<(Option<String>, Vec<(String, f64, usize, Vec<FieldValue>)>)>, ComponentError> {
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

        let color_column = spec.encoding.color.as_ref()
            .and_then(|field| data.get_column(&field.name));

        let mut series_map: HashMap<Option<String>, Vec<(String, f64, usize, Vec<FieldValue>)>> = HashMap::new();
        let mut series_order: Vec<Option<String>> = Vec::new();

        let row_count = data.row_count();
        for row_idx in 0..row_count {
            // 获取 x 值
            let x_value = x_column
                .get(row_idx)
                .and_then(|v| v.as_text())
                .unwrap_or("")
                .to_string();

            // 获取 y 值
            let y_value = y_column
                .get(row_idx)
                .and_then(|v| v.as_numeric())
                .unwrap_or(0.0);

            // 获取系列值
            let series = color_column
                .and_then(|col| col.get(row_idx))
                .and_then(|v| v.as_text())
                .map(|s| s.to_string());

            // 收集该行的所有字段值
            let mut row_data = Vec::new();
            for column in &data.columns {
                if let Some(value) = column.get(row_idx) {
                    row_data.push(value.clone());
                }
            }

            if !series_map.contains_key(&series) {
                series_order.push(series.clone());
            }
            series_map
                .entry(series)
                .or_insert_with(Vec::new)
                .push((x_value, y_value, row_idx, row_data));
        }

        Ok(series_order
            .into_iter()
            .map(|key| {
                let bars = series_map.remove(&key).unwrap_or_default();
                (key, bars)
            })
            .collect())
    }

    /// 渲染柱子(Tier 1 动画 + 交互渲染态)
    ///
    /// - 入场:柱高 = 最终高度 × ease(item_progress(t))(stagger 级联)
    /// - hover:提亮(hover_boost 向 hover_color 语义色/白色混合,即时)
    /// - selected:满色 + focus outline stroke;其余项 alpha × dim_factor(state_t 插值)
    /// - 命中区恒为稳态几何(动画期间交互按最终位置命中)
    #[allow(clippy::too_many_arguments)]
    fn render_bars<T: Theme>(
        _spec: &ChartSpec,
        theme: &T,
        x_scale: &BandScale,
        y_scale: &LinearScale,
        series_data: &[(Option<String>, Vec<(String, f64, usize, Vec<FieldValue>)>)],
        _plot_area: &PlotArea,
        anim: &ChartAnim,
    ) -> Result<(RenderOutput, Vec<HitRegion>), ComponentError> {
        let mut output = RenderOutput::new();
        let mut hit_regions = Vec::new();

        let series_count = series_data.len();

        // 计算基线位置（y=0 对应的像素位置）
        let baseline = y_scale.map(0.0);
        let dim = anim.dim_factor();

        for (series_idx, (_series_key, bars)) in series_data.iter().enumerate() {

            for (bar_idx, (category, value, row_idx, row_data)) in bars.iter().enumerate() {
                // 多系列按系列分色，单系列按类别分色
                let base_color = if series_count > 1 {
                    theme.series_color(series_idx).to_string()
                } else {
                    theme.series_color(bar_idx).to_string()
                };
                // 计算柱子位置
                let band_start = x_scale.band_start(category).ok_or_else(|| {
                    ComponentError::InvalidConfig {
                        reason: format!("category not found in x scale: {}", category),
                    }
                })?;

                let band_width = x_scale.band_width();

                // 多系列时，将每个 band 细分
                let (bar_x, bar_width) = if series_count > 1 {
                    let sub_width = band_width / series_count as f64;
                    let offset = series_idx as f64 * sub_width;
                    (band_start + offset, sub_width)
                } else {
                    (band_start, band_width)
                };

                // 最终（稳态）几何
                let y_final = y_scale.map(*value);
                let (final_y, final_height) = if *value >= 0.0 {
                    let height = baseline - y_final;
                    (y_final, height.max(1.0)) // 至少 1px
                } else {
                    let height = y_final - baseline;
                    (baseline, height.max(1.0))
                };

                // 入场生长:局部相位 → 缓动 → 高度缩放(从基线生长)
                let growth = anim.easing.apply(anim.item_progress(bar_idx, anim.enter_t));
                let (bar_y, bar_height) = if *value >= 0.0 {
                    let h = final_height * growth;
                    (baseline - h, h)
                } else {
                    (baseline, final_height * growth)
                };

                // 交互渲染态:色相处理顺序 = hover 提亮 → 未选中置灰
                let mut fill_color = base_color.clone();
                if anim.state.hovered == Some(*row_idx) {
                    // hover 提亮:theme.hover_color 语义色为基准(None 时向白混合近似)
                    fill_color = anim.hover_brighten(&fill_color);
                }
                let selected = anim.state.is_selected(*row_idx);
                // hairline outline:形状+颜色承载语义,线宽真实消费 outline_width
                // (design §9.4;convert 链把 WithWidth 的线宽传给宿主)
                let stroke = if selected {
                    Some(StrokeStyle::WithWidth {
                        color: anim.outline_color.clone(),
                        width: anim.outline_width,
                    })
                } else {
                    None
                };
                if anim.state.has_selection() && !selected {
                    fill_color = with_alpha(&fill_color, dim);
                }

                // 绘制柱子(growth=0 时高度为 0,跳过绘制避免 1px 残影)
                if bar_height > 0.5 {
                    output.add_command(DrawCmd::Rect {
                        x: bar_x,
                        y: bar_y,
                        width: bar_width,
                        height: bar_height,
                        fill: Some(FillStyle::Color(fill_color)),
                        stroke,
                        corner_radius: Some(BAR_CORNER_RADIUS),
                        corner_radii: None,
                        // id = 命中区索引(命令身份,宿主 per-item 声明式效果关联键)
                        id: Some(*row_idx as u32),
});
                }

                // 创建 HitRegion(稳态几何;声明式 hover:柱体提亮 8%)
                let region = HitRegion::from_rect(
                    bar_x,
                    final_y,
                    bar_width,
                    final_height,
                    *row_idx,
                    if series_count > 1 { Some(series_idx) } else { None },
                    row_data.clone(),
                )
                .with_hover(HoverEffect::brighten(0.08));
                hit_regions.push(region);
            }
        }

        Ok((output, hit_regions))
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{Encoding, Mark, Field};
    use deneb_core::{Column, DataType, FieldValue};
    use crate::theme::DefaultTheme;

    fn create_test_data() -> DataTable {
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
                    FieldValue::Numeric(10.0),
                    FieldValue::Numeric(20.0),
                    FieldValue::Numeric(15.0),
                ],
            ),
        ])
    }

    fn create_test_spec() -> ChartSpec {
        ChartSpec::builder()
            .mark(Mark::Bar)
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
    fn test_bar_chart_render_basic() {
        let spec = create_test_spec();
        let theme = DefaultTheme;
        let data = create_test_data();

        let result = BarChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(!output.hit_regions.is_empty());
        assert_eq!(output.hit_regions.len(), 3); // 3 个柱子
    }

    #[test]
    fn test_y_axis_title_vertical_rotation() {
        // Y 轴标题必须竖排(270°,Middle/Middle 锚定),且锚点落在左侧
        // margin 条带内(不越过绘图区左缘、不出图表左边界)
        let spec = create_test_spec();
        let theme = DefaultTheme;
        let data = create_test_data();
        let output = BarChart::render(&spec, &theme, &data).unwrap();

        let title_cmd = output
            .layers
            .all()
            .iter()
            .flat_map(|l| l.commands.semantic.iter())
            .find_map(|cmd| match cmd {
                DrawCmd::Text { x, y, content, style, anchor, baseline }
                    if content == "value" =>
                {
                    Some((*x, *y, style.rotation, anchor, baseline))
                }
                _ => None,
            })
            .expect("y-axis title command exists");

        let (x, _y, rotation, anchor, baseline) = title_cmd;
        assert!(
            (rotation - 270.0).abs() < f64::EPSILON,
            "y title must be rotated 270deg, got {rotation}"
        );
        assert!(matches!(anchor, TextAnchor::Middle));
        assert!(matches!(baseline, TextBaseline::Middle));
        // 左 margin 50(默认主题):锚点应在图表左边界与绘图区左缘之间
        let plot_left = theme.margin().left;
        assert!(
            x > 0.0 && x < plot_left,
            "y title anchor {x} should sit inside left margin strip (0, {plot_left})"
        );
    }

    #[test]
    fn test_bar_chart_render_with_title() {
        let spec = ChartSpec::builder()
            .mark(Mark::Bar)
            .encoding(
                Encoding::new()
                    .x(Field::nominal("category"))
                    .y(Field::quantitative("value")),
            )
            .title("Test Chart")
            .width(400.0)
            .height(300.0)
            .build()
            .unwrap();

        let theme = DefaultTheme;
        let data = create_test_data();

        let result = BarChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        // 检查标题层是否有内容
        let title_layer = output.layers.get_layer(LayerKind::Title);
        assert!(title_layer.is_some());
        assert!(!title_layer.unwrap().commands.is_empty());
    }

    #[test]
    fn test_bar_chart_render_empty_data() {
        let spec = create_test_spec();
        let theme = DefaultTheme;
        let data = DataTable::with_columns(vec![
            Column::new("category", DataType::Nominal, vec![]),
            Column::new("value", DataType::Quantitative, vec![]),
        ]);

        let result = BarChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.hit_regions.is_empty());
        // 背景层应该存在
        assert!(output.layers.get_layer(LayerKind::Background).is_some());
    }

    #[test]
    fn test_bar_chart_render_single_category() {
        let spec = create_test_spec();
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
                vec![FieldValue::Numeric(42.0)],
            ),
        ]);

        let result = BarChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.hit_regions.len(), 1);
    }

    #[test]
    fn test_bar_chart_render_negative_values() {
        let spec = create_test_spec();
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
                    FieldValue::Numeric(-10.0),
                    FieldValue::Numeric(20.0),
                ],
            ),
        ]);

        let result = BarChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.hit_regions.len(), 2);
    }

    #[test]
    fn test_bar_chart_render_multi_series() {
        let spec = ChartSpec::builder()
            .mark(Mark::Bar)
            .encoding(
                Encoding::new()
                    .x(Field::nominal("category"))
                    .y(Field::quantitative("value"))
                    .color(Field::nominal("series")),
            )
            .width(400.0)
            .height(300.0)
            .build()
            .unwrap();

        let theme = DefaultTheme;
        let data = DataTable::with_columns(vec![
            Column::new(
                "category",
                DataType::Nominal,
                vec![
                    FieldValue::Text("A".to_string()),
                    FieldValue::Text("A".to_string()),
                    FieldValue::Text("B".to_string()),
                    FieldValue::Text("B".to_string()),
                ],
            ),
            Column::new(
                "value",
                DataType::Quantitative,
                vec![
                    FieldValue::Numeric(10.0),
                    FieldValue::Numeric(15.0),
                    FieldValue::Numeric(20.0),
                    FieldValue::Numeric(25.0),
                ],
            ),
            Column::new(
                "series",
                DataType::Nominal,
                vec![
                    FieldValue::Text("X".to_string()),
                    FieldValue::Text("Y".to_string()),
                    FieldValue::Text("X".to_string()),
                    FieldValue::Text("Y".to_string()),
                ],
            ),
        ]);

        let result = BarChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.hit_regions.len(), 4);

        // 检查每个柱子都有系列索引
        for region in &output.hit_regions {
            assert!(region.series.is_some());
        }
    }

    #[test]
    fn test_bar_chart_hit_regions() {
        let spec = create_test_spec();
        let theme = DefaultTheme;
        let data = create_test_data();

        let result = BarChart::render(&spec, &theme, &data).unwrap();
        let regions = &result.hit_regions;

        assert_eq!(regions.len(), 3);

        // 检查第一个命中区域
        let region = &regions[0];
        assert_eq!(region.index, 0);
        assert!(region.series.is_none()); // 单系列
        assert!(!region.data.is_empty());
    }

    #[test]
    fn test_bar_chart_validate_data_missing_field() {
        let spec = create_test_spec();
        let theme = DefaultTheme;
        let data = DataTable::with_columns(vec![
            Column::new("wrong_field", DataType::Nominal, vec![]),
        ]);

        let result = BarChart::render(&spec, &theme, &data);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn test_bar_chart_zero_value() {
        let spec = create_test_spec();
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
                vec![FieldValue::Numeric(0.0)],
            ),
        ]);

        let result = BarChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.hit_regions.len(), 1);

        // 零值的柱子应该至少有 1px 高度
        let region = &output.hit_regions[0];
        assert!(region.bounds.height >= 1.0);
    }

    #[test]
    fn test_bar_chart_layers() {
        let spec = create_test_spec();
        let theme = DefaultTheme;
        let data = create_test_data();

        let result = BarChart::render(&spec, &theme, &data).unwrap();

        // 检查所有必需的层都存在
        assert!(result.layers.get_layer(LayerKind::Background).is_some());
        assert!(result.layers.get_layer(LayerKind::Grid).is_some());
        assert!(result.layers.get_layer(LayerKind::Axis).is_some());
        assert!(result.layers.get_layer(LayerKind::Data).is_some());
    }

    #[test]
    fn test_render_deterministic_across_calls() {
        // 回归:HashSet 随机序导致每次渲染柱位洗牌(hover 重渲染时图表乱跳)
        let spec = create_test_spec();
        let theme = DefaultTheme;
        let data = create_test_data();

        let first = BarChart::render(&spec, &theme, &data).unwrap();
        let rects_of = |out: &ChartOutput| -> Vec<(f64, f64)> {
            out.layers
                .get_layer(LayerKind::Data)
                .unwrap()
                .commands
                .semantic
                .iter()
                .map(|c| match c {
                    DrawCmd::Rect { x, height, .. } => (*x, *height),
                    _ => (0.0, 0.0),
                })
                .collect()
        };
        let r1 = rects_of(&first);
        for _ in 0..8 {
            let again = BarChart::render(&spec, &theme, &data).unwrap();
            assert_eq!(r1, rects_of(&again), "柱位/柱高必须跨渲染确定");
        }
    }

    #[test]
    fn test_bar_chart_with_custom_theme() {
        use crate::theme::DarkTheme;

        let spec = create_test_spec();
        let theme = DarkTheme;
        let data = create_test_data();

        let result = BarChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(!output.hit_regions.is_empty());
    }
}
