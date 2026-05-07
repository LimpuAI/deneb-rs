//! K 线图渲染器
//!
//! 将 ChartSpec 和 DataTable 渲染为 K 线图的 Canvas 指令。
//! 矩形实体表示 open-close，线条影线表示 high-low。

use crate::layout::{compute_layout, PlotArea};
use crate::spec::ChartSpec;
use crate::theme::Theme;
use crate::chart::ChartOutput;
use crate::error::ComponentError;
use deneb_core::*;

/// CandlestickChart 渲染器
pub struct CandlestickChart;

/// K 线单条数据
struct Candle {
    /// 类别/时间标签
    label: String,
    /// 开盘价
    open: f64,
    /// 最高价
    high: f64,
    /// 最低价
    low: f64,
    /// 收盘价
    close: f64,
}

impl CandlestickChart {
    /// 渲染 K 线图
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

        // 2. 提取 OHLC 数据
        let candles = Self::extract_candles(spec, data)?;

        if candles.is_empty() {
            return Ok(Self::render_empty(spec, theme));
        }

        // 3. 计算布局
        let layout = compute_layout(spec, theme, data);
        let plot_area = &layout.plot_area;

        // 4. 构建 Scale
        let (x_scale, y_scale) = Self::build_scales(&candles, plot_area)?;

        // 5. 生成渲染指令
        let mut layers = RenderLayers::new();
        let mut hit_regions = Vec::new();

        // 背景层
        layers.update_layer(LayerKind::Background, super::shared::render_background_with_border(spec, theme, plot_area));

        // 网格层
        if let Some(y_axis) = &layout.y_axis {
            layers.update_layer(LayerKind::Grid, super::shared::render_grid_horizontal(theme, &y_axis.tick_positions, plot_area));
        }

        // 数据层
        let (data_commands, candle_regions) = Self::render_candles(
            theme, &x_scale, &y_scale, &candles, data,
        )?;
        layers.update_layer(LayerKind::Data, data_commands);
        hit_regions.extend(candle_regions);

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

    /// 验证数据 — 需要 x + open + high + low + close
    fn validate_data(spec: &ChartSpec, data: &DataTable) -> Result<(), ComponentError> {
        if spec.encoding.x.is_none() {
            return Err(ComponentError::InvalidConfig {
                reason: "x encoding is required for candlestick".to_string(),
            });
        }

        for field_name in &["open", "high", "low", "close"] {
            let field = match *field_name {
                "open" => &spec.encoding.open,
                "high" => &spec.encoding.high,
                "low" => &spec.encoding.low,
                "close" => &spec.encoding.close,
                _ => unreachable!(),
            };

            if let Some(f) = field {
                if data.get_column(&f.name).is_none() {
                    return Err(ComponentError::InvalidConfig {
                        reason: format!("{} field '{}' not found in data", field_name, f.name),
                    });
                }
            } else {
                return Err(ComponentError::InvalidConfig {
                    reason: format!("{} encoding is required for candlestick", field_name),
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

    /// 提取 OHLC 数据
    fn extract_candles(
        spec: &ChartSpec,
        data: &DataTable,
    ) -> Result<Vec<Candle>, ComponentError> {
        let x_field = spec.encoding.x.as_ref().ok_or_else(|| ComponentError::InvalidConfig {
            reason: "x encoding is required".to_string(),
        })?;
        let open_field = spec.encoding.open.as_ref().ok_or_else(|| ComponentError::InvalidConfig {
            reason: "open encoding is required".to_string(),
        })?;
        let high_field = spec.encoding.high.as_ref().ok_or_else(|| ComponentError::InvalidConfig {
            reason: "high encoding is required".to_string(),
        })?;
        let low_field = spec.encoding.low.as_ref().ok_or_else(|| ComponentError::InvalidConfig {
            reason: "low encoding is required".to_string(),
        })?;
        let close_field = spec.encoding.close.as_ref().ok_or_else(|| ComponentError::InvalidConfig {
            reason: "close encoding is required".to_string(),
        })?;

        let x_col = data.get_column(&x_field.name).ok_or_else(|| ComponentError::InvalidConfig {
            reason: format!("x field '{}' not found in data", x_field.name),
        })?;
        let open_col = data.get_column(&open_field.name).ok_or_else(|| ComponentError::InvalidConfig {
            reason: format!("open field '{}' not found in data", open_field.name),
        })?;
        let high_col = data.get_column(&high_field.name).ok_or_else(|| ComponentError::InvalidConfig {
            reason: format!("high field '{}' not found in data", high_field.name),
        })?;
        let low_col = data.get_column(&low_field.name).ok_or_else(|| ComponentError::InvalidConfig {
            reason: format!("low field '{}' not found in data", low_field.name),
        })?;
        let close_col = data.get_column(&close_field.name).ok_or_else(|| ComponentError::InvalidConfig {
            reason: format!("close field '{}' not found in data", close_field.name),
        })?;

        let mut candles = Vec::new();
        let n = data.row_count();

        for i in 0..n {
            let label = x_col
                .get(i)
                .map(|v| {
                    v.as_text()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| format!("{}", i))
                })
                .unwrap_or_else(|| format!("{}", i));

            let open = open_col.get(i).and_then(|v| v.as_numeric()).unwrap_or(0.0);
            let high = high_col.get(i).and_then(|v| v.as_numeric()).unwrap_or(0.0);
            let low = low_col.get(i).and_then(|v| v.as_numeric()).unwrap_or(0.0);
            let close = close_col.get(i).and_then(|v| v.as_numeric()).unwrap_or(0.0);

            candles.push(Candle {
                label,
                open,
                high,
                low,
                close,
            });
        }

        Ok(candles)
    }

    /// 构建比例尺
    fn build_scales(
        candles: &[Candle],
        plot_area: &PlotArea,
    ) -> Result<(BandScale, LinearScale), ComponentError> {
        let categories: Vec<String> = candles.iter().map(|c| c.label.clone()).collect();
        let x_scale = BandScale::new(
            categories,
            plot_area.x,
            plot_area.x + plot_area.width,
            0.1,
        );

        // Y 轴范围覆盖所有 high/low，不从 0 开始
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;

        for c in candles {
            min = min.min(c.low);
            max = max.max(c.high);
        }

        if min > max {
            min = 0.0;
            max = 100.0;
        }

        // K 线图 Y 轴不从 0 开始（位置编码）
        let y_scale = LinearScale::new(
            min,
            max,
            plot_area.y + plot_area.height,
            plot_area.y,
        );

        Ok((x_scale, y_scale))
    }

    /// 渲染 K 线（实体 + 影线）
    fn render_candles<T: Theme>(
        _theme: &T,
        x_scale: &BandScale,
        y_scale: &LinearScale,
        candles: &[Candle],
        data: &DataTable,
    ) -> Result<(RenderOutput, Vec<HitRegion>), ComponentError> {
        let mut output = RenderOutput::new();
        let mut hit_regions = Vec::new();

        let body_fill_ratio = 0.7; // body 宽度占 band 的比例

        for (i, candle) in candles.iter().enumerate() {
            let band_center = x_scale.band_center(&candle.label).ok_or_else(|| {
                ComponentError::InvalidConfig {
                    reason: format!("category '{}' not found in x scale", candle.label),
                }
            })?;

            let band_w = x_scale.band_width();
            let body_width = band_w * body_fill_ratio;
            let body_x = band_center - body_width / 2.0;

            let bullish = candle.close >= candle.open;
            let body_color = if bullish {
                "#4caf50".to_string() // 绿色
            } else {
                "#f44336".to_string() // 红色
            };

            let wick_color = body_color.clone();

            // 影线（high-low 线）
            let high_y = y_scale.map(candle.high);
            let low_y = y_scale.map(candle.low);

            output.add_command(DrawCmd::Path {
                segments: vec![
                    PathSegment::MoveTo(band_center, high_y),
                    PathSegment::LineTo(band_center, low_y),
                ],
                fill: None,
                stroke: Some(StrokeStyle::Color(wick_color)),
            });

            // 实体（open-close 矩形）
            let open_y = y_scale.map(candle.open);
            let close_y = y_scale.map(candle.close);

            let body_top = open_y.min(close_y);
            let body_height = (close_y - open_y).abs().max(1.0); // open == close → 1px 线

            output.add_command(DrawCmd::Rect {
                x: body_x,
                y: body_top,
                width: body_width,
                height: body_height,
                fill: Some(FillStyle::Color(body_color)),
                stroke: None,
                corner_radius: None,
            });

            // HitRegion 覆盖整个 K 线区域
            let region_top = high_y;
            let region_height = (low_y - high_y).max(1.0);

            let row_data: Vec<FieldValue> = if i < data.row_count() {
                data.columns
                    .iter()
                    .filter_map(|col| col.values.get(i).cloned())
                    .collect()
            } else {
                vec![]
            };

            let region = HitRegion::from_rect(
                body_x,
                region_top,
                body_width,
                region_height,
                i,
                None,
                row_data,
            );
            hit_regions.push(region);
        }

        Ok((output, hit_regions))
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
                "date",
                DataType::Nominal,
                vec![
                    FieldValue::Text("Mon".to_string()),
                    FieldValue::Text("Tue".to_string()),
                    FieldValue::Text("Wed".to_string()),
                ],
            ),
            Column::new(
                "open",
                DataType::Quantitative,
                vec![
                    FieldValue::Numeric(100.0),
                    FieldValue::Numeric(110.0),
                    FieldValue::Numeric(105.0),
                ],
            ),
            Column::new(
                "high",
                DataType::Quantitative,
                vec![
                    FieldValue::Numeric(115.0),
                    FieldValue::Numeric(120.0),
                    FieldValue::Numeric(112.0),
                ],
            ),
            Column::new(
                "low",
                DataType::Quantitative,
                vec![
                    FieldValue::Numeric(95.0),
                    FieldValue::Numeric(100.0),
                    FieldValue::Numeric(98.0),
                ],
            ),
            Column::new(
                "close",
                DataType::Quantitative,
                vec![
                    FieldValue::Numeric(110.0),
                    FieldValue::Numeric(105.0),
                    FieldValue::Numeric(108.0),
                ],
            ),
        ])
    }

    fn create_test_spec() -> ChartSpec {
        ChartSpec::builder()
            .mark(Mark::Candlestick)
            .encoding(
                Encoding::new()
                    .x(Field::nominal("date"))
                    .y(Field::quantitative("close"))
                    .open(Field::quantitative("open"))
                    .high(Field::quantitative("high"))
                    .low(Field::quantitative("low"))
                    .close(Field::quantitative("close")),
            )
            .width(400.0)
            .height(300.0)
            .build()
            .unwrap()
    }

    #[test]
    fn test_candlestick_render_basic() {
        let spec = create_test_spec();
        let theme = DefaultTheme;
        let data = create_test_data();

        let result = CandlestickChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.hit_regions.len(), 3);
    }

    #[test]
    fn test_candlestick_render_empty_data() {
        let spec = create_test_spec();
        let theme = DefaultTheme;
        let data = DataTable::with_columns(vec![
            Column::new("date", DataType::Nominal, vec![]),
            Column::new("open", DataType::Quantitative, vec![]),
            Column::new("high", DataType::Quantitative, vec![]),
            Column::new("low", DataType::Quantitative, vec![]),
            Column::new("close", DataType::Quantitative, vec![]),
        ]);

        let result = CandlestickChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.hit_regions.is_empty());
        assert!(output.layers.get_layer(LayerKind::Background).is_some());
    }

    #[test]
    fn test_candlestick_missing_ohlc_field() {
        let spec = ChartSpec::builder()
            .mark(Mark::Candlestick)
            .encoding(
                Encoding::new()
                    .x(Field::nominal("date"))
                    .y(Field::quantitative("close")),
            )
            .width(400.0)
            .height(300.0)
            .build()
            .unwrap();

        let theme = DefaultTheme;
        let data = create_test_data();

        let result = CandlestickChart::render(&spec, &theme, &data);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("required for candlestick"));
    }

    #[test]
    fn test_candlestick_open_equals_close() {
        let data = DataTable::with_columns(vec![
            Column::new(
                "date",
                DataType::Nominal,
                vec![FieldValue::Text("Mon".to_string())],
            ),
            Column::new(
                "open",
                DataType::Quantitative,
                vec![FieldValue::Numeric(100.0)],
            ),
            Column::new(
                "high",
                DataType::Quantitative,
                vec![FieldValue::Numeric(105.0)],
            ),
            Column::new(
                "low",
                DataType::Quantitative,
                vec![FieldValue::Numeric(95.0)],
            ),
            Column::new(
                "close",
                DataType::Quantitative,
                vec![FieldValue::Numeric(100.0)],
            ),
        ]);

        let spec = create_test_spec();
        let theme = DefaultTheme;

        let result = CandlestickChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.hit_regions.len(), 1);
    }

    #[test]
    fn test_candlestick_layers() {
        let spec = create_test_spec();
        let theme = DefaultTheme;
        let data = create_test_data();

        let result = CandlestickChart::render(&spec, &theme, &data).unwrap();

        assert!(result.layers.get_layer(LayerKind::Background).is_some());
        assert!(result.layers.get_layer(LayerKind::Grid).is_some());
        assert!(result.layers.get_layer(LayerKind::Axis).is_some());
        assert!(result.layers.get_layer(LayerKind::Data).is_some());
    }

    #[test]
    fn test_candlestick_with_title() {
        let spec = ChartSpec::builder()
            .mark(Mark::Candlestick)
            .encoding(
                Encoding::new()
                    .x(Field::nominal("date"))
                    .y(Field::quantitative("close"))
                    .open(Field::quantitative("open"))
                    .high(Field::quantitative("high"))
                    .low(Field::quantitative("low"))
                    .close(Field::quantitative("close")),
            )
            .title("Stock Price")
            .width(400.0)
            .height(300.0)
            .build()
            .unwrap();

        let theme = DefaultTheme;
        let data = create_test_data();

        let result = CandlestickChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        let title_layer = output.layers.get_layer(LayerKind::Title);
        assert!(title_layer.is_some());
        assert!(!title_layer.unwrap().commands.is_empty());
    }
}
