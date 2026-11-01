#!/bin/bash

set -e

echo "Building tracker..."

cd "$(dirname "$0")/tracker"

cargo build \
  --release \
  --target x86_64-pc-windows-gnu \
  --target-dir ~/tracker-target

cp \
  ~/tracker-target/x86_64-pc-windows-gnu/release/tracker.exe \
  /mnt/c/ai/time-tracker/tracker.exe

echo "Build complete"
echo "Output: C:\\ai\\time-tracker\\tracker.exe"
