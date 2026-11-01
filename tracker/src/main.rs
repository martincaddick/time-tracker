// Release builds on Windows run without a console window.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod collector;
mod db;
mod server;
#[cfg(windows)]
mod tray;

use std::sync::atomic::AtomicBool;

pub const PORT: u16 = 47615;
/// e.g. "v0.3.0 build 12"
pub fn version() -> String {
    format!("v{} build {}", env!("CARGO_PKG_VERSION"), env!("BUILD_NUMBER"))
}

pub static PAUSED: AtomicBool = AtomicBool::new(false);
pub static QUIT: AtomicBool = AtomicBool::new(false);

pub fn open_page() {
    let url = format!("http://127.0.0.1:{PORT}/");
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("explorer").arg(&url).spawn();
    }
    #[cfg(not(windows))]
    println!("open {url}");
}

fn main() {
    // Binding the port doubles as a single-instance check: if another copy
    // is already running, just open its page and exit.
    let http = match tiny_http::Server::http(("127.0.0.1", PORT)) {
        Ok(s) => s,
        Err(_) => {
            open_page();
            return;
        }
    };

    if let Err(e) = db::init() {
        db::log(&format!("database init failed: {e}"));
        return;
    }

    db::log(&format!("started {}", version()));
    let collector = std::thread::spawn(collector::run);
    std::thread::spawn(move || server::serve(http));

    #[cfg(windows)]
    {
        tray::run(); // blocks until Quit
        QUIT.store(true, std::sync::atomic::Ordering::Relaxed);
        let _ = collector.join(); // lets the open segment flush (<= ~2s)
    }

    #[cfg(not(windows))]
    {
        let _ = collector;
        println!("serving on http://127.0.0.1:{PORT}/ (collector is Windows-only)");
        loop {
            std::thread::sleep(std::time::Duration::from_secs(3600));
        }
    }
}
