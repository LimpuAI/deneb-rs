//! Contour 图渲染器
//!
//! 将 ChartSpec 和 DataTable 渲染为等高线图的 Canvas 指令。
//! 使用 deneb_core::algorithm::contour::marching_squares 提取等值线。

use crate::layout::PlotArea;
use crate::spec::ChartSpec;
use crate::theme::Theme;
use crate::chart::ChartOutput;
use crate::error::ComponentError;
use deneb_core::*;
use deneb_core::algorithm::contour::{marching_squares, close_open_path_at_boundary};
use deneb_core::scale::{LinearScale, Scale};

/// ContourChart 渲染器
pub struct ContourChart;

/// 默认等高线级数
const DEFAULT_NUM_LEVELS: usize = 8;

/// 等高线颜色梯度（从低到高）
const CONTOUR_COLORS: &[&str] = &[
    "#313695", "#4575b4", "#74add1", "#abd9e9",
    "#fee090", "#fdae61", "#f46d43", "#d73027", "#a50026",
];

impl ContourChart {
    /// 渲染等高线图
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

        // 2. 计算布局区域
        let padding = theme.margin();
        let plot_area = PlotArea {
            x: padding.left,
            y: padding.top,
            width: spec.width - padding.horizontal(),
            height: spec.height - padding.vertical(),
        };

        // 3. 提取数据点
        let points = Self::extract_points(spec, data)?;

        // 4. <3 个点 → 散点图
        if points.len() < 3 {
            return Self::render_as_scatter(spec, theme, &plot_area, &points);
        }

        // 5. 构建 2D 网格
        let (grid, _x_range, _y_range, x_scale, y_scale) = Self::build_grid(
            &points, &plot_area,
        );

        // 6. 计算等高线级别
        let levels = Self::compute_levels(&grid);

        // 7. 运行 marching squares
        let x_scr = (plot_area.x, plot_area.x + plot_area.width);
        let y_scr = (plot_area.y, plot_area.y + plot_area.height);
        let contours = marching_squares(&grid, &levels, x_scr, y_scr);

        // 8. 生成渲染指令
        let mut layers = RenderLayers::new();
        let mut hit_regions = Vec::new();

        layers.update_layer(LayerKind::Background, super::shared::render_background_with_border(spec, theme, &plot_area));

        let data_commands = Self::render_contours(theme, &contours, &grid, x_scr, y_scr);
        layers.update_layer(LayerKind::Data, data_commands);

        // 标题层
        if let Some(title) = &spec.title {
            layers.update_layer(LayerKind::Title, super::shared::render_title(theme, title, &plot_area));
        }

        // 为数据点创建 HitRegion
        for (idx, (px, py, _pv)) in points.iter().enumerate() {
            let screen_x = x_scale.map(*px);
            let screen_y = y_scale.map(*py);
            let region = HitRegion::from_rect(
                screen_x - 3.0,
                screen_y - 3.0,
                6.0,
                6.0,
                idx,
                None,
                vec![FieldValue::Numeric(*py)],
            );
            hit_regions.push(region);
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
        } else {
            return Err(ComponentError::InvalidConfig {
                reason: "x encoding is required for Contour chart".to_string(),
            });
        }

        if let Some(y_field) = &spec.encoding.y {
            if data.get_column(&y_field.name).is_none() {
                return Err(ComponentError::InvalidConfig {
                    reason: format!("y field '{}' not found in data", y_field.name),
                });
            }
        } else {
            return Err(ComponentError::InvalidConfig {
                reason: "y encoding is required for Contour chart".to_string(),
            });
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

        layers.update_layer(LayerKind::Background, super::shared::render_background_with_border(spec, theme, &plot_area));

        if let Some(title) = &spec.title {
            layers.update_layer(LayerKind::Title, super::shared::render_title(theme, title, &plot_area));
        }

        ChartOutput {
            layers,
            hit_regions: Vec::new(),
        }
    }

    /// 提取数据点 (x, y, value)
    fn extract_points(
        spec: &ChartSpec,
        data: &DataTable,
    ) -> Result<Vec<(f64, f64, f64)>, ComponentError> {
        let x_field = spec.encoding.x.as_ref().ok_or_else(|| ComponentError::InvalidConfig {
            reason: "x encoding is required".to_string(),
        })?;
        let y_field = spec.encoding.y.as_ref().ok_or_else(|| ComponentError::InvalidConfig {
            reason: "y encoding is required".to_string(),
        })?;

        let x_column = data.get_column(&x_field.name).ok_or_else(|| ComponentError::InvalidConfig {
            reason: format!("x field '{}' not found in data", x_field.name),
        })?;
        let y_column = data.get_column(&y_field.name).ok_or_else(|| ComponentError::InvalidConfig {
            reason: format!("y field '{}' not found in data", y_field.name),
        })?;

        // color 字段作为值；若无 color 编码，y 字段既当位置又当值
        let value_column = spec.encoding.color.as_ref()
            .and_then(|field| data.get_column(&field.name))
            .unwrap_or(y_column);

        let mut points = Vec::new();
        let row_count = data.row_count();
        for i in 0..row_count {
            let x = x_column.get(i).and_then(|v| v.as_numeric());
            let y = y_column.get(i).and_then(|v| v.as_numeric());
            let v = value_column.get(i).and_then(|v| v.as_numeric());

            if let (Some(x), Some(y), Some(v)) = (x, y, v) {
                points.push((x, y, v));
            }
        }

        Ok(points)
    }

    /// 构建 2D 网格
    fn build_grid(
        points: &[(f64, f64, f64)],
        plot_area: &PlotArea,
    ) -> (Vec<Vec<f64>>, (f64, f64), (f64, f64), LinearScale, LinearScale) {
        // 计算数据范围
        let x_min = points.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
        let x_max = points.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
        let y_min = points.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
        let y_max = points.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);

        let x_range = (x_min, x_max);
        let y_range = (y_min, y_max);

        let x_scale = LinearScale::new(x_min, x_max, plot_area.x, plot_area.x + plot_area.width);
        let y_scale = LinearScale::new(y_min, y_max, plot_area.y + plot_area.height, plot_area.y);

        // 网格尺寸：根据数据点数量决定
        let n = points.len();
        let grid_res = ((n as f64).sqrt().ceil() as usize).clamp(4, 50);
        let ncols = grid_res;
        let nrows = grid_res;

        let mut grid = vec![vec![0.0; ncols]; nrows];
        let mut count = vec![vec![0usize; ncols]; nrows];

        let dx = if x_max > x_min { (x_max - x_min) / ncols as f64 } else { 1.0 };
        let dy = if y_max > y_min { (y_max - y_min) / nrows as f64 } else { 1.0 };

        // 将数据点分配到网格单元
        for &(x, y, v) in points {
            let col = (((x - x_min) / dx) as usize).min(ncols - 1);
            let row = (((y - y_min) / dy) as usize).min(nrows - 1);
            grid[row][col] += v;
            count[row][col] += 1;
        }

        // 平均化
        for r in 0..nrows {
            for c in 0..ncols {
                if count[r][c] > 0 {
                    grid[r][c] /= count[r][c] as f64;
                }
            }
        }

        // 简单插值填充空单元
        // 对空单元使用最近邻的平均值
        let mut filled = grid.clone();
        for r in 0..nrows {
            for c in 0..ncols {
                if count[r][c] == 0 {
                    let mut sum = 0.0;
                    let mut cnt = 0;
                    for dr in -1i32..=1 {
                        for dc in -1i32..=1 {
                            if dr == 0 && dc == 0 { continue; }
                            let nr = (r as i32 + dr) as usize;
                            let nc = (c as i32 + dc) as usize;
                            if nr < nrows && nc < ncols && count[nr][nc] > 0 {
                                sum += grid[nr][nc];
                                cnt += 1;
                            }
                        }
                    }
                    if cnt > 0 {
                        filled[r][c] = sum / cnt as f64;
                    }
                }
            }
        }

        (filled, x_range, y_range, x_scale, y_scale)
    }

    /// 计算等高线级别
    fn compute_levels(grid: &[Vec<f64>]) -> Vec<f64> {
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        for row in grid {
            for &v in row {
                min = min.min(v);
                max = max.max(v);
            }
        }

        if min >= max {
            return vec![min];
        }

        let n_levels = DEFAULT_NUM_LEVELS;
        let step = (max - min) / (n_levels + 1) as f64;
        (1..=n_levels)
            .map(|i| min + i as f64 * step)
            .collect()
    }

    /// 渲染等高线
    fn render_contours<T: Theme>(
        _theme: &T,
        contours: &[deneb_core::algorithm::contour::ContourPath],
        grid: &[Vec<f64>],
        x_range: (f64, f64),
        y_range: (f64, f64),
    ) -> RenderOutput {
        let mut output = RenderOutput::new();

        // 找到全局值范围用于颜色映射
        let mut global_min = f64::INFINITY;
        let mut global_max = f64::NEG_INFINITY;
        for row in grid {
            for &v in row {
                global_min = global_min.min(v);
                global_max = global_max.max(v);
            }
        }
        let value_range = (global_max - global_min).max(1e-10);

        for contour in contours {
            // 根据级别选择颜色
            let t = if value_range > 1e-10 {
                ((contour.level - global_min) / value_range).clamp(0.0, 1.0)
            } else {
                0.5
            };
            let color_idx = (t * (CONTOUR_COLORS.len() - 1) as f64).round() as usize;
            let color = CONTOUR_COLORS[color_idx.min(CONTOUR_COLORS.len() - 1)];

            for path in &contour.paths {
                if path.len() < 2 {
                    continue;
                }

                // 尝试关闭开放路径
                let closed_path = close_open_path_at_boundary(path, x_range, y_range);
                let is_closed = closed_path.last() == Some(&closed_path[0]);

                let mut segments = Vec::new();
                segments.push(PathSegment::MoveTo(closed_path[0].0, closed_path[0].1));
                for i in 1..closed_path.len() {
                    segments.push(PathSegment::LineTo(closed_path[i].0, closed_path[i].1));
                }
                if is_closed {
                    segments.push(PathSegment::Close);
                }

                let fill = if is_closed {
                    Some(FillStyle::Color(Self::with_alpha(color, 0.15)))
                } else {
                    None
                };

                output.add_command(DrawCmd::Path {
                    segments,
                    fill,
                    stroke: Some(StrokeStyle::Color(color.to_string())),
                });
            }
        }

        output
    }

    /// 当数据点 <3 时渲染为散点
    fn render_as_scatter<T: Theme>(
        spec: &ChartSpec,
        theme: &T,
        plot_area: &PlotArea,
        points: &[(f64, f64, f64)],
    ) -> Result<ChartOutput, ComponentError> {
        let mut layers = RenderLayers::new();
        let mut hit_regions = Vec::new();

        layers.update_layer(LayerKind::Background, super::shared::render_background_with_border(spec, theme, plot_area));

        let mut data_output = RenderOutput::new();

        if !points.is_empty() {
            let x_min = points.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
            let x_max = points.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
            let y_min = points.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
            let y_max = points.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);

            let x_scale = LinearScale::new(
                x_min - (x_max - x_min) * 0.05,
                x_max + (x_max - x_min) * 0.05,
                plot_area.x,
                plot_area.x + plot_area.width,
            );
            let y_scale = LinearScale::new(
                y_min - (y_max - y_min) * 0.05,
                y_max + (y_max - y_min) * 0.05,
                plot_area.y + plot_area.height,
                plot_area.y,
            );

            let point_radius = theme.layout_config().point_radius;
            for (idx, (px, py, _pv)) in points.iter().enumerate() {
                let sx = x_scale.map(*px);
                let sy = y_scale.map(*py);

                data_output.add_command(DrawCmd::Circle {
                    cx: sx,
                    cy: sy,
                    r: point_radius,
                    fill: Some(FillStyle::Color(theme.series_color(0).to_string())),
                    stroke: None,
                });

                let region = HitRegion::from_rect(
                    sx - point_radius,
                    sy - point_radius,
                    point_radius * 2.0,
                    point_radius * 2.0,
                    idx,
                    None,
                    vec![FieldValue::Numeric(*py)],
                );
                hit_regions.push(region);
            }
        }

        layers.update_layer(LayerKind::Data, data_output);

        if let Some(title) = &spec.title {
            layers.update_layer(LayerKind::Title, super::shared::render_title(theme, title, plot_area));
        }

        Ok(ChartOutput {
            layers,
            hit_regions,
        })
    }

    /// 为颜色添加透明度
    fn with_alpha(color: &str, alpha: f64) -> String {
        let hex = color.trim_start_matches('#');
        if hex.len() == 6 {
            format!("#{}{:02x}", hex, (alpha * 255.0) as u8)
        } else {
            color.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{Encoding, Mark, Field};
    use crate::theme::DefaultTheme;

    fn create_contour_data() -> DataTable {
        // 创建一个简单的梯度数据
        let mut x_vals = Vec::new();
        let mut y_vals = Vec::new();
        let mut v_vals = Vec::new();

        for i in 0..5 {
            for j in 0..5 {
                let x = i as f64;
                let y = j as f64;
                let v = x + y; // 简单梯度
                x_vals.push(FieldValue::Numeric(x));
                y_vals.push(FieldValue::Numeric(y));
                v_vals.push(FieldValue::Numeric(v));
            }
        }

        DataTable::with_columns(vec![
            Column::new("x", DataType::Quantitative, x_vals),
            Column::new("y", DataType::Quantitative, y_vals),
            Column::new("value", DataType::Quantitative, v_vals),
        ])
    }

    fn create_contour_spec() -> ChartSpec {
        ChartSpec::builder()
            .mark(Mark::Contour)
            .encoding(
                Encoding::new()
                    .x(Field::quantitative("x"))
                    .y(Field::quantitative("y"))
                    .color(Field::quantitative("value")),
            )
            .width(400.0)
            .height(300.0)
            .build()
            .unwrap()
    }

    #[test]
    fn test_contour_render_basic() {
        let spec = create_contour_spec();
        let theme = DefaultTheme;
        let data = create_contour_data();

        let result = ContourChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.layers.get_layer(LayerKind::Background).is_some());
        assert!(output.layers.get_layer(LayerKind::Data).is_some());
        // 25 个数据点 → 25 个 hit regions
        assert_eq!(output.hit_regions.len(), 25);
    }

    #[test]
    fn test_contour_render_empty() {
        let spec = create_contour_spec();
        let theme = DefaultTheme;
        let data = DataTable::with_columns(vec![
            Column::new("x", DataType::Quantitative, vec![]),
            Column::new("y", DataType::Quantitative, vec![]),
            Column::new("value", DataType::Quantitative, vec![]),
        ]);

        let result = ContourChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.hit_regions.is_empty());
    }

    #[test]
    fn test_contour_render_few_points_as_scatter() {
        // < 3 个点 → 散点图
        let spec = create_contour_spec();
        let theme = DefaultTheme;
        let data = DataTable::with_columns(vec![
            Column::new("x", DataType::Quantitative, vec![
                FieldValue::Numeric(1.0),
                FieldValue::Numeric(2.0),
            ]),
            Column::new("y", DataType::Quantitative, vec![
                FieldValue::Numeric(3.0),
                FieldValue::Numeric(4.0),
            ]),
            Column::new("value", DataType::Quantitative, vec![
                FieldValue::Numeric(10.0),
                FieldValue::Numeric(20.0),
            ]),
        ]);

        let result = ContourChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.hit_regions.len(), 2);
    }

    #[test]
    fn test_contour_with_title() {
        let spec = ChartSpec::builder()
            .mark(Mark::Contour)
            .encoding(
                Encoding::new()
                    .x(Field::quantitative("x"))
                    .y(Field::quantitative("y"))
                    .color(Field::quantitative("value")),
            )
            .title("Temperature Map")
            .width(400.0)
            .height(300.0)
            .build()
            .unwrap();

        let theme = DefaultTheme;
        let data = create_contour_data();

        let result = ContourChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        let title_layer = output.layers.get_layer(LayerKind::Title);
        assert!(title_layer.is_some());
    }

    #[test]
    fn test_contour_validate_missing_field() {
        let spec = create_contour_spec();
        let theme = DefaultTheme;
        let data = DataTable::with_columns(vec![
            Column::new("wrong", DataType::Quantitative, vec![]),
        ]);

        let result = ContourChart::render(&spec, &theme, &data);
        assert!(result.is_err());
    }

    #[test]
    fn test_contour_without_color_encoding() {
        // 无 color 编码 → y 字段既当位置又当值
        let spec = ChartSpec::builder()
            .mark(Mark::Contour)
            .encoding(
                Encoding::new()
                    .x(Field::quantitative("x"))
                    .y(Field::quantitative("y")),
            )
            .width(400.0)
            .height(300.0)
            .build()
            .unwrap();

        let theme = DefaultTheme;
        let data = create_contour_data();

        let result = ContourChart::render(&spec, &theme, &data);
        assert!(result.is_ok());
    }
}
