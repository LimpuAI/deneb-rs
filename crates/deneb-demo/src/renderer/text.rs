//! fontdue 文本渲染
//!
//! 将 DrawCmd::Text 渲染到 tiny-skia Pixmap。
//! 运行时加载系统字体，支持 TextAnchor/TextBaseline 对齐。

use deneb_core::{TextAnchor, TextBaseline};

/// 字体状态，持有一个 fontdue Font
pub struct FontState {
    font: fontdue::Font,
}

impl FontState {
    /// 从字体字节创建
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let font = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default())
            .map_err(|e| format!("Failed to parse font: {}", e))?;
        Ok(Self { font })
    }

    /// 尝试加载系统字体（macOS / Linux / Windows）
    pub fn load_system_font() -> Result<Self, String> {
        let candidates = [
            "/System/Library/Fonts/Geneva.ttf",
            "/System/Library/Fonts/SFNSMono.ttf",
            "/Library/Fonts/Arial.ttf",
            "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
            "C:\\Windows\\Fonts\\arial.ttf",
            "C:\\Windows\\Fonts\\segoeui.ttf",
        ];

        let mut last_err = "no system font found".to_string();
        for path in &candidates {
            match std::fs::read(path) {
                Ok(bytes) => return Self::from_bytes(&bytes),
                Err(e) => last_err = e.to_string(),
            }
        }

        Err(format!("Failed to load system font: {}", last_err))
    }

    /// 渲染一行文本到 Pixmap
    pub fn draw_text(
        &self,
        pixmap: &mut tiny_skia::Pixmap,
        text: &str,
        x: f64,
        y: f64,
        font_size: f64,
        anchor: TextAnchor,
        baseline: TextBaseline,
        color: &str,
    ) {
        let color = match super::color::parse_color(color) {
            Some(c) => c,
            None => tiny_skia::Color::BLACK,
        };

        let size = font_size as f32;

        let line_metrics = match self.font.horizontal_line_metrics(size) {
            Some(m) => m,
            None => return,
        };

        // 布局：逐字符 rasterize
        let mut glyphs: Vec<(f32, fontdue::Metrics, Vec<u8>)> = Vec::new();
        let mut cursor_x: f32 = 0.0;

        for ch in text.chars() {
            let (metrics, bitmap) = self.font.rasterize(ch, size);
            glyphs.push((cursor_x, metrics, bitmap));
            cursor_x += metrics.advance_width;
        }

        let text_width = cursor_x;

        let x_offset = match anchor {
            TextAnchor::Start => 0.0,
            TextAnchor::Middle => -text_width / 2.0,
            TextAnchor::End => -text_width,
        };

        let y_offset = match baseline {
            TextBaseline::Top => line_metrics.ascent,
            TextBaseline::Middle => (line_metrics.ascent + line_metrics.descent) / 2.0,
            TextBaseline::Bottom => line_metrics.descent,
            TextBaseline::Alphabetic => 0.0,
        };

        let base_x = x as f32 + x_offset;
        let base_y = y as f32 + y_offset;

        let pw = pixmap.width() as i32;
        let ph = pixmap.height() as i32;
        let stride = pixmap.width();
        let pixels = pixmap.pixels_mut();
        let (fr, fg, fb, fa) = (color.red(), color.green(), color.blue(), color.alpha());

        for (glyph_x, metrics, bitmap) in &glyphs {
            let gw = metrics.width;
            let gh = metrics.height;
            if gw == 0 || gh == 0 {
                continue;
            }

            let start_x = (base_x + glyph_x + metrics.xmin as f32).round() as i32;
            let start_y = (base_y + metrics.ymin as f32).round() as i32;

            for py in 0..gh {
                let dst_y = start_y + py as i32;
                if dst_y < 0 || dst_y >= ph {
                    continue;
                }
                for px in 0..gw {
                    let dst_x = start_x + px as i32;
                    if dst_x < 0 || dst_x >= pw {
                        continue;
                    }

                    let coverage = bitmap[py * gw + px] as f32 / 255.0;
                    if coverage < 0.01 {
                        continue;
                    }

                    let idx = (dst_y as u32 * stride + dst_x as u32) as usize;
                    let bg = pixels[idx];

                    let alpha = fa * coverage;
                    let inv = 1.0 - alpha;

                    let r = ((fr * alpha + bg.red() as f32 / 255.0 * inv) * 255.0) as u8;
                    let g = ((fg * alpha + bg.green() as f32 / 255.0 * inv) * 255.0) as u8;
                    let b = ((fb * alpha + bg.blue() as f32 / 255.0 * inv) * 255.0) as u8;
                    let a = ((alpha * 255.0) + bg.alpha() as f32 * inv).min(255.0) as u8;

                    pixels[idx] = tiny_skia::ColorU8::from_rgba(r, g, b, a).premultiply();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_state_load() {
        let _ = FontState::load_system_font();
    }
}
