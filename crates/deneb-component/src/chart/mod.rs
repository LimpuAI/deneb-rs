//! Chart 渲染模块
//!
//! 提供各种图表类型的渲染实现，包括折线图、柱状图等。

/// Shared rendering helpers
pub mod shared;

pub mod line;
pub mod bar;

/// ScatterChart 实现
pub mod scatter;

/// AreaChart 实现
pub mod area;

/// HistogramChart 实现
pub mod histogram;

/// WaterfallChart 实现
pub mod waterfall;

/// CandlestickChart 实现
pub mod candlestick;

/// BoxPlotChart 实现
pub mod box_plot;

/// StripChart 实现
pub mod strip;

/// HeatmapChart 实现
pub mod heatmap;

/// PieChart 实现
pub mod pie;

/// RadarChart 实现
pub mod radar;

/// SankeyChart 实现
pub mod sankey;

/// ChordChart 实现
pub mod chord;

/// ContourChart 实现
pub mod contour_chart;

use deneb_core::{RenderLayers, HitRegion};

/// Chart 渲染结果
///
/// 包含分层渲染输出和命中测试区域，支持增量渲染和交互。
#[derive(Clone, Debug)]
pub struct ChartOutput {
    /// 分层渲染输出
    pub layers: RenderLayers,
    /// 命中区域列表（按数据点索引顺序）
    pub hit_regions: Vec<HitRegion>,
}

impl ChartOutput {
    /// 创建新的 ChartOutput
    pub fn new() -> Self {
        Self {
            layers: RenderLayers::new(),
            hit_regions: Vec::new(),
        }
    }

    /// 创建带渲染层的 ChartOutput
    pub fn with_layers(layers: RenderLayers) -> Self {
        Self {
            layers,
            hit_regions: Vec::new(),
        }
    }

    /// 添加命中区域
    pub fn add_hit_region(&mut self, region: HitRegion) {
        self.hit_regions.push(region);
    }

    /// 扩展命中区域列表
    pub fn extend_hit_regions(&mut self, regions: impl IntoIterator<Item = HitRegion>) {
        self.hit_regions.extend(regions);
    }

    /// 判断是否有脏层
    pub fn has_dirty_layers(&self) -> bool {
        self.layers.has_dirty_layers()
    }

    /// 获取脏层数量
    pub fn dirty_count(&self) -> usize {
        self.layers.dirty_count()
    }
}

impl Default for ChartOutput {
    fn default() -> Self {
        Self::new()
    }
}

impl From<RenderLayers> for ChartOutput {
    fn from(layers: RenderLayers) -> Self {
        Self::with_layers(layers)
    }
}

pub use line::LineChart;
pub use bar::BarChart;
pub use scatter::ScatterChart;
pub use area::AreaChart;
pub use histogram::HistogramChart;
pub use waterfall::WaterfallChart;
pub use candlestick::CandlestickChart;
pub use box_plot::BoxPlotChart;
pub use strip::StripChart;
pub use sankey::SankeyChart;
pub use chord::ChordChart;
pub use contour_chart::ContourChart;
pub use heatmap::HeatmapChart;
pub use pie::PieChart;
pub use radar::RadarChart;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chart_output_new() {
        let output = ChartOutput::new();
        assert!(output.has_dirty_layers()); // 新创建的 RenderLayers 所有层都是脏的
        assert_eq!(output.dirty_count(), 7); // 7 个标准层都是脏的
        assert!(output.hit_regions.is_empty());
    }

    #[test]
    fn test_chart_output_add_hit_region() {
        use deneb_core::{BoundingBox, FieldValue};
        let mut output = ChartOutput::new();
        let region = HitRegion::new(
            0,
            None,
            BoundingBox::new(10.0, 20.0, 5.0, 5.0),
            vec![FieldValue::Numeric(42.0)],
        );
        output.add_hit_region(region);
        assert_eq!(output.hit_regions.len(), 1);
    }

    #[test]
    fn test_chart_output_from_render_layers() {
        let layers = RenderLayers::new();
        let output = ChartOutput::from(layers);
        assert_eq!(output.layers.all().len(), 7);
    }
}
