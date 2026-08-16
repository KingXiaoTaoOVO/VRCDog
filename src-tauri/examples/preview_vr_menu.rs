//! 离线渲染头显菜单各主题预览（PNG）。
//! 用法（在 src-tauri/ 目录）：
//!   cargo run --example preview_vr_menu
//! 输出文件落到仓库根，覆盖现有 vr_menu_*.png。

use ab_glyph::{Font, FontVec};
use image::{ImageBuffer, Rgba};
use std::fs;
use std::path::Path;

use tauri_app_lib::ovr::OvrConfig;
use tauri_app_lib::vr_ui::VrUiRenderer;

fn load_font() -> Option<FontVec> {
    let candidates = [
        r"C:\Windows\Fonts\msyh.ttc",
        r"C:\Windows\Fonts\simhei.ttf",
        r"C:\Windows\Fonts\msyhbd.ttc",
        r"C:\Windows\Fonts\arial.ttf",
        r"C:\Windows\Fonts\segoeui.ttf",
    ];
    for p in candidates.iter() {
        if let Ok(data) = fs::read(p) {
            for idx in 0..4u32 {
                if let Ok(font) = FontVec::try_from_vec_and_index(data.clone(), idx) {
                    if font.glyph_id('\u{4F60}').0 != 0 {
                        return Some(font);
                    }
                }
            }
            if let Ok(font) = FontVec::try_from_vec(data) {
                return Some(font);
            }
        }
    }
    None
}

fn save_png(path: &Path, pixels: &[u8], w: u32, h: u32) {
    let img = ImageBuffer::<Rgba<u8>, _>::from_raw(w, h, pixels.to_vec())
        .expect("pixel buffer size mismatch");
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    img.save(path).expect("failed to save PNG");
    println!("wrote {}", path.display());
}

fn main() {
    let font = load_font().expect("no usable font found in C:\\Windows\\Fonts");
    let repo_root = Path::new("..");
    let w = 1024u32;
    let h = 640u32;

    // 4 themes (must mirror src/theme.ts colors)
    let themes: [(&str, &str, &str, &str, &str); 4] = [
        // (name, accent, bg, text, text_muted)
        ("dog",    "#d97706", "#faf7ed", "#5d4037", "#76584d"),
        ("cat",    "#059669", "#f1f5f0", "#1f3a2e", "#3a5b4a"),
        ("helmet", "#e11d48", "#fff1f3", "#5b1f2f", "#7a3a4a"),
        ("mono",   "#475569", "#f6f6f5", "#1f2937", "#4b5563"),
    ];

    for (name, accent, bg, text, muted) in themes.iter() {
        let mut cfg = OvrConfig::default();
        cfg.vr_menu_accent = accent.to_string();
        cfg.vr_menu_bg = bg.to_string();
        cfg.vr_menu_text = text.to_string();
        cfg.vr_menu_text_muted = muted.to_string();
        // 模拟 page=0 (主菜单) 与 page=1 (基础设置) — 主菜单展示两行层级
        let p0 = VrUiRenderer::render_vr_menu(&font, 0, 1, false, true, &cfg);
        let path = repo_root.join(format!("vr_menu_{}_main.png", name));
        save_png(&path, &p0, w, h);

        let p1 = VrUiRenderer::render_vr_menu(&font, 1, 0, false, true, &cfg);
        let path = repo_root.join(format!("vr_menu_{}_settings.png", name));
        save_png(&path, &p1, w, h);
    }

    // helmet / mono 只保留 main（避免堆太多未跟踪图）
    println!("done.");
}