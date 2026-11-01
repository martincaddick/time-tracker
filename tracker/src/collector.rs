//! Samples the foreground window every POLL_MS and merges consecutive
//! identical (process, title) samples into segments.

use crate::{db, PAUSED, QUIT};
use rusqlite::{params, Connection};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const POLL_MS: u64 = 2_000;
/// Input within this many ms of a sample counts that interval as "active".
const ACTIVE_WINDOW_MS: u32 = 5_000;
/// A gap between polls longer than this means sleep/hibernate.
const GAP_MS: u128 = 30_000;
/// The open segment is saved at most this often (and always when it closes).
const FLUSH_EVERY: Duration = Duration::from_secs(15);

/// Never recorded: your own tool and Windows UI chrome. Edit to taste.
const IGNORE_PROCESSES: &[&str] = &["tracker.exe"];
const IGNORE_TITLES: &[&str] = &[
    "System tray overflow window.",
    "Snipping Tool Overlay",
    "Task Switching",
    "Task View",
];

fn ignored(s: &Sample) -> bool {
    IGNORE_PROCESSES.iter().any(|p| s.process.eq_ignore_ascii_case(p))
        || IGNORE_TITLES.iter().any(|t| s.title == *t)
}

pub struct Sample {
    pub process: String,
    pub title: String,
}

struct Segment {
    id: i64,
    process: String,
    title: String,
    end: i64,
    active_ms: u64,
    total_ms: u64,
    last_flush: Instant,
}

fn unix_now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64
}

#[cfg(windows)]
mod win {
    use super::Sample;
    use std::mem::size_of;
    use windows::core::PWSTR;
    use windows::Win32::Foundation::{CloseHandle, MAX_PATH};
    use windows::Win32::System::SystemInformation::GetTickCount;
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
    };

    /// Milliseconds since the last keyboard/mouse input.
    pub fn idle_ms() -> u32 {
        unsafe {
            let mut info = LASTINPUTINFO {
                cbSize: size_of::<LASTINPUTINFO>() as u32,
                dwTime: 0,
            };
            if GetLastInputInfo(&mut info).as_bool() {
                GetTickCount().wrapping_sub(info.dwTime)
            } else {
                0
            }
        }
    }

    fn process_name(pid: u32) -> String {
        unsafe {
            let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
                return "unknown".into();
            };
            let mut buf = [0u16; MAX_PATH as usize];
            let mut len = buf.len() as u32;
            let ok = QueryFullProcessImageNameW(
                handle,
                PROCESS_NAME_WIN32,
                PWSTR(buf.as_mut_ptr()),
                &mut len,
            )
            .is_ok();
            let _ = CloseHandle(handle);
            if !ok {
                return "unknown".into();
            }
            let path = String::from_utf16_lossy(&buf[..len as usize]);
            path.rsplit('\\').next().unwrap_or("unknown").to_string()
        }
    }

    pub fn foreground() -> Option<Sample> {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.is_invalid() {
                return None; // lock screen, UAC prompt, desktop switch
            }
            let n = GetWindowTextLengthW(hwnd);
            let mut buf = vec![0u16; (n as usize) + 1];
            let copied = GetWindowTextW(hwnd, &mut buf);
            let title = String::from_utf16_lossy(&buf[..copied.max(0) as usize]);
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            Some(Sample { process: process_name(pid), title })
        }
    }
}

#[cfg(not(windows))]
mod win {
    use super::Sample;
    pub fn idle_ms() -> u32 {
        0
    }
    pub fn foreground() -> Option<Sample> {
        None // collector only runs on Windows; the web UI works anywhere
    }
}

fn open_segment(conn: &Connection, s: &Sample, now: i64) -> rusqlite::Result<Segment> {
    conn.execute(
        "INSERT INTO segments (process, title, start, end, active_secs, total_secs)
         VALUES (?1, ?2, ?3, ?3, 0, 0)",
        params![s.process, s.title, now],
    )?;
    Ok(Segment {
        id: conn.last_insert_rowid(),
        process: s.process.clone(),
        title: s.title.clone(),
        end: now,
        active_ms: 0,
        total_ms: 0,
        last_flush: Instant::now(),
    })
}

fn flush(conn: &Connection, seg: &mut Segment) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE segments SET end = ?1, active_secs = ?2, total_secs = ?3 WHERE id = ?4",
        params![seg.end, seg.active_ms as f64 / 1000.0, seg.total_ms as f64 / 1000.0, seg.id],
    )?;
    seg.last_flush = Instant::now();
    Ok(())
}

fn close(conn: &Connection, seg: &mut Option<Segment>) {
    if let Some(mut s) = seg.take() {
        if let Err(e) = flush(conn, &mut s) {
            db::log(&format!("flush failed: {e}"));
        }
    }
}

pub fn run() {
    if let Err(e) = run_inner() {
        db::log(&format!("collector stopped: {e}"));
    }
}

fn run_inner() -> rusqlite::Result<()> {
    let conn = db::open()?;
    let mut current: Option<Segment> = None;
    let mut last_tick = Instant::now();

    while !QUIT.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(POLL_MS));
        let dt = last_tick.elapsed();
        last_tick = Instant::now();
        let now = unix_now();

        // Machine slept or the process was suspended: don't credit the gap.
        if dt.as_millis() > GAP_MS {
            close(&conn, &mut current);
        }
        let dt_ms = dt.as_millis().min(GAP_MS) as u64;

        if PAUSED.load(Ordering::Relaxed) {
            close(&conn, &mut current);
            continue;
        }
        let Some(sample) = win::foreground().filter(|s| !ignored(s)) else {
            close(&conn, &mut current);
            continue;
        };

        let changed = match &current {
            Some(c) => c.process != sample.process || c.title != sample.title,
            None => true,
        };
        if changed {
            close(&conn, &mut current);
            current = Some(open_segment(&conn, &sample, now)?);
        }

        let seg = current.as_mut().unwrap();
        seg.total_ms += dt_ms;
        if win::idle_ms() < ACTIVE_WINDOW_MS {
            seg.active_ms += dt_ms;
        }
        seg.end = now;
        if seg.last_flush.elapsed() >= FLUSH_EVERY {
            flush(&conn, seg)?;
        }
    }
    close(&conn, &mut current); // flush on quit
    Ok(())
}
