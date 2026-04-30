//! 渲染层系统
//!
//! 提供分层渲染能力，支持脏标记和增量更新。

use crate::instruction::RenderOutput;

/// 层类型
///
/// 定义可视化中不同功能的层，按照 z-index 排序。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LayerKind {
    /// 背景层 (z-index: 0)
    Background,
    /// 网格层 (z-index: 1)
    Grid,
    /// 坐标轴层 (z-index: 2)
    Axis,
    /// 数据层 (z-index: 3)
    Data,
    /// 图例层 (z-index: 4)
    Legend,
    /// 标题层 (z-index: 5)
    Title,
    /// 标注层 (z-index: 6)
    Annotation,
}

impl LayerKind {
    /// 获取默认的 z-index
    pub fn default_z_index(&self) -> u32 {
        match self {
            LayerKind::Background => 0,
            LayerKind::Grid => 1,
            LayerKind::Axis => 2,
            LayerKind::Data => 3,
            LayerKind::Legend => 4,
            LayerKind::Title => 5,
            LayerKind::Annotation => 6,
        }
    }

    /// 获取所有标准层的类型
    pub fn all_standard_kinds() -> Vec<Self> {
        vec![
            Self::Background,
            Self::Grid,
            Self::Axis,
            Self::Data,
            Self::Legend,
            Self::Title,
            Self::Annotation,
        ]
    }
}

/// 渲染层
///
/// 表示一个渲染层，包含类型、脏标记、渲染指令和 z-index。
#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    /// 层类型
    pub kind: LayerKind,
    /// 是否脏（需要重新渲染）
    pub dirty: bool,
    /// 渲染指令
    pub commands: RenderOutput,
    /// z-index (用于排序)
    pub z_index: u32,
}

impl Layer {
    /// 创建新层
    pub fn new(kind: LayerKind) -> Self {
        Self {
            kind,
            dirty: true,
            commands: RenderOutput::new(),
            z_index: kind.default_z_index(),
        }
    }

    /// 创建带渲染指令的层
    pub fn with_commands(kind: LayerKind, commands: RenderOutput) -> Self {
        Self {
            kind,
            dirty: true,
            commands,
            z_index: kind.default_z_index(),
        }
    }

    /// 创建带自定义 z-index 的层
    pub fn with_z_index(kind: LayerKind, z_index: u32) -> Self {
        Self {
            kind,
            dirty: true,
            commands: RenderOutput::new(),
            z_index,
        }
    }

    /// 标记为脏
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// 标记为干净
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    /// 更新渲染指令
    pub fn update_commands(&mut self, commands: RenderOutput) {
        self.commands = commands;
        self.dirty = true;
    }

    /// 清空渲染指令
    pub fn clear(&mut self) {
        self.commands.clear();
        self.dirty = true;
    }
}

impl From<LayerKind> for Layer {
    fn from(kind: LayerKind) -> Self {
        Self::new(kind)
    }
}

/// 渲染层集合
///
/// 管理所有渲染层，提供按类型查找、更新等功能。
#[derive(Debug, Clone, PartialEq)]
pub struct RenderLayers {
    /// 层列表
    layers: Vec<Layer>,
}

impl RenderLayers {
    /// 创建新的渲染层集合（包含所有标准层）
    pub fn new() -> Self {
        let layers = LayerKind::all_standard_kinds()
            .into_iter()
            .map(Layer::from)
            .collect();

        Self { layers }
    }

    /// 创建空的渲染层集合
    pub fn empty() -> Self {
        Self {
            layers: Vec::new(),
        }
    }

    /// 获取所有层
    pub fn all(&self) -> &[Layer] {
        &self.layers
    }

    /// 获取可变的所有层
    pub fn all_mut(&mut self) -> &mut [Layer] {
        &mut self.layers
    }

    /// 获取脏层迭代器
    pub fn dirty_layers(&self) -> impl Iterator<Item = &Layer> {
        self.layers.iter().filter(|layer| layer.dirty)
    }

    /// 获取可变脏层迭代器
    pub fn dirty_layers_mut(&mut self) -> impl Iterator<Item = &mut Layer> {
        self.layers.iter_mut().filter(|layer| layer.dirty)
    }

    /// 标记指定层为脏
    pub fn mark_dirty(&mut self, kind: LayerKind) {
        if let Some(layer) = self.get_layer_mut(kind) {
            layer.mark_dirty();
        }
    }

    /// 标记所有层为脏
    pub fn mark_all_dirty(&mut self) {
        for layer in &mut self.layers {
            layer.mark_dirty();
        }
    }

    /// 标记指定层为干净
    pub fn mark_clean(&mut self, kind: LayerKind) {
        if let Some(layer) = self.get_layer_mut(kind) {
            layer.mark_clean();
        }
    }

    /// 标记所有层为干净
    pub fn mark_all_clean(&mut self) {
        for layer in &mut self.layers {
            layer.mark_clean();
        }
    }

    /// 获取指定类型的层
    pub fn get_layer(&self, kind: LayerKind) -> Option<&Layer> {
        self.layers.iter().find(|layer| layer.kind == kind)
    }

    /// 获取可变的指定类型层
    pub fn get_layer_mut(&mut self, kind: LayerKind) -> Option<&mut Layer> {
        self.layers.iter_mut().find(|layer| layer.kind == kind)
    }

    /// 更新指定层的渲染指令
    pub fn update_layer(&mut self, kind: LayerKind, commands: RenderOutput) {
        if let Some(layer) = self.get_layer_mut(kind) {
            layer.update_commands(commands);
        }
    }

    /// 添加自定义层
    pub fn add_layer(&mut self, layer: Layer) {
        self.layers.push(layer);
        // 保持按 z-index 排序
        self.layers.sort_by_key(|l| l.z_index);
    }

    /// 移除指定类型的层
    pub fn remove_layer(&mut self, kind: LayerKind) -> Option<Layer> {
        let pos = self.layers.iter().position(|l| l.kind == kind)?;
        Some(self.layers.remove(pos))
    }

    /// 判断是否有脏层
    pub fn has_dirty_layers(&self) -> bool {
        self.layers.iter().any(|layer| layer.dirty)
    }

    /// 获取脏层数量
    pub fn dirty_count(&self) -> usize {
        self.layers.iter().filter(|layer| layer.dirty).count()
    }

    /// 清空所有层的渲染指令
    pub fn clear_all(&mut self) {
        for layer in &mut self.layers {
            layer.clear();
        }
    }
}

impl Default for RenderLayers {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruction::DrawCmd;
    use crate::style::FillStyle;

    #[test]
    fn test_layer_kind_z_index() {
        assert_eq!(LayerKind::Background.default_z_index(), 0);
        assert_eq!(LayerKind::Grid.default_z_index(), 1);
        assert_eq!(LayerKind::Axis.default_z_index(), 2);
        assert_eq!(LayerKind::Data.default_z_index(), 3);
        assert_eq!(LayerKind::Legend.default_z_index(), 4);
        assert_eq!(LayerKind::Title.default_z_index(), 5);
        assert_eq!(LayerKind::Annotation.default_z_index(), 6);
    }

    #[test]
    fn test_layer_creation() {
        let layer = Layer::new(LayerKind::Data);
        assert_eq!(layer.kind, LayerKind::Data);
        assert!(layer.dirty);
        assert_eq!(layer.z_index, 3);
        assert!(layer.commands.is_empty());
    }

    #[test]
    fn test_layer_mark_dirty() {
        let mut layer = Layer::new(LayerKind::Data);
        layer.mark_clean();
        assert!(!layer.dirty);

        layer.mark_dirty();
        assert!(layer.dirty);
    }

    #[test]
    fn test_layer_update_commands() {
        let mut layer = Layer::new(LayerKind::Data);
        let commands = RenderOutput::from_commands(vec![DrawCmd::Rect {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
            fill: Some(FillStyle::Color("#fff".to_string())),
            stroke: None,
            corner_radius: None,
        }]);

        layer.update_commands(commands);
        assert!(layer.dirty);
        assert!(!layer.commands.is_empty());
    }

    #[test]
    fn test_render_layers_new() {
        let layers = RenderLayers::new();
        assert_eq!(layers.all().len(), 7); // 7 个标准层
    }

    #[test]
    fn test_render_layers_empty() {
        let layers = RenderLayers::empty();
        assert_eq!(layers.all().len(), 0);
    }

    #[test]
    fn test_render_layers_get_layer() {
        let layers = RenderLayers::new();

        let data_layer = layers.get_layer(LayerKind::Data);
        assert!(data_layer.is_some());
        assert_eq!(data_layer.unwrap().kind, LayerKind::Data);

        let nonexistent = layers.get_layer(LayerKind::Data);
        assert!(nonexistent.is_some());
    }

    #[test]
    fn test_render_layers_mark_dirty() {
        let mut layers = RenderLayers::new();
        layers.mark_all_clean();

        layers.mark_dirty(LayerKind::Data);
        assert_eq!(layers.dirty_count(), 1);

        layers.mark_all_dirty();
        assert_eq!(layers.dirty_count(), 7);
    }

    #[test]
    fn test_render_layers_update_layer() {
        let mut layers = RenderLayers::new();
        let commands = RenderOutput::from_commands(vec![DrawCmd::Circle {
            cx: 0.0,
            cy: 0.0,
            r: 5.0,
            fill: None,
            stroke: None,
        }]);

        layers.update_layer(LayerKind::Data, commands);

        let data_layer = layers.get_layer(LayerKind::Data).unwrap();
        assert!(!data_layer.commands.is_empty());
        assert!(data_layer.dirty);
    }

    #[test]
    fn test_render_layers_dirty_layers() {
        let mut layers = RenderLayers::new();
        layers.mark_all_clean();

        layers.mark_dirty(LayerKind::Data);
        layers.mark_dirty(LayerKind::Axis);

        let dirty_layers: Vec<_> = layers.dirty_layers().collect();
        assert_eq!(dirty_layers.len(), 2);
    }

    #[test]
    fn test_render_layers_add_custom_layer() {
        let mut layers = RenderLayers::new();
        let custom_layer = Layer::with_z_index(LayerKind::Data, 10);

        layers.add_layer(custom_layer);

        // 应该有 8 个层（7 个标准层 + 1 个自定义层）
        assert_eq!(layers.all().len(), 8);

        // 自定义层应该在最后（z-index 最大）
        assert_eq!(layers.all().last().unwrap().z_index, 10);
    }

    #[test]
    fn test_render_layers_remove_layer() {
        let mut layers = RenderLayers::new();

        let removed = layers.remove_layer(LayerKind::Data);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().kind, LayerKind::Data);

        assert_eq!(layers.all().len(), 6);

        let removed_again = layers.remove_layer(LayerKind::Data);
        assert!(removed_again.is_none());
    }

    #[test]
    fn test_render_layers_clear_all() {
        let mut layers = RenderLayers::new();
        let commands = RenderOutput::from_commands(vec![DrawCmd::Rect {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
            fill: None,
            stroke: None,
            corner_radius: None,
        }]);

        layers.update_layer(LayerKind::Data, commands);
        layers.mark_all_clean();

        layers.clear_all();

        // 所有层应该都是脏的（因为被清空了）
        assert!(layers.has_dirty_layers());

        // 所有层的命令都应该是空的
        for layer in layers.all() {
            assert!(layer.commands.is_empty());
        }
    }

    #[test]
    fn test_layer_from_kind() {
        let layer: Layer = LayerKind::Data.into();
        assert_eq!(layer.kind, LayerKind::Data);
        assert!(layer.dirty);
    }

    #[test]
    fn test_render_layers_has_dirty_layers() {
        let mut layers = RenderLayers::new();
        layers.mark_all_clean();

        assert!(!layers.has_dirty_layers());

        layers.mark_dirty(LayerKind::Data);
        assert!(layers.has_dirty_layers());
    }

    #[test]
    fn test_render_layers_mark_clean() {
        let mut layers = RenderLayers::new();

        layers.mark_clean(LayerKind::Data);
        assert!(!layers.get_layer(LayerKind::Data).unwrap().dirty);

        layers.mark_all_clean();
        assert!(!layers.has_dirty_layers());
    }
}
