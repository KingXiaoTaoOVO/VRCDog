//! 输入模拟模块 — 键盘/鼠标事件注入
//!
//! 使用 Windows SendInput API 实现远程输入转发。
//! 支持：鼠标移动、点击、滚轮、键盘按键。

use crate::remote_assist::types::InputEvent;

#[cfg(target_os = "windows")]
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYBD_EVENT_FLAGS,
    KEYEVENTF_KEYUP, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
    MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN,
    MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_WHEEL, MOUSEINPUT,
};

/// 输入模拟器
pub struct InputSimulator {
    screen_width: i32,
    screen_height: i32,
}

impl InputSimulator {
    pub fn new() -> Self {
        // 获取屏幕分辨率用于坐标归一化
        let (w, h) = get_screen_resolution();
        Self {
            screen_width: w,
            screen_height: h,
        }
    }

    /// 处理输入事件
    pub fn handle_event(&self, event: &InputEvent) {
        match event {
            InputEvent::MouseMove { x, y } => {
                self.move_mouse(*x, *y);
            }
            InputEvent::MouseDown { button } => {
                self.mouse_button(*button, true);
            }
            InputEvent::MouseUp { button } => {
                self.mouse_button(*button, false);
            }
            InputEvent::MouseWheel { delta } => {
                self.mouse_wheel(*delta);
            }
            InputEvent::KeyDown { code } => {
                self.key_event(*code, true);
            }
            InputEvent::KeyUp { code } => {
                self.key_event(*code, false);
            }
        }
    }

    #[cfg(target_os = "windows")]
    fn move_mouse(&self, x: i32, y: i32) {
        // 归一化坐标到 0-65535 范围
        let abs_x = (x as f64 / self.screen_width as f64 * 65535.0) as i32;
        let abs_y = (y as f64 / self.screen_height as f64 * 65535.0) as i32;

        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: abs_x,
                    dy: abs_y,
                    dwFlags: MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE,
                    ..Default::default()
                },
            },
        };
        unsafe {
            SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        }
    }

    #[cfg(target_os = "windows")]
    fn mouse_button(&self, button: u8, down: bool) {
        let flags = match (button, down) {
            (0, true) => MOUSEEVENTF_LEFTDOWN,
            (0, false) => MOUSEEVENTF_LEFTUP,
            (1, true) => MOUSEEVENTF_RIGHTDOWN,
            (1, false) => MOUSEEVENTF_RIGHTUP,
            (2, true) => MOUSEEVENTF_MIDDLEDOWN,
            (2, false) => MOUSEEVENTF_MIDDLEUP,
            _ => return,
        };

        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dwFlags: flags,
                    ..Default::default()
                },
            },
        };
        unsafe {
            SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        }
    }

    #[cfg(target_os = "windows")]
    fn mouse_wheel(&self, delta: i32) {
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    mouseData: delta as u32,
                    dwFlags: MOUSEEVENTF_WHEEL,
                    ..Default::default()
                },
            },
        };
        unsafe {
            SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        }
    }

    #[cfg(target_os = "windows")]
    fn key_event(&self, vk_code: u32, down: bool) {
        let flags = if down {
            KEYBD_EVENT_FLAGS(0)
        } else {
            KEYEVENTF_KEYUP
        };

        let input = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(vk_code as u16),
                    dwFlags: flags,
                    ..Default::default()
                },
            },
        };
        unsafe {
            SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn move_mouse(&self, _x: i32, _y: i32) {}
    #[cfg(not(target_os = "windows"))]
    fn mouse_button(&self, _button: u8, _down: bool) {}
    #[cfg(not(target_os = "windows"))]
    fn mouse_wheel(&self, _delta: i32) {}
    #[cfg(not(target_os = "windows"))]
    fn key_event(&self, _code: u32, _down: bool) {}
}

#[cfg(target_os = "windows")]
fn get_screen_resolution() -> (i32, i32) {
    use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
    unsafe {
        let w = GetSystemMetrics(SM_CXSCREEN);
        let h = GetSystemMetrics(SM_CYSCREEN);
        (w, h)
    }
}

#[cfg(not(target_os = "windows"))]
fn get_screen_resolution() -> (i32, i32) {
    (1920, 1080)
}
