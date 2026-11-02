# Architecture Overview

This document describes the architecture of the `time-tracker` application.

## High-Level Components

The application is composed of several key components that work together to track and display activity segments.

### 1. Data Layer
- **Persistence**: The application uses [SQLite](https://www.sqlite.org/) via the `rusqlite` crate for persistent storage.
- **Models**: The `tracker/src/models.rs` file defines the core data structures, primarily the `Segment` struct, which represents a period of activity.

### 2. Server Layer
- **HTTP Server**: Built using the `tiny_http` crate.
- **Endpoints**: 
    - `GET /`: Serves the main HTML dashboard.
    - `GET /api/segments`: Returns all activity segments in JSON format.
    - `GET /api/debug`: Provides information about database connectivity.
    - `GET /api/debug/counts`: Returns count queries to verify data integrity.
- **Concurrency**: The server handles requests and interacts with the database to serve real-time data.

### 3. Frontend Layer
- **UI**: A simple HTML/JavaScript interface (found in `tracker/src/index.html`).
- **Interactions**: The frontend fetches data from the server's API endpoints and renders the activity segments to the user.

### 4. Build and Deployment
- **Target**: The application is compiled for `x86_64-pc-windows-gnu`.
- **Build Script**: A shell script (`build.sh`) handles the compilation process, including:
    - Cleaning up existing binaries.
    - Invoking `cargo build --release`.
    - Copying the resulting executable to the target path.
- **Environment**: The build process is designed to run within a WSL environment on Windows.

## Data Flow
1. The **Frontend** requests data from the **Server**.
2. The **Server** queries the **Data Layer** (SQLite) for the requested segments.
3. The **Data Layer** returns the records, which the **Server** serializes into JSON.
4. The **Server** sends the JSON back to the **Frontend** for rendering.
