//! CSS 颜色字符串工具 — alpha 乘法与提亮
//!
//! deneb 的颜色表示是 Canvas 语义的 CSS 字符串("#rgb"/"#rrggbb"/"rgb()"/"rgba()")。
//! 入场淡入、置灰、hover 提亮都通过颜色字符串变换表达(与 Canvas 2D 渐变/透明度语义一致)。

/// 解析后的 RGBA(0-255, alpha 0-1)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgba {
    /// 红 (0-255)
    pub r: u8,
    /// 绿 (0-255)
    pub g: u8,
    /// 蓝 (0-255)
    pub b: u8,
    /// 透明度 (0-1)
    pub a: f64,
}

fn parse_hex_component(s: &str) -> Option<u8> {
    u8::from_str_radix(s, 16).ok()
}

impl Rgba {
    /// 解析 CSS 颜色字符串;不支持的名字色返回 None(调用方回退原串)
    pub fn parse(color: &str) -> Option<Self> {
        let c = color.trim();
        if let Some(hex) = c.strip_prefix('#') {
            return match hex.len() {
                3 => Some(Self {
                    r: parse_hex_component(&hex[0..1].repeat(2))?,
                    g: parse_hex_component(&hex[1..2].repeat(2))?,
                    b: parse_hex_component(&hex[2..3].repeat(2))?,
                    a: 1.0,
                }),
                4 => Some(Self {
                    r: parse_hex_component(&hex[0..1].repeat(2))?,
                    g: parse_hex_component(&hex[1..2].repeat(2))?,
                    b: parse_hex_component(&hex[2..3].repeat(2))?,
                    a: parse_hex_component(&hex[3..4].repeat(2))? as f64 / 255.0,
                }),
                6 => Some(Self {
                    r: parse_hex_component(&hex[0..2])?,
                    g: parse_hex_component(&hex[2..4])?,
                    b: parse_hex_component(&hex[4..6])?,
                    a: 1.0,
                }),
                8 => Some(Self {
                    r: parse_hex_component(&hex[0..2])?,
                    g: parse_hex_component(&hex[2..4])?,
                    b: parse_hex_component(&hex[4..6])?,
                    a: parse_hex_component(&hex[6..8])? as f64 / 255.0,
                }),
                _ => None,
            };
        }
        let lower = c.to_lowercase();
        if let Some(rest) = lower.strip_prefix("rgba(").and_then(|s| s.strip_suffix(')')) {
            let parts: Vec<&str> = rest.split(',').map(|s| s.trim()).collect();
            if parts.len() == 4 {
                return Some(Self {
                    r: parts[0].parse().ok()?,
                    g: parts[1].parse().ok()?,
                    b: parts[2].parse().ok()?,
                    a: parts[3].parse().ok()?,
                });
            }
        }
        if let Some(rest) = lower.strip_prefix("rgb(").and_then(|s| s.strip_suffix(')')) {
            let parts: Vec<&str> = rest.split(',').map(|s| s.trim()).collect();
            if parts.len() == 3 {
                return Some(Self {
                    r: parts[0].parse().ok()?,
                    g: parts[1].parse().ok()?,
                    b: parts[2].parse().ok()?,
                    a: 1.0,
                });
            }
        }
        None
    }

    /// 输出 "rgba(r,g,b,a)" 字符串
    pub fn to_css(&self) -> String {
        format!("rgba({},{},{},{:.4})", self.r, self.g, self.b, self.a)
    }
}

/// 颜色 alpha 乘法:解析失败时原样返回(容错未知名)。
///
/// 用于置灰(dim × 0.32)、入场淡入等;乘法语义与 Canvas globalAlpha 一致。
pub fn with_alpha(color: &str, alpha: f64) -> String {
    match Rgba::parse(color) {
        Some(rgba) => Rgba { a: (rgba.a * alpha.clamp(0.0, 1.0)).min(1.0), ..rgba }.to_css(),
        None => color.to_string(),
    }
}

/// 向白色混合(hover ColorPop 提亮):amount ∈ [0,1]。
pub fn lighten(color: &str, amount: f64) -> String {
    mix_toward(color, "#ffffff", amount)
}

/// 向目标色混合:amount ∈ [0,1](0 = 原色,1 = 目标色)。
///
/// hover 提亮语义色通道(theme.hover_color)的混色基准 — 与 `lighten` 的
/// 向白混合语义同构,只是目标从白色换成主题语义色。解析失败时原样返回
/// (alpha 通道同样参与插值)。
pub fn mix_toward(color: &str, target: &str, amount: f64) -> String {
    match (Rgba::parse(color), Rgba::parse(target)) {
        (Some(c), Some(t)) => {
            let a = amount.clamp(0.0, 1.0);
            let mix = |a0: u8, b0: u8| -> u8 { (a0 as f64 + (b0 as f64 - a0 as f64) * a).round() as u8 };
            Rgba {
                r: mix(c.r, t.r),
                g: mix(c.g, t.g),
                b: mix(c.b, t.b),
                a: c.a + (t.a - c.a) * a,
            }
            .to_css()
        }
        _ => color.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex() {
        let c = Rgba::parse("#1f77b4").unwrap();
        assert_eq!((c.r, c.g, c.b), (0x1f, 0x77, 0xb4));
        assert_eq!(c.a, 1.0);
        let c3 = Rgba::parse("#fff").unwrap();
        assert_eq!((c3.r, c3.g, c3.b), (255, 255, 255));
    }

    #[test]
    fn test_parse_rgb_function() {
        let c = Rgba::parse("rgb(10, 20, 30)").unwrap();
        assert_eq!((c.r, c.g, c.b), (10, 20, 30));
        let ca = Rgba::parse("rgba(10,20,30,0.5)").unwrap();
        assert!((ca.a - 0.5).abs() < 1e-9);
    }

    #[test]
    fn test_with_alpha() {
        let out = with_alpha("#ff0000", 0.32);
        assert!(out.starts_with("rgba(255,0,0,"), "{}", out);
        // 渐进乘法:rgba 输入再乘
        let out2 = with_alpha(&out, 0.5);
        let parsed = Rgba::parse(&out2).unwrap();
        assert!((parsed.a - 0.16).abs() < 1e-3, "{}", parsed.a);
        // 解析失败回退原串
        assert_eq!(with_alpha("rebeccapurple-ish", 0.5), "rebeccapurple-ish");
    }

    #[test]
    fn test_lighten() {
        let out = lighten("#000000", 0.5);
        let parsed = Rgba::parse(&out).unwrap();
        assert_eq!(parsed.r, 128);
        assert_eq!(parsed.g, 128);
        assert_eq!(parsed.b, 128);
    }

    #[test]
    fn test_mix_toward() {
        // 0 = 原色,1 = 目标色
        let out = mix_toward("#000000", "#ffffff", 0.08);
        let parsed = Rgba::parse(&out).unwrap();
        assert_eq!(parsed.r, 20);
        assert_eq!(parsed.g, 20);
        assert_eq!(parsed.b, 20);
        // 端点
        let keep = mix_toward("#123456", "#abcdef", 0.0);
        let parsed = Rgba::parse(&keep).unwrap();
        assert_eq!((parsed.r, parsed.g, parsed.b), (0x12, 0x34, 0x56));
        let full = mix_toward("#123456", "#abcdef", 1.0);
        let parsed = Rgba::parse(&full).unwrap();
        assert_eq!((parsed.r, parsed.g, parsed.b), (0xab, 0xcd, 0xef));
        // 解析失败回退原串
        assert_eq!(mix_toward("not-a-color", "#ffffff", 0.5), "not-a-color");
    }
}
