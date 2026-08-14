use ab_glyph::{point, Font, FontVec, PxScale, ScaleFont};

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
    pub fn render_vr_menu(
        font: &ab_glyph::FontVec,
        page: usize,
        selection: usize,
        scan_active: bool,
        translation_enabled: bool,
        config: &crate::ovr::OvrConfig,
    ) -> Vec<u8> {
        let w: u32 = 512;
        let h: u32 = 320;
        let mut pixels = vec![0u8; (w * h * 4) as usize];

        // White background with rounded corners (Glassmorphism)
        let bg = [255u8, 255, 255];
        let bg_alpha = 240u8;
        for y in 0..h {
            for x in 0..w {
                let idx = ((y * w + x) * 4) as usize;
                pixels[idx] = bg[0];
                pixels[idx + 1] = bg[1];
                pixels[idx + 2] = bg[2];
                pixels[idx + 3] = bg_alpha;
            }
        }

        // Header bar (light indigo accent)
        for y in 0..36 {
            for x in 0..w {
                let idx = ((y * w + x) * 4) as usize;
                pixels[idx] = 238;
                pixels[idx + 1] = 242;
                pixels[idx + 2] = 255;
                pixels[idx + 3] = 255;
            }
        }

        // Build menu text based on page
        let page_names = [
            "VrcDog主菜单",
            "基础设置",
            "桌面投屏翻译",
            "OCR设置",
            "翻译服务",
            "叠加层外观",
            "高级性能",
            "操作说明",
            "VrcDog社交状态",
            "VrcDog语音输入",
            "VRCLS 日志追踪",
            "OVRAS 空间控制",
            "VRPiano 播放控制",
        ];
        let header = format!("  {}  < 摇杆左右切页 >", page_names[page.min(12)]);

        let items: Vec<String> = if page == 8 {
            vec![
                "  好友列表".to_string(),
                "  当前实例玩家".to_string(),
                "  通知中心".to_string(),
                "  返回主菜单".to_string(),
            ]
        } else if page == 9 {
            vec![
                "  打开语音翻译".to_string(),
                "  打开 VRPiano".to_string(),
                "  打开直播弹幕".to_string(),
                "  返回主菜单".to_string(),
            ]
        } else if page == 10 {
            vec![
                "  打开事件动态".to_string(),
                "  打开统计图表".to_string(),
                "  打开通知中心".to_string(),
                "  返回主菜单".to_string(),
            ]
        } else if page == 11 {
            vec![
                format!("  切换高度: {}", if config.height_toggle_enabled { "开" } else { "关" }),
                "  重置游玩空间".to_string(),
                "  以右手柄修复地板".to_string(),
                "  返回主菜单".to_string(),
            ]
        } else if page == 12 {
            vec![
                "  上一首并用内置播放器播放".to_string(),
                "  播放 / 暂停内置播放器".to_string(),
                "  从头重新播放".to_string(),
                "  下一首并用内置播放器播放".to_string(),
                "  返回语音与媒体菜单".to_string(),
            ]
        } else {
            match page {
            0 => vec![
                "  ► [1] 常规与翻译设置".to_string(),
                "  ► [8] VrcDog: 好友与社交".to_string(),
                "  ► [9] VrcDog: 麦克风语音".to_string(),
                "  ► [10] VRCLS: 游戏内日志".to_string(),
                "  ► [11] OVRAS: 游玩空间".to_string(),
                "  ► [7] 操作说明 (必看)".to_string(),
            ],
            1 => vec![
                format!(
                    "  主功能启用: {}",
                    if translation_enabled { "开" } else { "关" }
                ),
                format!(
                    "  原文与译文切换显示: {}",
                    if config.dual_display { "开" } else { "关" }
                ),
                format!(
                    "  手腕常驻显示: {}",
                    if config.wrist_mode { "开" } else { "关" }
                ),
                "  ◄ 返回主菜单".to_string(),
            ],
            2 => vec![
                format!(
                    "  桌面翻译模式: {}",
                    if config.desktop_mode { "开" } else { "关" }
                ),
                format!(
                    "  自动扫描: {}",
                    if config.auto_scan_enabled {
                        "开"
                    } else {
                        "关"
                    }
                ),
                format!("  扫描间隔: {}秒", config.auto_scan_interval),
                "  ◄ 返回上一页".to_string(),
            ],
            3 => vec![
                format!("  识别语言: {}", config.ocr_language),
                format!(
                    "  图像增强: {}",
                    if config.ocr_image_enhance {
                        "开"
                    } else {
                        "关"
                    }
                ),
                format!("  速度模式: {}", config.ocr_speed_mode),
                "  ◄ 返回上一页".to_string(),
            ],
            4 => vec![
                format!("  服务: {}", config.trans_service),
                format!("  目标语言: {}", config.trans_target_lang),
                "  ◄ 返回上一页".to_string(),
            ],
            5 => vec![
                format!("  锁定模式: {}", config.overlay_lock_mode),
                format!("  透明度: {:.0}%", config.overlay_bg_opacity * 100.0),
                "  ◄ 返回上一页".to_string(),
            ],
            6 => vec![
                format!(
                    "  TTS语音播报: {}",
                    if config.tts_enabled { "开" } else { "关" }
                ),
                format!(
                    "  OSC聊天框输出: {}",
                    if config.osc_chatbox_enabled {
                        "开"
                    } else {
                        "关"
                    }
                ),
                format!(
                    "  扫描框状态: {}",
                    if scan_active { "显示" } else { "隐藏" }
                ),
                "  ◄ 返回上一页".to_string(),
            ],
            7 => vec![
                "  🎮 高级操作指南:".to_string(),
                "  • 双击 [右手B键]: 开关主菜单".to_string(),
                "  • 双击并按住 [右手扳机]: 拖拽框选翻译".to_string(),
                "  • 握住 [右手Grip]+摇杆: 缩放推拉菜单".to_string(),
                "  ◄ 返回主菜单".to_string(),
            ],
            8 => vec![
                "  👥 在线好友: 暂未拉取".to_string(),
                "  📍 当前实例: 加载中...".to_string(),
                "  🔔 通知: 无新通知".to_string(),
                "  [扳机] 手动刷新".to_string(),
                "  ◄ 返回主菜单".to_string(),
            ],
            9 => vec![
                "  🎤 麦克风: 监听中".to_string(),
                "  🔊 语音识别(STT): 开启".to_string(),
                "  🗣️ 文字转语音(TTS): 开启".to_string(),
                "  [扳机] 切换静音".to_string(),
                "  ◄ 返回主菜单".to_string(),
            ],
            10 => vec![
                // VRCLS
                "  📝 最新日志:".to_string(),
                "  [Player Joined] Alice".to_string(),
                "  [Player Left] Bob".to_string(),
                "  [Video] Res URL: www.youtube...".to_string(),
                "  ◄ 返回主菜单".to_string(),
            ],
            11 => vec![
                // OVRAS
                "  🚀 空间X/Z偏移: 0.0m".to_string(),
                "  🚀 高度偏移: 0.3m (切换: 关)".to_string(),
                "  🔄 旋转偏移: 0°".to_string(),
                "  [扳机] 一键重置空间 / 修复地板".to_string(),
                "  ◄ 返回主菜单".to_string(),
            ],
            _ => vec!["  ◄ 返回主菜单".to_string()],
            }
        };

        // Selection highlight
        let sel_y_start = 44 + selection as u32 * 38;
        let sel_y_end = (sel_y_start + 34).min(h);
        for y in sel_y_start..sel_y_end {
            for x in 8..w - 8 {
                let idx = ((y * w + x) * 4) as usize;
                pixels[idx] = 224; // indigo-100 highlight
                pixels[idx + 1] = 231;
                pixels[idx + 2] = 255;
                pixels[idx + 3] = 200;
            }
        }

        // Render text
        let scale = ab_glyph::PxScale::from(22.0);
        let scaled = font.as_scaled(scale);
        let line_h = scaled.height() + scaled.line_gap();

        // Header
        let mut cy = 8.0 + scaled.ascent();
        Self::render_line_to_pixels(
            font,
            &header,
            scale,
            12.0,
            cy,
            &mut pixels,
            w,
            h,
            [49, 46, 129],
        );
        cy = 48.0 + scaled.ascent();

        // Menu items
        for item in &items {
            Self::render_line_to_pixels(
                font,
                item,
                scale,
                12.0,
                cy,
                &mut pixels,
                w,
                h,
                [67, 56, 202],
            );
            cy += line_h + 12.0;
        }

        // Footer
        cy = h as f32 - 22.0;
        Self::render_line_to_pixels(
            font,
            "扳机=确认  摇杆↕=选择  ↔=切页  B长按=关闭",
            ab_glyph::PxScale::from(16.0),
            30.0,
            cy,
            &mut pixels,
            w,
            h,
            [156, 163, 175],
        );

        pixels
    }

    /// Helper: render a single line of text into pixel buffer
    #[allow(clippy::too_many_arguments)]
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
