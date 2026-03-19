#!/usr/bin/env bash
set -euo pipefail

echo "Building taskflow..."
cargo build --bin taskflow

mkdir -p screenshots/vhs screenshots/svg screenshots/png

VHS=$(go env GOPATH)/bin/vhs

echo "Running VHS..."
"$VHS" screenshots/vhs/demo.tape

echo "Setting GIF loop..."
convert -loop 0 screenshots/vhs/demo.gif screenshots/vhs/demo.gif

echo ""
echo "Done! Outputs:"
echo "  screenshots/vhs/demo.gif    (animated tour)"
echo "  screenshots/png/*.png       (static screenshots)"
