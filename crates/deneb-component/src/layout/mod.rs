//! 通用布局引擎
//!
//! 负责计算图表内部各元素的位置和尺寸。

use crate::spec::{ChartSpec, Field};
use deneb_core::{
    BandScale, LinearScale, Scale, TimeScale,
};

/// 轴方向
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orientation {
    /// 顶部
    Top,
    /// 底部
    Bottom,
    /// 左侧
    Left,
    /// 右侧
    Right,
}

/// 绘图区域
#[derive(Clone, Debug, PartialEq)]
pub struct PlotArea {
    /// X 坐标
    pub x: f64,
    /// Y 坐标
    pub y: f64,
    /// 宽度
    pub width: f64,
    /// 高度
    pub height: f64,
}

/// 轴布局
#[derive(Clone, Debug)]
pub struct AxisLayout {
    /// 轴的方向
    pub orientation: Orientation,
    /// 轴线位置（像素坐标）
    pub position: f64,
    /// 刻度位置（像素坐标列表）
    pub tick_positions: Vec<f64>,
    /// 刻度标签文本
    pub tick_labels: Vec<String>,
}

/// 布局计算结果
#[derive(Clone, Debug)]
pub struct LayoutResult {
    /// 绘图区域（不含 margin）
    pub plot_area: PlotArea,
    /// X 轴配置
    pub x_axis: Option<AxisLayout>,
    /// Y 轴配置
    pub y_axis: Option<AxisLayout>,
}

/// 计算刻度数量和大小的辅助
pub struct TickCalculator;

impl TickCalculator {
    /// 根据数据范围和可用像素宽度计算合适的刻度
    /// 返回 (刻度值列表, 格式化标签列表)
    pub fn calculate_linear_ticks(
        min: f64,
        max: f64,
        max_ticks: usize,
    ) -> (Vec<f64>, Vec<String>) {
        if min >= max {
            // 范围无效，返回单个刻度
            return (vec![min], vec![Self::format_number(min)]);
        }

        let range = max - min;

        // 计算步长：选择"漂亮"的数字 (1, 2, 5, 10, 20, 50, ...)
        let rough_step = range / max_ticks as f64;
        let power_of_10 = 10.0f64.powf(rough_step.log10().floor());
        let normalized_step = rough_step / power_of_10;

        let nice_step = if normalized_step < 1.5 {
            1.0
        } else if normalized_step < 3.0 {
            2.0
        } else if normalized_step < 7.0 {
            5.0
        } else {
            10.0
        };

        let step = nice_step * power_of_10;

        // 计算刻度起始值（对齐到步长的整数倍）
        let start = (min / step).floor() * step;
        let end = (max / step).ceil() * step;

        // 生成刻度
        let mut ticks = Vec::new();
        let mut current = start;
        while current <= end + 1e-10 {
            // 加小量避免浮点误差
            ticks.push(current);
            current += step;
        }

        // 格式化标签
        let labels: Vec<String> = ticks.iter().map(|&v| Self::format_number(v)).collect();

        (ticks, labels)
    }

    /// 时间刻度计算
    pub fn calculate_time_ticks(
        min: f64,
        max: f64,
        max_ticks: usize,
    ) -> (Vec<f64>, Vec<String>) {
        // 简化实现：使用线性刻度，然后格式化为时间
        let (ticks, _) = Self::calculate_linear_ticks(min, max, max_ticks);
        let labels: Vec<String> = ticks.iter().map(|&v| Self::format_timestamp(v)).collect();
        (ticks, labels)
    }

    /// 格式化数字：整数不带小数点，浮点数保留必要精度
    fn format_number(value: f64) -> String {
        if value.fract() == 0.0 && value.abs() < 1e10 {
            // 整数
            format!("{}", value as i64)
        } else if value.abs() < 1e-6 || value.abs() >= 1e10 {
            // 科学计数法
            format!("{:.2e}", value)
        } else if value.abs() < 0.01 {
            // 小数，保留足够精度
            format!("{:.6}", value).trim_end_matches('0').trim_end_matches('.').to_string()
        } else if value.abs() >= 1000.0 {
            // 大数字，保留合适精度
            format!("{:.2}", value).trim_end_matches('0').trim_end_matches('.').to_string()
        } else {
            // 普通浮点数
            format!("{:.2}", value).trim_end_matches('0').trim_end_matches('.').to_string()
        }
    }

    /// 格式化时间戳
    fn format_timestamp(timestamp: f64) -> String {
        // 简化实现：只显示秒数
        // 实际应用中应该使用 chrono 库格式化为可读时间
        format!("{:.0}", timestamp)
    }

    /// 计算离散刻度（用于序数和名义数据）
    pub fn calculate_discrete_ticks(categories: &[String]) -> (Vec<f64>, Vec<String>) {
        let count = categories.len();
        let ticks: Vec<f64> = (0..count).map(|i| i as f64).collect();
        let labels = categories.to_vec();
        (ticks, labels)
    }
}

/// 计算布局
pub fn compute_layout<T: crate::theme::Theme>(
    spec: &ChartSpec,
    theme: &T,
    data: &deneb_core::DataTable,
) -> LayoutResult {
    let padding = theme.padding();

    // 计算绘图区域
    let plot_area = PlotArea {
        x: padding.left,
        y: padding.top,
        width: spec.width - padding.horizontal(),
        height: spec.height - padding.vertical(),
    };

    // 计算 X 轴布局
    let x_axis = spec.encoding.x.as_ref().and_then(|x_field| {
        compute_axis_layout::<T>(
            x_field,
            data,
            &plot_area,
            Orientation::Bottom,
            true,
            theme,
        )
    });

    // 计算 Y 轴布局
    let y_axis = spec.encoding.y.as_ref().and_then(|y_field| {
        compute_axis_layout::<T>(
            y_field,
            data,
            &plot_area,
            Orientation::Left,
            false,
            theme,
        )
    });

    LayoutResult {
        plot_area,
        x_axis,
        y_axis,
    }
}

/// 计算轴布局
fn compute_axis_layout<T: crate::theme::Theme>(
    field: &Field,
    data: &deneb_core::DataTable,
    plot_area: &PlotArea,
    orientation: Orientation,
    is_horizontal: bool,
    _theme: &T,
) -> Option<AxisLayout> {
    // 从数据中获取列
    let column = data.get_column(&field.name)?;

    if column.is_empty() {
        return None;
    }

    // 计算刻度
    let (tick_positions, tick_labels) = match field.data_type {
        deneb_core::DataType::Quantitative => {
            if let Some((min, max)) = get_numeric_range(column) {
                let max_ticks = if is_horizontal {
                    (plot_area.width / 50.0) as usize
                } else {
                    (plot_area.height / 30.0) as usize
                };
                let max_ticks = max_ticks.clamp(3, 10);
                let (ticks, labels) = TickCalculator::calculate_linear_ticks(min, max, max_ticks);

                // 映射到像素空间
                let scale = if is_horizontal {
                    LinearScale::new(min, max, plot_area.x, plot_area.x + plot_area.width)
                } else {
                    LinearScale::new(
                        min,
                        max,
                        plot_area.y + plot_area.height,
                        plot_area.y,
                    )
                };

                let positions: Vec<f64> = ticks.iter().map(|&v| scale.map(v)).collect();
                (positions, labels)
            } else {
                return None;
            }
        }
        deneb_core::DataType::Temporal => {
            if let Some((min, max)) = get_numeric_range(column) {
                let max_ticks = if is_horizontal {
                    (plot_area.width / 80.0) as usize
                } else {
                    (plot_area.height / 40.0) as usize
                };
                let max_ticks = max_ticks.clamp(3, 8);
                let (ticks, labels) = TickCalculator::calculate_time_ticks(min, max, max_ticks);

                // 映射到像素空间
                let scale = if is_horizontal {
                    TimeScale::new(min, max, plot_area.x, plot_area.x + plot_area.width)
                } else {
                    TimeScale::new(
                        min,
                        max,
                        plot_area.y + plot_area.height,
                        plot_area.y,
                    )
                };

                let positions: Vec<f64> = ticks.iter().map(|&v| scale.map(v)).collect();
                (positions, labels)
            } else {
                return None;
            }
        }
        deneb_core::DataType::Nominal | deneb_core::DataType::Ordinal => {
            // 获取唯一类别
            let categories: Vec<String> = column
                .values
                .iter()
                .filter_map(|v| v.as_text().map(|s| s.to_string()))
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();

            if categories.is_empty() {
                return None;
            }

            let (_ticks, labels) = TickCalculator::calculate_discrete_ticks(&categories);

            // 使用 BandScale 映射到像素空间
            let scale = BandScale::new(
                categories.clone(),
                if is_horizontal {
                    plot_area.x
                } else {
                    plot_area.y
                },
                if is_horizontal {
                    plot_area.x + plot_area.width
                } else {
                    plot_area.y + plot_area.height
                },
                0.1,
            );

            let positions: Vec<f64> = categories
                .iter()
                .map(|cat| scale.band_center(cat).unwrap_or(0.0))
                .collect();

            (positions, labels)
        }
    };

    // 计算轴线位置
    let position = match orientation {
        Orientation::Top => plot_area.y,
        Orientation::Bottom => plot_area.y + plot_area.height,
        Orientation::Left => plot_area.x,
        Orientation::Right => plot_area.x + plot_area.width,
    };

    Some(AxisLayout {
        orientation,
        position,
        tick_positions,
        tick_labels,
    })
}

/// 获取数值列的范围
fn get_numeric_range(column: &deneb_core::Column) -> Option<(f64, f64)> {
    let mut min: Option<f64> = None;
    let mut max: Option<f64> = None;

    for value in &column.values {
        if let Some(num) = value.as_numeric() {
            min = Some(min.map_or(num, |m: f64| m.min(num)));
            max = Some(max.map_or(num, |m: f64| m.max(num)));
        } else if let Some(ts) = value.as_timestamp() {
            min = Some(min.map_or(ts, |m: f64| m.min(ts)));
            max = Some(max.map_or(ts, |m: f64| m.max(ts)));
        }
    }

    match (min, max) {
        (Some(min), Some(max)) => Some((min, max)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{Encoding, Mark};
    use crate::theme::Margin;
    use deneb_core::{Column, DataType, FieldValue};

    #[test]
    fn test_margin() {
        let margin = Margin::new(10.0, 20.0, 30.0, 40.0);
        assert_eq!(margin.horizontal(), 60.0);
        assert_eq!(margin.vertical(), 40.0);
    }

    #[test]
    fn test_tick_calculate_linear_ticks_basic() {
        let (ticks, labels) = TickCalculator::calculate_linear_ticks(0.0, 100.0, 5);
        assert_eq!(ticks, vec![0.0, 20.0, 40.0, 60.0, 80.0, 100.0]);
        assert_eq!(labels, vec!["0", "20", "40", "60", "80", "100"]);
    }

    #[test]
    fn test_tick_calculate_linear_ticks_small_range() {
        let (ticks, labels) = TickCalculator::calculate_linear_ticks(0.001, 0.009, 5);
        // 应该选择 0.002 作为步长
        assert_eq!(ticks, vec![0.0, 0.002, 0.004, 0.006, 0.008, 0.01]);
        assert!(labels[1].contains("002") || labels[1] == "0.002");
    }

    #[test]
    fn test_tick_calculate_linear_ticks_negative() {
        let (ticks, _labels) = TickCalculator::calculate_linear_ticks(-50.0, 50.0, 5);
        // 范围 100，要求 5 个刻度，步长约为 20
        // 从 -60 开始（对齐到 20 的倍数）
        assert_eq!(ticks, vec![-60.0, -40.0, -20.0, 0.0, 20.0, 40.0, 60.0]);
    }

    #[test]
    fn test_tick_calculate_linear_ticks_invalid_range() {
        let (ticks, labels) = TickCalculator::calculate_linear_ticks(100.0, 100.0, 5);
        assert_eq!(ticks, vec![100.0]);
        assert_eq!(labels, vec!["100"]);
    }

    #[test]
    fn test_tick_format_number() {
        assert_eq!(TickCalculator::format_number(42.0), "42");
        assert_eq!(TickCalculator::format_number(3.14159), "3.14");
        assert_eq!(TickCalculator::format_number(0.0001234), "0.000123");
        assert_eq!(TickCalculator::format_number(1000000.0), "1000000");
    }

    #[test]
    fn test_tick_calculate_discrete_ticks() {
        let categories = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let (ticks, labels) = TickCalculator::calculate_discrete_ticks(&categories);
        assert_eq!(ticks, vec![0.0, 1.0, 2.0]);
        assert_eq!(labels, categories);
    }

    #[test]
    fn test_compute_layout_basic() {
        let theme = crate::theme::DefaultTheme;
        let spec = ChartSpec::builder()
            .mark(Mark::Line)
            .encoding(
                Encoding::new()
                    .x(crate::spec::Field::quantitative("x"))
                    .y(crate::spec::Field::quantitative("y")),
            )
            .width(800.0)
            .height(400.0)
            .build()
            .unwrap();

        let data = deneb_core::DataTable::with_columns(vec![
            Column::new("x", DataType::Quantitative, vec![
                FieldValue::Numeric(0.0),
                FieldValue::Numeric(50.0),
                FieldValue::Numeric(100.0),
            ]),
            Column::new("y", DataType::Quantitative, vec![
                FieldValue::Numeric(0.0),
                FieldValue::Numeric(50.0),
                FieldValue::Numeric(100.0),
            ]),
        ]);

        let layout = compute_layout(&spec, &theme, &data);

        // 验证绘图区域
        assert_eq!(layout.plot_area.x, 50.0); // left padding
        assert_eq!(layout.plot_area.y, 30.0); // top padding
        assert_eq!(layout.plot_area.width, 730.0); // 800 - 50 - 20
        assert_eq!(layout.plot_area.height, 330.0); // 400 - 30 - 40

        // 验证轴存在
        assert!(layout.x_axis.is_some());
        assert!(layout.y_axis.is_some());

        // 验证 X 轴
        let x_axis = layout.x_axis.unwrap();
        assert_eq!(x_axis.orientation, Orientation::Bottom);
        assert_eq!(x_axis.position, 360.0); // 30 + 330

        // 验证 Y 轴
        let y_axis = layout.y_axis.unwrap();
        assert_eq!(y_axis.orientation, Orientation::Left);
        assert_eq!(y_axis.position, 50.0);
    }

    #[test]
    fn test_get_numeric_range() {
        let column = Column::new(
            "value",
            DataType::Quantitative,
            vec![
                FieldValue::Numeric(10.0),
                FieldValue::Numeric(20.0),
                FieldValue::Numeric(15.0),
            ],
        );

        let range = get_numeric_range(&column);
        assert_eq!(range, Some((10.0, 20.0)));
    }

    #[test]
    fn test_get_numeric_range_with_nulls() {
        let column = Column::new(
            "value",
            DataType::Quantitative,
            vec![
                FieldValue::Numeric(10.0),
                FieldValue::Null,
                FieldValue::Numeric(20.0),
            ],
        );

        let range = get_numeric_range(&column);
        assert_eq!(range, Some((10.0, 20.0)));
    }

    #[test]
    fn test_get_numeric_range_empty() {
        let column = Column::new("value", DataType::Quantitative, vec![]);
        let range = get_numeric_range(&column);
        assert!(range.is_none());
    }

    #[test]
    fn test_get_numeric_range_timestamp() {
        let column = Column::new(
            "time",
            DataType::Temporal,
            vec![
                FieldValue::Timestamp(1000.0),
                FieldValue::Timestamp(2000.0),
            ],
        );

        let range = get_numeric_range(&column);
        assert_eq!(range, Some((1000.0, 2000.0)));
    }

    #[test]
    fn test_layout_result_clone() {
        let layout = LayoutResult {
            plot_area: PlotArea {
                x: 10.0,
                y: 20.0,
                width: 100.0,
                height: 200.0,
            },
            x_axis: None,
            y_axis: None,
        };

        let layout2 = layout.clone();
        assert_eq!(layout.plot_area, layout2.plot_area);
    }
}
