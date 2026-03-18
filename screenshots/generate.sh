#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

echo "Building..."
cargo build --bin taskflow --bin screenshot

mkdir -p screenshots/svg screenshots/png screenshots/vhs

echo ""
echo "Generating SVGs..."
cargo run --bin screenshot
mv screenshots/*.svg screenshots/svg/

echo ""
echo "Generating PNGs + GIF via VHS..."
VHS=$(go env GOPATH)/bin/vhs
"$VHS" screenshots/vhs/demo.tape

echo ""
echo "Done! Outputs:"
echo "  screenshots/svg/*.svg       (static SVG screenshots)"
echo "  screenshots/png/*.png       (static PNG screenshots)"
echo "  screenshots/vhs/demo.gif    (animated tour)"
