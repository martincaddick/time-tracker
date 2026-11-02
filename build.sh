#!/bin/bash
set -e

# Paths
TARGET_EXE="/mnt/c/ai/time-tracker/tracker.exe"
TRACKER_DIR="$(dirname "$0")/tracker"

echo "Cleaning up old tracker instance..."

# Kill the process if it is running
# We use cmd.exe to call taskkill, which is the standard way to handle Windows processes from WSL.
# /F forces the termination, /IM specifies the image name, and /T kills child processes.
# We redirect output to null to ignore errors if the process isn't running.
cmd.exe /c "taskkill /F /IM tracker.exe /T >nul 2>&1" || true

# Remove the old executable if it exists
if [ -f "$TARGET_EXE" ]; then
    rm -f "$TARGET_EXE"
fi

echo "Building tracker..."
cd "$TRACKER_DIR"

cargo build \
  --release \
  --target x86_64-pc-windows-gnu \
  --target-dir ~/tracker-target

cp \
  ~/tracker-target/x86_64-pc-windows-gnu/release/tracker.exe \
  "$TARGET_EXE"

echo "Build complete"
echo "Output: $TARGET_EXE"
