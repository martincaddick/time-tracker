//! System tray icon + Win32 message loop (Windows only).
//! Left-click opens the page. Right-click menu: Open, Paused, Start with Windows, Quit.

use crate::{open_page, version, PAUSED};
use std::os::windows::process::CommandExt;
use std::process::Command;
use std::sync::atomic::Ordering;
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, PostQuitMessage, TranslateMessage, MSG,
};

const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn reg(args: &[&str]) -> bool {
    Command::new("reg")
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn autostart_enabled() -> bool {
    reg(&["query", RUN_KEY, "/v", "Tracker"])
}

fn toggle_autostart() {
    if autostart_enabled() {
        reg(&["delete", RUN_KEY, "/v", "Tracker", "/f"]);
    } else if let Ok(exe) = std::env::current_exe() {
        let value = format!("\"{}\"", exe.display());
        reg(&["add", RUN_KEY, "/v", "Tracker", "/t", "REG_SZ", "/d", &value, "/f"]);
    }
}

/// 32x32 blue clock face, drawn in code so there is no icon asset to ship.
fn make_icon() -> Icon {
    const N: i32 = 32;
    let mut px = Vec::with_capacity((N * N * 4) as usize);
    for y in 0..N {
        for x in 0..N {
            let (dx, dy) = (x as f32 - 15.5, y as f32 - 15.5);
            let d = (dx * dx + dy * dy).sqrt();
            let hand = (x == 15 || x == 16) && (6..=16).contains(&y)      // minute hand (up)
                || (y == 15 || y == 16) && (16..=23).contains(&x);        // hour hand (right)
            let c: [u8; 4] = if d > 15.0 {
                [0, 0, 0, 0]
            } else if hand {
                [255, 255, 255, 255]
            } else if d > 12.5 {
                [255, 255, 255, 255]
            } else {
                [37, 99, 235, 255]
            };
            px.extend_from_slice(&c);
        }
    }
    Icon::from_rgba(px, N as u32, N as u32).expect("icon")
}

pub fn run() {
    let open = MenuItem::new("Open timeline", true, None);
    let about = MenuItem::new(format!("Tracker {}", version()), false, None); // greyed-out label
    let paused = CheckMenuItem::new("Paused", true, false, None);
    let autostart = CheckMenuItem::new("Start with Windows", true, autostart_enabled(), None);
    let quit = MenuItem::new("Quit", true, None);

    let menu = Menu::new();
    let _ = menu.append_items(&[&open, &paused, &autostart, &PredefinedMenuItem::separator(), &about, &quit]);

    let (open_id, paused_id, auto_id, quit_id) =
        (open.id().clone(), paused.id().clone(), autostart.id().clone(), quit.id().clone());

    // The check items toggle themselves when clicked; the handlers only
    // mirror that change into the app state.
    MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
        if e.id == open_id {
            open_page();
        } else if e.id == paused_id {
            PAUSED.fetch_xor(true, Ordering::Relaxed);
        } else if e.id == auto_id {
            toggle_autostart();
        } else if e.id == quit_id {
            unsafe { PostQuitMessage(0) };
        }
    }));

    TrayIconEvent::set_event_handler(Some(|e: TrayIconEvent| {
        if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
            open_page();
        }
    }));

    let _tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_menu_on_left_click(false)
        .with_tooltip(format!("Tracker {} - click to open timeline", version()))
        .with_icon(make_icon())
        .build()
        .expect("tray icon");

    let mut msg = MSG::default();
    unsafe {
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
