# Time Tracker

A Windows system tray time tracker that automatically records your active window usage. Similar to "Clockify Autotracker," it monitors which applications you are using and calculates how much time is spent on each.

## Features

- **Automatic Tracking**: Automatically detects the foreground window and starts/stops tracking segments.
- **Active Time Detection**: Uses system input monitoring to determine if you are actively interacting with a window or if it's just sitting in the foreground.
- **SQLite Storage**: Saves all time tracking data into a local SQLite database.
- **System Tray Integration**: Runs as a lightweight background process in your system tray.
- **Smart Filtering**: Automatically ignores common UI elements (like the task switcher or snipping tool) and the tracker itself.

## How it Works

The project is built with Rust and uses the following components:

- **Collector**: Periodically polls the foreground window using the Windows API. It handles "active" vs "idle" state based on keyboard/mouse input.
- **Database**: Manages persistence of segments (process name, window title, start/end times, and active seconds).
- **Server**: Provides a web UI to view and manage your tracked time.
- **Tray**: Handles the system tray icon and provides basic controls (Pause, Quit).

## Getting Started

### Prerequisites
- Windows OS
- Rust and Cargo installed

### Installation
1. Clone the repository.
2. Build the project:
   ```bash
   cargo build --release
   ```
3. Run the tracker:
   ```bash
   cargo run --release
   ```

## Technology Stack
- **Language**: Rust
- **Database**: SQLite (`rusqlite`)
- **Windows APIs**: Used for window management and input monitoring.
