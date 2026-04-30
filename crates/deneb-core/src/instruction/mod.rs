//! Canvas 2D 指令类型定义
//!
//! 提供语义化的绘图指令和底层的 Canvas 操作指令。

use crate::style::{FillStyle, StrokeStyle, TextStyle, TextAnchor, TextBaseline};

/// 绘图指令
///
/// 语义化的绘图指令，表示要绘制的内容。
#[derive(Clone, Debug, PartialEq)]
pub enum DrawCmd {
    /// 矩形
    Rect {
        /// x 坐标
        x: f64,
        /// y 坐标
        y: f64,
        /// 宽度
        width: f64,
        /// 高度
        height: f64,
        /// 填充样式
        fill: Option<FillStyle>,
        /// 描边样式
        stroke: Option<StrokeStyle>,
        /// 圆角半径
        corner_radius: Option<f64>,
    },
    /// 路径
    Path {
        /// 路径段
        segments: Vec<PathSegment>,
        /// 填充样式
        fill: Option<FillStyle>,
        /// 描边样式
        stroke: Option<StrokeStyle>,
    },
    /// 圆形
    Circle {
        /// 圆心 x 坐标
        cx: f64,
        /// 圆心 y 坐标
        cy: f64,
        /// 半径
        r: f64,
        /// 填充样式
        fill: Option<FillStyle>,
        /// 描边样式
        stroke: Option<StrokeStyle>,
    },
    /// 文本
    Text {
        /// x 坐标
        x: f64,
        /// y 坐标
        y: f64,
        /// 文本内容
        content: String,
        /// 文本样式
        style: TextStyle,
        /// 文本锚点
        anchor: TextAnchor,
        /// 文本基线
        baseline: TextBaseline,
    },
    /// 分组
    Group {
        /// 分组标签
        label: Option<String>,
        /// 子指令
        items: Vec<DrawCmd>,
    },
}

impl DrawCmd {
    /// 转换为 Canvas 操作序列
    pub fn to_canvas_ops(&self) -> Vec<CanvasOp> {
        match self {
            DrawCmd::Rect {
                x,
                y,
                width,
                height,
                fill,
                stroke,
                corner_radius,
            } => {
                let mut ops = Vec::new();

                if let Some(radius) = corner_radius {
                    // 圆角矩形需要用路径绘制
                    ops.extend(Self::rounded_rect_path(*x, *y, *width, *height, *radius, fill, stroke));
                } else {
                    // 普通矩形可以直接使用 fillRect/strokeRect
                    if let Some(fill_style) = fill {
                        if let FillStyle::Color(color) = fill_style {
                            ops.push(CanvasOp::SetFillStyle(color.clone()));
                            ops.push(CanvasOp::FillRect(*x, *y, *width, *height));
                        }
                    }
                    if let Some(stroke_style) = stroke {
                        if let StrokeStyle::Color(color) = stroke_style {
                            ops.push(CanvasOp::SetStrokeStyle(color.clone()));
                            ops.push(CanvasOp::StrokeRect(*x, *y, *width, *height));
                        }
                    }
                }

                ops
            }
            DrawCmd::Path {
                segments,
                fill,
                stroke,
            } => {
                let mut ops = Vec::new();

                if segments.is_empty() {
                    return ops;
                }

                ops.push(CanvasOp::BeginPath);

                for segment in segments {
                    ops.extend(segment.to_canvas_ops());
                }

                if let Some(fill_style) = fill {
                    if let FillStyle::Color(color) = fill_style {
                        ops.push(CanvasOp::SetFillStyle(color.clone()));
                        ops.push(CanvasOp::Fill);
                    }
                }
                if let Some(stroke_style) = stroke {
                    if let StrokeStyle::Color(color) = stroke_style {
                        ops.push(CanvasOp::SetStrokeStyle(color.clone()));
                        ops.push(CanvasOp::Stroke);
                    }
                }

                ops
            }
            DrawCmd::Circle {
                cx,
                cy,
                r,
                fill,
                stroke,
            } => {
                let mut ops = Vec::new();

                ops.push(CanvasOp::BeginPath);
                ops.push(CanvasOp::Arc(*cx, *cy, *r, 0.0, 2.0 * std::f64::consts::PI, false));

                if let Some(fill_style) = fill {
                    if let FillStyle::Color(color) = fill_style {
                        ops.push(CanvasOp::SetFillStyle(color.clone()));
                        ops.push(CanvasOp::Fill);
                    }
                }
                if let Some(stroke_style) = stroke {
                    if let StrokeStyle::Color(color) = stroke_style {
                        ops.push(CanvasOp::SetStrokeStyle(color.clone()));
                        ops.push(CanvasOp::Stroke);
                    }
                }

                ops
            }
            DrawCmd::Text {
                x,
                y,
                content,
                style,
                anchor,
                baseline,
            } => {
                let mut ops = Vec::new();

                ops.push(CanvasOp::SetFont(style.to_css_font()));
                ops.push(CanvasOp::SetTextAlign(anchor.to_string()));
                ops.push(CanvasOp::SetTextBaseline(baseline.to_string()));

                if let FillStyle::Color(color) = &style.fill {
                    ops.push(CanvasOp::SetFillStyle(color.clone()));
                    ops.push(CanvasOp::FillText(content.clone(), *x, *y));
                }

                ops
            }
            DrawCmd::Group { label: _, items } => {
                let mut ops = Vec::new();
                ops.push(CanvasOp::Save);
                for item in items {
                    ops.extend(item.to_canvas_ops());
                }
                ops.push(CanvasOp::Restore);
                ops
            }
        }
    }

    // 辅助方法：生成圆角矩形路径
    fn rounded_rect_path(
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        radius: f64,
        fill: &Option<FillStyle>,
        stroke: &Option<StrokeStyle>,
    ) -> Vec<CanvasOp> {
        let mut ops = Vec::new();
        let r = radius.min(width / 2.0).min(height / 2.0);

        ops.push(CanvasOp::BeginPath);
        ops.push(CanvasOp::MoveTo(x + r, y));
        ops.push(CanvasOp::LineTo(x + width - r, y));
        ops.push(CanvasOp::QuadraticCurveTo(x + width, y, x + width, y + r));
        ops.push(CanvasOp::LineTo(x + width, y + height - r));
        ops.push(CanvasOp::QuadraticCurveTo(
            x + width,
            y + height,
            x + width - r,
            y + height,
        ));
        ops.push(CanvasOp::LineTo(x + r, y + height));
        ops.push(CanvasOp::QuadraticCurveTo(x, y + height, x, y + height - r));
        ops.push(CanvasOp::LineTo(x, y + r));
        ops.push(CanvasOp::QuadraticCurveTo(x, y, x + r, y));
        ops.push(CanvasOp::ClosePath);

        if let Some(fill_style) = fill {
            if let FillStyle::Color(color) = fill_style {
                ops.push(CanvasOp::SetFillStyle(color.clone()));
                ops.push(CanvasOp::Fill);
            }
        }
        if let Some(stroke_style) = stroke {
            if let StrokeStyle::Color(color) = stroke_style {
                ops.push(CanvasOp::SetStrokeStyle(color.clone()));
                ops.push(CanvasOp::Stroke);
            }
        }

        ops
    }
}

/// 路径段
///
/// 表示路径中的一个操作。
#[derive(Clone, Debug, PartialEq)]
pub enum PathSegment {
    /// 移动到 (x, y)
    MoveTo(f64, f64),
    /// 直线到 (x, y)
    LineTo(f64, f64),
    /// 三次贝塞尔曲线到 (x, y)，控制点 (cp1x, cp1y) 和 (cp2x, cp2y)
    BezierTo(f64, f64, f64, f64, f64, f64),
    /// 二次贝塞尔曲线到 (x, y)，控制点 (cpx, cpy)
    QuadraticTo(f64, f64, f64, f64),
    /// 圆弧
    Arc(f64, f64, f64, f64, f64, bool),
    /// 闭合路径
    Close,
}

impl PathSegment {
    /// 转换为 Canvas 操作
    pub fn to_canvas_ops(&self) -> Vec<CanvasOp> {
        match self {
            PathSegment::MoveTo(x, y) => vec![CanvasOp::MoveTo(*x, *y)],
            PathSegment::LineTo(x, y) => vec![CanvasOp::LineTo(*x, *y)],
            PathSegment::BezierTo(cp1x, cp1y, cp2x, cp2y, x, y) => vec![CanvasOp::BezierCurveTo(
                *cp1x, *cp1y, *cp2x, *cp2y, *x, *y,
            )],
            PathSegment::QuadraticTo(cpx, cpy, x, y) => {
                vec![CanvasOp::QuadraticCurveTo(*cpx, *cpy, *x, *y)]
            }
            PathSegment::Arc(x, y, r, start_angle, end_angle, anticlockwise) => vec![
                CanvasOp::Arc(*x, *y, *r, *start_angle, *end_angle, *anticlockwise),
            ],
            PathSegment::Close => vec![CanvasOp::ClosePath],
        }
    }
}

/// Canvas 操作
///
/// 底层的 Canvas 2D API 调用。
#[derive(Clone, Debug, PartialEq)]
pub enum CanvasOp {
    /// 保存当前状态
    Save,
    /// 恢复上一次保存的状态
    Restore,
    /// 设置填充样式
    SetFillStyle(String),
    /// 设置描边样式
    SetStrokeStyle(String),
    /// 设置线宽
    SetLineWidth(f64),
    /// 设置字体
    SetFont(String),
    /// 设置文本对齐
    SetTextAlign(String),
    /// 设置文本基线
    SetTextBaseline(String),
    /// 开始路径
    BeginPath,
    /// 闭合路径
    ClosePath,
    /// 移动到
    MoveTo(f64, f64),
    /// 直线到
    LineTo(f64, f64),
    /// 三次贝塞尔曲线
    BezierCurveTo(f64, f64, f64, f64, f64, f64),
    /// 二次贝塞尔曲线
    QuadraticCurveTo(f64, f64, f64, f64),
    /// 圆弧
    Arc(f64, f64, f64, f64, f64, bool),
    /// 填充
    Fill,
    /// 描边
    Stroke,
    /// 填充矩形
    FillRect(f64, f64, f64, f64),
    /// 描边矩形
    StrokeRect(f64, f64, f64, f64),
    /// 清除矩形
    ClearRect(f64, f64, f64, f64),
    /// 填充文本
    FillText(String, f64, f64),
    /// 描边文本
    StrokeText(String, f64, f64),
}

/// 渲染输出
///
/// 包含语义指令和 Canvas 操作序列。
#[derive(Clone, Debug, PartialEq)]
pub struct RenderOutput {
    /// 语义指令
    pub semantic: Vec<DrawCmd>,
    /// Canvas 操作序列
    pub canvas_ops: Vec<CanvasOp>,
}

impl RenderOutput {
    /// 创建新的渲染输出
    pub fn new() -> Self {
        Self {
            semantic: Vec::new(),
            canvas_ops: Vec::new(),
        }
    }

    /// 添加语义指令
    pub fn add_command(&mut self, cmd: DrawCmd) {
        self.semantic.push(cmd.clone());
        self.canvas_ops.extend(cmd.to_canvas_ops());
    }

    /// 扩展多个语义指令
    pub fn extend_commands(&mut self, cmds: impl IntoIterator<Item = DrawCmd>) {
        for cmd in cmds {
            self.add_command(cmd);
        }
    }

    /// 清空输出
    pub fn clear(&mut self) {
        self.semantic.clear();
        self.canvas_ops.clear();
    }

    /// 判断是否为空
    pub fn is_empty(&self) -> bool {
        self.semantic.is_empty()
    }

    /// 获取指令数量
    pub fn len(&self) -> usize {
        self.semantic.len()
    }

    /// 从语义指令构建渲染输出
    pub fn from_commands(commands: Vec<DrawCmd>) -> Self {
        let mut output = Self::new();
        output.extend_commands(commands);
        output
    }
}

impl Default for RenderOutput {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Vec<DrawCmd>> for RenderOutput {
    fn from(commands: Vec<DrawCmd>) -> Self {
        Self::from_commands(commands)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{FillStyle, StrokeStyle, TextStyle};

    #[test]
    fn test_rect_to_canvas_ops() {
        let rect = DrawCmd::Rect {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
            fill: Some(FillStyle::Color("#ff0000".to_string())),
            stroke: Some(StrokeStyle::Color("#000000".to_string())),
            corner_radius: None,
        };

        let ops = rect.to_canvas_ops();
        assert_eq!(ops.len(), 4);
        assert!(matches!(ops[0], CanvasOp::SetFillStyle(_)));
        assert!(matches!(ops[1], CanvasOp::FillRect(10.0, 20.0, 100.0, 50.0)));
        assert!(matches!(ops[2], CanvasOp::SetStrokeStyle(_)));
        assert!(matches!(ops[3], CanvasOp::StrokeRect(10.0, 20.0, 100.0, 50.0)));
    }

    #[test]
    fn test_circle_to_canvas_ops() {
        let circle = DrawCmd::Circle {
            cx: 50.0,
            cy: 50.0,
            r: 25.0,
            fill: Some(FillStyle::Color("#0000ff".to_string())),
            stroke: None,
        };

        let ops = circle.to_canvas_ops();
        assert!(ops.len() >= 3);
        assert!(matches!(ops[0], CanvasOp::BeginPath));
        // 检查是否包含 Arc 操作
        assert!(ops.iter().any(|op| matches!(op, CanvasOp::Arc(_, _, _, _, _, _))));
        // 检查是否包含 Fill 操作
        assert!(ops.iter().any(|op| matches!(op, CanvasOp::Fill)));
    }

    #[test]
    fn test_text_to_canvas_ops() {
        let text = DrawCmd::Text {
            x: 10.0,
            y: 20.0,
            content: "Hello".to_string(),
            style: TextStyle::new(),
            anchor: TextAnchor::Start,
            baseline: TextBaseline::Alphabetic,
        };

        let ops = text.to_canvas_ops();
        assert!(ops.len() >= 4);
        assert!(matches!(ops[0], CanvasOp::SetFont(_)));
        assert!(matches!(ops[1], CanvasOp::SetTextAlign(_)));
        assert!(matches!(ops[2], CanvasOp::SetTextBaseline(_)));
        assert!(matches!(ops[3], CanvasOp::SetFillStyle(_)));
    }

    #[test]
    fn test_path_segment_to_canvas_ops() {
        let segment = PathSegment::MoveTo(10.0, 20.0);
        let ops = segment.to_canvas_ops();
        assert_eq!(ops, vec![CanvasOp::MoveTo(10.0, 20.0)]);

        let segment = PathSegment::LineTo(30.0, 40.0);
        let ops = segment.to_canvas_ops();
        assert_eq!(ops, vec![CanvasOp::LineTo(30.0, 40.0)]);
    }

    #[test]
    fn test_render_output() {
        let mut output = RenderOutput::new();
        assert!(output.is_empty());
        assert_eq!(output.len(), 0);

        output.add_command(DrawCmd::Rect {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
            fill: Some(FillStyle::Color("#fff".to_string())),
            stroke: None,
            corner_radius: None,
        });

        assert!(!output.is_empty());
        assert_eq!(output.len(), 1);
        assert!(!output.canvas_ops.is_empty());

        output.clear();
        assert!(output.is_empty());
    }

    #[test]
    fn test_render_output_from_commands() {
        let commands = vec![
            DrawCmd::Circle {
                cx: 0.0,
                cy: 0.0,
                r: 5.0,
                fill: None,
                stroke: None,
            },
            DrawCmd::Circle {
                cx: 10.0,
                cy: 10.0,
                r: 5.0,
                fill: None,
                stroke: None,
            },
        ];

        let output = RenderOutput::from_commands(commands);
        assert_eq!(output.len(), 2);
        assert_eq!(output.semantic.len(), 2);
        assert!(!output.canvas_ops.is_empty());
    }

    #[test]
    fn test_group() {
        let group = DrawCmd::Group {
            label: Some("test".to_string()),
            items: vec![
                DrawCmd::Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 10.0,
                    height: 10.0,
                    fill: None,
                    stroke: None,
                    corner_radius: None,
                },
            ],
        };

        let ops = group.to_canvas_ops();
        assert!(ops.len() >= 2);
        assert!(matches!(ops.first(), Some(CanvasOp::Save)));
        assert!(matches!(ops.last(), Some(CanvasOp::Restore)));
    }

    #[test]
    fn test_rounded_rect() {
        let rect = DrawCmd::Rect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 50.0,
            fill: Some(FillStyle::Color("#fff".to_string())),
            stroke: None,
            corner_radius: Some(10.0),
        };

        let ops = rect.to_canvas_ops();
        // 圆角矩形应该生成路径而不是直接调用 fillRect
        assert!(ops.iter().any(|op| matches!(op, CanvasOp::BeginPath)));
    }
}
