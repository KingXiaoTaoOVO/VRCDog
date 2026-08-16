use ab_glyph::{point, Font, FontVec, PxScale, ScaleFont};

/// Structured menu item for the native VR menu (replaces raw display strings so we
/// can render labels, optional value pills and back/info affordances cleanly).
struct VrMenuItem {
    label: String,
    value: Option<String>,
    back: bool,
    info: bool,
}

pub struct VrUiRenderer;

impl VrUiRenderer {
    pub fn parse_hex_rgb(hex: &str) -> [u8; 3] {
        let hex = hex.trim_start_matches('#');
        if hex.len() >= 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255);
            [r, g, b]
        } else {
            [255, 255, 255]
        }
    }

    /// Render text to RGBA buffer using ab_glyph font rasterization
    #[allow(clippy::too_many_arguments)]
    pub fn render_text_to_rgba(
        font: &FontVec,
        original: &str,
        translated: &str,
        show_original: bool,
        width: u32,
        height: u32,
        text_color: [u8; 3],
        bg_color: [u8; 3],
        bg_alpha: f32,
    ) -> Vec<u8> {
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        let bg_a = (bg_alpha * 255.0).clamp(0.0, 255.0) as u8;

        // Fill background
        for chunk in pixels.chunks_exact_mut(4) {
            chunk[0] = bg_color[0];
            chunk[1] = bg_color[1];
            chunk[2] = bg_color[2];
            chunk[3] = bg_a;
        }

        // Draw rounded border effect (top/bottom gradient lines)
        let border_color = [text_color[0], text_color[1], text_color[2], 60u8];
        for x in 4..width.saturating_sub(4) {
            for dy in 0..2u32 {
                // Top border
                let idx = ((dy * width + x) * 4) as usize;
                if idx + 3 < pixels.len() {
                    pixels[idx..idx + 4].copy_from_slice(&border_color);
                }
                // Bottom border
                let idx = (((height - 1 - dy) * width + x) * 4) as usize;
                if idx + 3 < pixels.len() {
                    pixels[idx..idx + 4].copy_from_slice(&border_color);
                }
            }
        }

        let scale = PxScale::from(34.0); // Larger font for VR readability
        let scaled_font = font.as_scaled(scale);

        let text_to_render = if show_original && !original.is_empty() {
            format!("{}\n─────\n{}", translated, original)
        } else {
            translated.to_string()
        };

        let line_height = scaled_font.height() + scaled_font.line_gap();
        let padding = 16.0f32;
        let max_text_width = width as f32 - padding * 2.0;
        let mut cursor_y = padding + scaled_font.ascent();

        for line in text_to_render.lines() {
            let mut cursor_x = padding;

            for ch in line.chars() {
                let glyph_id = font.glyph_id(ch);
                let advance = scaled_font.h_advance(glyph_id);

                // Auto word-wrap: if glyph exceeds width, move to next line
                if cursor_x + advance > padding + max_text_width {
                    cursor_x = padding;
                    cursor_y += line_height;
                    if cursor_y > height as f32 - padding {
                        break;
                    }
                }

                let glyph = glyph_id.with_scale_and_position(scale, point(cursor_x, cursor_y));

                if let Some(outlined) = font.outline_glyph(glyph) {
                    let bounds = outlined.px_bounds();
                    outlined.draw(|gx, gy, coverage| {
                        let px = bounds.min.x as i32 + gx as i32;
                        let py = bounds.min.y as i32 + gy as i32;
                        if px >= 0 && py >= 0 && (px as u32) < width && (py as u32) < height {
                            let idx = ((py as u32 * width + px as u32) * 4) as usize;
                            if idx + 3 < pixels.len() {
                                let alpha = (coverage * 255.0) as u8;
                                let inv = 255 - alpha;
                                pixels[idx] = ((text_color[0] as u16 * alpha as u16
                                    + pixels[idx] as u16 * inv as u16)
                                    / 255) as u8;
                                pixels[idx + 1] = ((text_color[1] as u16 * alpha as u16
                                    + pixels[idx + 1] as u16 * inv as u16)
                                    / 255) as u8;
                                pixels[idx + 2] = ((text_color[2] as u16 * alpha as u16
                                    + pixels[idx + 2] as u16 * inv as u16)
                                    / 255) as u8;
                                pixels[idx + 3] = pixels[idx + 3].max(alpha);
                            }
                        }
                    });
                }

                cursor_x += advance;
            }

            cursor_y += line_height;
            if cursor_y > height as f32 - padding {
                break;
            }
        }

        pixels
    }

    /// Render a configurable translucent glass text panel for VR overlays.
    #[allow(clippy::too_many_arguments)]
    pub fn render_text_to_rgba_styled(
        font: &FontVec,
        original: &str,
        translated: &str,
        show_original: bool,
        width: u32,
        height: u32,
        text_color: [u8; 3],
        bg_color: [u8; 3],
        bg_alpha: f32,
        font_size: f32,
        border_alpha: f32,
        corner_radius: u32,
        shadow_strength: f32,
    ) -> Vec<u8> {
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        let bg_a = (bg_alpha * 255.0).clamp(0.0, 255.0) as u8;
        let border_a = (border_alpha * 255.0).clamp(0.0, 255.0) as u8;
        let shadow_a = (shadow_strength * 110.0).clamp(0.0, 140.0) as u8;
        let radius = corner_radius.min(width.min(height) / 3).max(2);

        for y in 0..height {
            for x in 0..width {
                let idx = ((y * width + x) * 4) as usize;
                let dx = if x < radius {
                    radius - x
                } else if x >= width.saturating_sub(radius) {
                    x - (width - radius - 1)
                } else {
                    0
                };
                let dy = if y < radius {
                    radius - y
                } else if y >= height.saturating_sub(radius) {
                    y - (height - radius - 1)
                } else {
                    0
                };
                let inside = dx == 0 || dy == 0 || dx * dx + dy * dy <= radius * radius;
                if !inside {
                    continue;
                }

                let vertical = y as f32 / height.max(1) as f32;
                let lift = (18.0 * (1.0 - vertical)) as u8;
                pixels[idx] = bg_color[0].saturating_add(lift);
                pixels[idx + 1] = bg_color[1].saturating_add(lift);
                pixels[idx + 2] = bg_color[2].saturating_add(lift);
                pixels[idx + 3] = bg_a;

                let border_band =
                    x < 3 || y < 3 || x >= width.saturating_sub(3) || y >= height.saturating_sub(3);
                if border_band {
                    pixels[idx] = ((text_color[0] as u16 + pixels[idx] as u16) / 2) as u8;
                    pixels[idx + 1] = ((text_color[1] as u16 + pixels[idx + 1] as u16) / 2) as u8;
                    pixels[idx + 2] = ((text_color[2] as u16 + pixels[idx + 2] as u16) / 2) as u8;
                    pixels[idx + 3] = pixels[idx + 3].max(border_a);
                }

                let shadow_band = y > height.saturating_sub(18) || x > width.saturating_sub(18);
                if shadow_band {
                    pixels[idx] = pixels[idx].saturating_sub(10);
                    pixels[idx + 1] = pixels[idx + 1].saturating_sub(12);
                    pixels[idx + 2] = pixels[idx + 2].saturating_sub(14);
                    pixels[idx + 3] = pixels[idx + 3].max(shadow_a);
                }
            }
        }

        let scale = PxScale::from(font_size.clamp(18.0, 56.0));
        let scaled_font = font.as_scaled(scale);
        let separator = "------------";
        let text_to_render = if show_original && !original.is_empty() {
            format!("{}\n{}\n{}", translated, separator, original)
        } else {
            translated.to_string()
        };

        let line_height = scaled_font.height() + scaled_font.line_gap() + 4.0;
        let padding = (font_size * 0.65).clamp(14.0, 28.0);
        let max_text_width = width as f32 - padding * 2.0;
        let mut cursor_y = padding + scaled_font.ascent();

        for line in text_to_render.lines() {
            let mut cursor_x = padding;

            for ch in line.chars() {
                let glyph_id = font.glyph_id(ch);
                let advance = scaled_font.h_advance(glyph_id);
                if cursor_x + advance > padding + max_text_width {
                    cursor_x = padding;
                    cursor_y += line_height;
                    if cursor_y > height as f32 - padding {
                        break;
                    }
                }

                let glyph = glyph_id.with_scale_and_position(scale, point(cursor_x, cursor_y));
                if let Some(outlined) = font.outline_glyph(glyph) {
                    let bounds = outlined.px_bounds();
                    outlined.draw(|gx, gy, coverage| {
                        let px = bounds.min.x as i32 + gx as i32;
                        let py = bounds.min.y as i32 + gy as i32;
                        if px >= 0 && py >= 0 && (px as u32) < width && (py as u32) < height {
                            let idx = ((py as u32 * width + px as u32) * 4) as usize;
                            if idx + 3 < pixels.len() {
                                let alpha = (coverage * 255.0) as u8;
                                let inv = 255 - alpha;
                                pixels[idx] = ((text_color[0] as u16 * alpha as u16
                                    + pixels[idx] as u16 * inv as u16)
                                    / 255) as u8;
                                pixels[idx + 1] = ((text_color[1] as u16 * alpha as u16
                                    + pixels[idx + 1] as u16 * inv as u16)
                                    / 255) as u8;
                                pixels[idx + 2] = ((text_color[2] as u16 * alpha as u16
                                    + pixels[idx + 2] as u16 * inv as u16)
                                    / 255) as u8;
                                pixels[idx + 3] = pixels[idx + 3].max(alpha);
                            }
                        }
                    });
                }

                cursor_x += advance;
            }

            cursor_y += line_height;
            if cursor_y > height as f32 - padding {
                break;
            }
        }

        pixels
    }

    // ==================== VR Thread ====================

    /// Render scan frame overlay with custom color (border, transparent center, corner marks)
    pub fn render_scan_frame(width: u32, height: u32, color_hex: &str) -> Vec<u8> {
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        let border = 6u32;
        let corner_len = 30u32;
        let rgb = Self::parse_hex_rgb(color_hex);
        let bright: [u8; 4] = [rgb[0], rgb[1], rgb[2], 220];
        let dim: [u8; 4] = [
            (rgb[0] as f32 * 0.78) as u8,
            (rgb[1] as f32 * 0.78) as u8,
            (rgb[2] as f32 * 0.78) as u8,
            80,
        ];

        for y in 0..height {
            for x in 0..width {
                let idx = ((y * width + x) * 4) as usize;
                let on_top = y < border;
                let on_bottom = y >= height - border;
                let on_left = x < border;
                let on_right = x >= width - border;

                // Corner marks (bright color)
                let in_corner = (x < corner_len || x >= width - corner_len)
                    && (y < corner_len || y >= height - corner_len);

                if (on_top || on_bottom || on_left || on_right) && in_corner {
                    pixels[idx..idx + 4].copy_from_slice(&bright);
                } else if on_top || on_bottom || on_left || on_right {
                    // Dim border lines between corners
                    pixels[idx..idx + 4].copy_from_slice(&dim);
                }
                // Center stays transparent (alpha = 0)
            }
        }

        // Add crosshair at center
        let cx = width / 2;
        let cy = height / 2;
        for d in 0..10u32 {
            for (px, py) in [(cx + d, cy), (cx - d, cy), (cx, cy + d), (cx, cy - d)] {
                if px < width && py < height {
                    let idx = ((py * width + px) * 4) as usize;
                    pixels[idx..idx + 4].copy_from_slice(&[rgb[0], rgb[1], rgb[2], 150]);
                }
            }
        }

        pixels
    }

    /// Render interactive VR menu page
    /// Render interactive VR menu page with the app's glass theme.
    pub fn render_vr_menu(
        font: &ab_glyph::FontVec,
        page: usize,
        selection: usize,
        scan_active: bool,
        translation_enabled: bool,
        config: &crate::ovr::OvrConfig,
    ) -> Vec<u8> {
        let w: u32 = 1024;
        let h: u32 = 640;
        let mut pixels = vec![0u8; (w * h * 4) as usize];

        // ---- Live theme colors (fall back to the "dog" theme) ----
        let accent = Self::parse_hex_rgb(&theme_or(&config.vr_menu_accent, "#d97706"));
        let bg = Self::parse_hex_rgb(&theme_or(&config.vr_menu_bg, "#faf7ed"));
        let text_c = Self::parse_hex_rgb(&theme_or(&config.vr_menu_text, "#5d4037"));
        let muted = Self::parse_hex_rgb(&theme_or(&config.vr_menu_text_muted, "#76584d"));
        // Neutral, very subtle hairline — never an accent/colored bar.
        let border = mix(bg, text_c, 0.14);
        // PC-style active/selected state: a soft accent wash, not a solid block.
        let header_bg = mix(bg, accent, 0.06);
        let sel_bg = mix(bg, accent, 0.18);
        // Faint lifted card for ordinary rows.
        let card = mix(bg, [255u8, 255, 255], 0.5);
        let green = [22u8, 163, 108];
        let gray = [150u8, 140, 128];

        // ---- Floating rounded glass panel (no colored edges) ----
        let px0 = 16i32;
        let py0 = 16i32;
        let pw = 992u32;
        let ph = 608u32;
        let radius = 32u32;
        // Soft drop shadow for depth.
        fill_rounded(&mut pixels, w, h, px0 + 10, py0 + 16, pw, ph, radius, [40, 30, 20], 0.16);
        // Hairline neutral border ring.
        fill_rounded(&mut pixels, w, h, px0, py0, pw, ph, radius, border, 1.0);
        // Glass body.
        fill_rounded(&mut pixels, w, h, px0 + 2, py0 + 2, pw - 4, ph - 4, radius - 2, bg, 1.0);
        // Top sheen.
        fill_rounded(&mut pixels, w, h, px0 + 2, py0 + 2, pw - 4, 54, radius - 2, mix(bg, [255, 255, 255], 0.5), 0.30);

        // ---- Header ----
        let header_h: i32 = 84;
        let hy = py0 + header_h;
        fill_rounded(&mut pixels, w, h, px0 + 2, py0 + 2, pw - 4, header_h as u32, radius - 2, header_bg, 0.9);
        for x in (px0 + 28)..(px0 + pw as i32 - 28) {
            let idx = (((hy as u32) * w + x as u32) * 4) as usize;
            blend(&mut pixels, idx, border, 0.28);
        }
        draw_text(&mut pixels, w, h, font, "VrcDog", 46.0, 48.0, py0 as f32 + 58.0, accent, pw as f32 - 60.0, 0);
        let page_names = [
            "VrcDog 主菜单", "基础设置", "桌面投屏翻译", "OCR 设置", "翻译服务", "叠加层外观",
            "高级性能", "操作说明", "VrcDog 社交状态", "VrcDog 语音输入", "VRCLS 日志追踪",
            "OVRAS 空间控制", "VRPiano 播放控制", "VRChat 自动绘画",
        ];
        let pname = page_names[page.min(13)];
        draw_text(&mut pixels, w, h, font, pname, 30.0, 40.0, py0 as f32 + 56.0, text_c, pw as f32 - 80.0, 2);
        fill_rounded(&mut pixels, w, h, px0 + pw as i32 - 52, py0 + 34, 16, 16, 8, if translation_enabled { green } else { gray }, 1.0);

        // ---- Left page tab strip (OVRAS-style orientation) ----
        let tab_x = 32i32;
        let tab_w = 216u32;
        let area_top = 116i32;
        let area_bottom = 584i32;
        let tab_h = ((area_bottom - area_top) as f32 / 14.0) as i32;
        let tab_short = [
            "主菜单", "基础设置", "投屏翻译", "OCR 设置", "翻译服务", "叠加外观", "性能", "操作说明",
            "社交状态", "语音输入", "日志追踪", "空间控制", "钢琴控制", "自动绘画",
        ];
        for i in 0..14 {
            let ty = area_top + i * tab_h;
            let active = i == (page as i32);
            let ry = ty + 3;
            let rh = (tab_h - 6) as u32;
            if active {
                fill_rounded(&mut pixels, w, h, tab_x, ry, tab_w, rh, 12, sel_bg, 1.0);
                let lbl = fit_text(font, tab_short[i as usize], 20.0, tab_w as f32 - 24.0);
                draw_text(&mut pixels, w, h, font, &lbl, 20.0, tab_x as f32 + 14.0, ry as f32 + rh as f32 / 2.0 + 7.0, accent, tab_w as f32 - 24.0, 0);
            } else {
                draw_text(&mut pixels, w, h, font, tab_short[i as usize], 19.0, tab_x as f32 + 14.0, ry as f32 + rh as f32 / 2.0 + 7.0, muted, tab_w as f32 - 24.0, 0);
            }
        }

        // ---- Content list ----
        let c_x = 272i32;
        let c_w = (px0 + pw as i32 - 16) - c_x;
        let item_h = 66i32;
        let gap = 8i32;
        let base_y = 120i32;
        let items = build_vr_menu_items(page, scan_active, translation_enabled, config);
        for (i, item) in items.iter().enumerate() {
            let iy = base_y + i as i32 * (item_h + gap);
            if iy + item_h > area_bottom {
                break;
            }
            let rect = (c_x, iy, c_w as u32, item_h as u32);
            let selected = i == selection;
            if selected {
                // Soft accent wash only — no border, no vertical bar.
                fill_rounded(&mut pixels, w, h, rect.0, rect.1, rect.2, rect.3, 16, sel_bg, 1.0);
            } else if item.back {
                fill_rounded(&mut pixels, w, h, rect.0, rect.1, rect.2, rect.3, 16, mix(bg, text_c, 0.06), 1.0);
            } else {
                fill_rounded(&mut pixels, w, h, rect.0, rect.1, rect.2, rect.3, 16, card, 0.55);
            }
            let tcol = if item.info {
                muted
            } else if selected {
                accent
            } else {
                text_c
            };
            let lsize = if item.info { 22.0 } else { 28.0 };
            let lx = rect.0 as f32 + 20.0;
            let baseline = rect.1 as f32 + rect.3 as f32 / 2.0 + lsize * 0.35;
            let max_lw = if item.value.is_some() { rect.2 as f32 - 210.0 } else { rect.2 as f32 - 40.0 };
            let label = fit_text(font, &item.label, lsize, max_lw);
            draw_text(&mut pixels, w, h, font, &label, lsize, lx, baseline, tcol, max_lw, 0);
            if let Some(v) = &item.value {
                let psize = 24.0;
                let vw = text_width(font, v, PxScale::from(psize)) + 40.0;
                let pvx = rect.0 as f32 + rect.2 as f32 - vw - 16.0;
                let pvy = rect.1 as f32 + (rect.3 as f32 - 42.0) / 2.0;
                let (pbg, pfg) = if is_on(v) {
                    (green, [255, 255, 255])
                } else if is_off(v) {
                    (gray, [255, 255, 255])
                } else {
                    (mix(bg, accent, 0.35), accent)
                };
                fill_rounded(&mut pixels, w, h, pvx as i32, pvy as i32, vw as u32, 42, 21, pbg, 1.0);
                let vfit = fit_text(font, v, psize, vw - 32.0);
                draw_text(&mut pixels, w, h, font, &vfit, psize, pvx + 20.0, pvy + 42.0 / 2.0 + psize * 0.35, pfg, vw - 32.0, 1);
            }
        }

        // ---- Footer hints ----
        let footer = "扳机=确认    摇杆上下=选择    摇杆左右=切页    B键=关闭菜单";
        draw_text(&mut pixels, w, h, font, footer, 22.0, 40.0, 584.0 + 28.0, muted, pw as f32 - 80.0, 1);

        pixels
    }


    /// Helper: render a single line of text into pixel buffer
    #[allow(clippy::too_many_arguments, dead_code)]
    fn render_line_to_pixels(
        font: &ab_glyph::FontVec,
        text: &str,
        scale: ab_glyph::PxScale,
        start_x: f32,
        start_y: f32,
        pixels: &mut [u8],
        width: u32,
        height: u32,
        color: [u8; 3],
    ) {
        use ab_glyph::{point, Font, ScaleFont};
        let scaled = font.as_scaled(scale);
        let mut cx = start_x;
        for ch in text.chars() {
            let glyph_id = font.glyph_id(ch);
            let advance = scaled.h_advance(glyph_id);
            if cx + advance > width as f32 - 8.0 {
                break;
            }
            let glyph = glyph_id.with_scale_and_position(scale, point(cx, start_y));
            if let Some(outlined) = font.outline_glyph(glyph) {
                let bounds = outlined.px_bounds();
                outlined.draw(|gx, gy, cov| {
                    let px = bounds.min.x as i32 + gx as i32;
                    let py = bounds.min.y as i32 + gy as i32;
                    if px >= 0 && py >= 0 && (px as u32) < width && (py as u32) < height {
                        let idx = ((py as u32 * width + px as u32) * 4) as usize;
                        if idx + 3 < pixels.len() {
                            let a = (cov * 255.0) as u8;
                            let inv = 255 - a;
                            pixels[idx] = ((color[0] as u16 * a as u16
                                + pixels[idx] as u16 * inv as u16)
                                / 255) as u8;
                            pixels[idx + 1] = ((color[1] as u16 * a as u16
                                + pixels[idx + 1] as u16 * inv as u16)
                                / 255) as u8;
                            pixels[idx + 2] = ((color[2] as u16 * a as u16
                                + pixels[idx + 2] as u16 * inv as u16)
                                / 255) as u8;
                            pixels[idx + 3] = pixels[idx + 3].max(a);
                        }
                    }
                });
            }
            cx += advance;
        }
    }
}
// ==================== VR Menu helper functions ====================

fn theme_or(s: &str, def: &str) -> String {
    if s.trim().is_empty() {
        def.to_string()
    } else {
        s.to_string()
    }
}

fn lerp(a: u8, b: u8, t: f32) -> u8 {
    let v = a as f32 + (b as f32 - a as f32) * t.clamp(0.0, 1.0);
    v.round().clamp(0.0, 255.0) as u8
}

fn mix(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    [lerp(a[0], b[0], t), lerp(a[1], b[1], t), lerp(a[2], b[2], t)]
}

fn blend(px: &mut [u8], idx: usize, color: [u8; 3], alpha: f32) {
    if idx + 3 >= px.len() {
        return;
    }
    let a = (alpha.clamp(0.0, 1.0) * 255.0) as u8;
    if a == 0 {
        return;
    }
    let inv = 255 - a;
    px[idx] = ((color[0] as u16 * a as u16 + px[idx] as u16 * inv as u16) / 255) as u8;
    px[idx + 1] = ((color[1] as u16 * a as u16 + px[idx + 1] as u16 * inv as u16) / 255) as u8;
    px[idx + 2] = ((color[2] as u16 * a as u16 + px[idx + 2] as u16 * inv as u16) / 255) as u8;
    px[idx + 3] = px[idx + 3].max(a);
}

fn fill_rounded(
    px: &mut [u8],
    w: u32,
    h: u32,
    rx: i32,
    ry: i32,
    rw: u32,
    rh: u32,
    radius: u32,
    color: [u8; 3],
    alpha: f32,
) {
    let r = radius.min(rw / 2).min(rh / 2).max(1);
    let x1 = rx.max(0);
    let y1 = ry.max(0);
    let x2 = (rx + rw as i32).min(w as i32).max(0);
    let y2 = (ry + rh as i32).min(h as i32).max(0);
    for y in y1..y2 {
        for x in x1..x2 {
            let dx = if x < rx + r as i32 {
                r as i32 - (x - rx)
            } else if x >= rx + rw as i32 - r as i32 {
                (x - rx) - (rw as i32 - r as i32 - 1)
            } else {
                0
            };
            let dy = if y < ry + r as i32 {
                r as i32 - (y - ry)
            } else if y >= ry + rh as i32 - r as i32 {
                (y - ry) - (rh as i32 - r as i32 - 1)
            } else {
                0
            };
            let inside = dx == 0 || dy == 0 || dx * dx + dy * dy <= r as i32 * r as i32;
            if inside {
                let idx = ((y as u32 * w + x as u32) * 4) as usize;
                blend(px, idx, color, alpha);
            }
        }
    }
}

fn text_width(font: &FontVec, text: &str, scale: PxScale) -> f32 {
    let sf = font.as_scaled(scale);
    let mut width = 0.0f32;
    for ch in clean_text(text).chars() {
        width += sf.h_advance(font.glyph_id(ch));
    }
    width
}

fn draw_text(
    px: &mut [u8],
    w: u32,
    h: u32,
    font: &FontVec,
    text: &str,
    size: f32,
    x: f32,
    y: f32,
    color: [u8; 3],
    max_w: f32,
    align: u8,
) {
    if max_w <= 0.0 {
        return;
    }
    let scale = PxScale::from(size);
    let cleaned = clean_text(text);
    let tw = text_width(font, &cleaned, scale);
    let start = match align {
        0 => x,
        1 => x + (max_w - tw) / 2.0,
        _ => x + max_w - tw,
    };
    let mut cx = start.max(x);
    for ch in cleaned.chars() {
        let gid = font.glyph_id(ch);
        let adv = font.as_scaled(scale).h_advance(gid);
        if cx + adv > x + max_w {
            break;
        }
        let glyph = gid.with_scale_and_position(scale, point(cx, y));
        if let Some(outlined) = font.outline_glyph(glyph) {
            let b = outlined.px_bounds();
            outlined.draw(|gx, gy, cov| {
                let px2 = b.min.x as i32 + gx as i32;
                let py2 = b.min.y as i32 + gy as i32;
                if px2 >= 0 && py2 >= 0 && (px2 as u32) < w && (py2 as u32) < h {
                    let idx = ((py2 as u32 * w + px2 as u32) * 4) as usize;
                    blend(px, idx, color, cov);
                }
            });
        }
        cx += adv;
    }
}

fn fit_text(font: &FontVec, text: &str, size: f32, max_w: f32) -> String {
    let scale = PxScale::from(size);
    let cleaned = clean_text(text);
    if text_width(font, &cleaned, scale) <= max_w {
        return cleaned;
    }
    let sf = font.as_scaled(scale);
    let mut out = String::new();
    let mut cur = 0.0f32;
    for ch in cleaned.chars() {
        let adv = sf.h_advance(font.glyph_id(ch));
        if cur + adv > max_w - 14.0 {
            break;
        }
        out.push(ch);
        cur += adv;
    }
    out.push('…');
    out
}

fn clean_text(s: &str) -> String {
    s.chars()
        .filter(|&c| {
            let cp = c as u32;
            if cp == 0xFE0F {
                return false;
            }
            if (0x2190..=0x21FF).contains(&cp) {
                return false;
            }
            if (0x2300..=0x23FF).contains(&cp) {
                return false;
            }
            if (0x25A0..=0x25FF).contains(&cp) {
                return false;
            }
            if (0x2600..=0x27BF).contains(&cp) {
                return false;
            }
            if (0x2B00..=0x2BFF).contains(&cp) {
                return false;
            }
            if (0x1F000..=0x1FAFF).contains(&cp) {
                return false;
            }
            if (0x1F1E6..=0x1F1FF).contains(&cp) {
                return false;
            }
            true
        })
        .collect()
}

fn is_on(v: &str) -> bool {
    matches!(v, "开" | "启用" | "开启" | "是" | "On" | "ON" | "on" | "启用中" | "监听中" | "显示")
}

fn is_off(v: &str) -> bool {
    matches!(v, "关" | "禁用" | "关闭" | "否" | "Off" | "off" | "隐藏" | "暂停")
}

fn mi(label: &str) -> VrMenuItem {
    VrMenuItem { label: label.to_string(), value: None, back: false, info: false }
}
fn mib(label: &str) -> VrMenuItem {
    VrMenuItem { label: label.to_string(), value: None, back: true, info: false }
}
fn mii(label: &str) -> VrMenuItem {
    VrMenuItem { label: label.to_string(), value: None, back: false, info: true }
}
fn miv(label: &str, v: String) -> VrMenuItem {
    VrMenuItem { label: label.to_string(), value: Some(v), back: false, info: false }
}

fn build_vr_menu_items(
    page: usize,
    scan_active: bool,
    translation_enabled: bool,
    config: &crate::ovr::OvrConfig,
) -> Vec<VrMenuItem> {
    match page {
        8 => vec![
            mi("好友列表"),
            mi("当前实例玩家"),
            mi("通知中心"),
            mib("返回主菜单"),
        ],
        9 => vec![
            mi("打开语音翻译"),
            mi("打开 VRPiano"),
            mi("打开直播弹幕"),
            mi("打开自动绘画"),
            mib("返回主菜单"),
        ],
        10 => vec![
            mi("打开事件动态"),
            mi("打开统计图表"),
            mi("打开通知中心"),
            mib("返回主菜单"),
        ],
        11 => vec![
            miv("切换高度", (if config.height_toggle_enabled { "开" } else { "关" }).to_string()),
            mi("重置游玩空间"),
            mi("以右手柄修复地板"),
            mib("返回主菜单"),
        ],
        12 => vec![
            mi("上一首并播放"),
            mi("播放 / 暂停"),
            mi("从头重新播放"),
            mi("下一首并播放"),
            mib("返回语音与媒体"),
        ],
        13 => {
            let (state, progress) = crate::vrdrawing::vr_status_lines();
            vec![
                mi(&format!("开始绘画 · {state}")),
                mi(&format!("暂停 / 继续 · {progress}")),
                mi("停止并释放画笔"),
                mi("打开完整绘画工作台"),
                mib("返回语音与媒体"),
            ]
        }
        _ => match page {
            0 => vec![
                mi("常规与翻译设置"),
                mi("VrcDog · 好友与社交"),
                mi("VrcDog · 麦克风语音"),
                mi("VRCLS · 游戏内日志"),
                mi("OVRAS · 游玩空间"),
                mi("操作说明 (必看)"),
            ],
            1 => vec![
                miv("主功能启用", (if translation_enabled { "开" } else { "关" }).to_string()),
                miv("原文 / 译文切换", (if config.dual_display { "开" } else { "关" }).to_string()),
                miv("手腕常驻显示", (if config.wrist_mode { "开" } else { "关" }).to_string()),
                mib("返回主菜单"),
            ],
            2 => vec![
                miv("桌面翻译模式", (if config.desktop_mode { "开" } else { "关" }).to_string()),
                miv("自动扫描", (if config.auto_scan_enabled { "开" } else { "关" }).to_string()),
                miv("扫描间隔", format!("{}秒", config.auto_scan_interval)),
                mib("返回上一页"),
            ],
            3 => vec![
                miv("识别语言", config.ocr_language.clone()),
                miv("图像增强", (if config.ocr_image_enhance { "开" } else { "关" }).to_string()),
                miv("速度模式", config.ocr_speed_mode.clone()),
                mib("返回上一页"),
            ],
            4 => vec![
                miv("翻译服务", config.trans_service.clone()),
                miv("目标语言", config.trans_target_lang.clone()),
                mib("返回上一页"),
            ],
            5 => vec![
                miv("锁定模式", config.overlay_lock_mode.clone()),
                miv("透明度", format!("{:.0}%", config.overlay_bg_opacity * 100.0)),
                mib("返回上一页"),
            ],
            6 => vec![
                miv("TTS 语音播报", (if config.tts_enabled { "开" } else { "关" }).to_string()),
                miv("OSC 聊天框输出", (if config.osc_chatbox_enabled { "开" } else { "关" }).to_string()),
                miv("扫描框状态", (if scan_active { "显示" } else { "隐藏" }).to_string()),
                mib("返回上一页"),
            ],
            7 => vec![
                mii("高级操作指南"),
                mii("双击 右手B键: 开关主菜单"),
                mii("双击按住 右手扳机: 拖拽框选翻译"),
                mii("握住 右手Grip+摇杆: 缩放推拉菜单"),
                mib("返回主菜单"),
            ],
            _ => vec![mib("返回主菜单")],
        },
    }
}
