#!/usr/bin/env bash
# Render demo/demo.gif from *live* `codemmet expand` output, so the demo can
# never drift from real behavior. Self-contained: needs only ffmpeg, a DejaVu
# Sans Mono font, and the codemmet binary. Regenerate with `just demo`.
set -euo pipefail
cd "$(dirname "$0")/.."

BIN="${CODEMMET_BIN:-target/release/codemmet}"
if [ ! -x "$BIN" ]; then
  echo "building release binary…" >&2
  cargo build --release -p codemmet >&2
  BIN="target/release/codemmet"
fi

FONT="${DEMO_FONT:-/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf}"
[ -f "$FONT" ] || { echo "font not found: $FONT (set DEMO_FONT)" >&2; exit 1; }

W=900 H=500 FS=20 LS=12 X=36 Y=26
BG=0x1e1e2e FG=0xcdd6f4
FRAMES=demo/.frames
rm -rf "$FRAMES"; mkdir -p "$FRAMES"
LIST="$FRAMES/list.txt"; : > "$LIST"
n=0 last=""

# frame <seconds> <text>
frame() {
  local png txt
  png=$(printf '%03d.png' "$n")
  txt="$FRAMES/$(printf '%03d.txt' "$n")"
  printf '%s' "$2" > "$txt"
  ffmpeg -y -loglevel error -f lavfi -i "color=c=$BG:s=${W}x${H}" \
    -vf "drawtext=fontfile=${FONT}:textfile=${txt}:fontcolor=${FG}:fontsize=${FS}:x=${X}:y=${Y}:line_spacing=${LS}" \
    -frames:v 1 "$FRAMES/$png"
  printf "file '%s'\nduration %s\n" "$png" "$1" >> "$LIST"
  last="$png"; n=$((n + 1))
}

cmdline() { printf '$ codemmet expand --lang %s "%s"' "$1" "$2"; }
output() { "$BIN" expand --lang "$1" "$2" | expand -t4; }
# A screen showing the prompt+command followed by the live expansion.
session() { printf '%s\n%s' "$(cmdline "$1" "$2")" "$(output "$1" "$2")"; }

# --- storyboard -------------------------------------------------------------
frame 2.2 $'  codemmet\n\n  Emmet-style abbreviations \xe2\x86\x92 real code, for programming\n  languages. Type a terse abbreviation, get structured\n  code with tabstops \xe2\x80\x94 in the CLI, an LSP, or Neovim.'

frame 1.3 "$(cmdline rust 'fn:main>fori10>pr')"
frame 2.6 "$(session rust 'fn:main>fori10>pr')"

frame 1.3 "$(cmdline rust 'fn:handle>match:msg>arm*3^ret')"
frame 2.8 "$(session rust 'fn:handle>match:msg>arm*3^ret')"

frame 1.3 "$(cmdline go 'fn:main>fori:i,10>pr')"
frame 2.6 "$(session go 'fn:main>fori:i,10>pr')"

frame 3.0 $'  Seven languages. Nesting, siblings, repeats, climb-up.\n\n  Learn it interactively:\n\n  $ codemmet train'

# The concat demuxer ignores the final entry's duration unless it recurs.
printf "file '%s'\n" "$last" >> "$LIST"

echo "assembling demo/demo.gif…" >&2
ffmpeg -y -loglevel error -f concat -safe 0 -i "$LIST" \
  -filter_complex "fps=12,scale=${W}:-1:flags=lanczos,split[a][b];[a]palettegen=stats_mode=full[p];[b][p]paletteuse=dither=bayer" \
  -loop 0 demo/demo.gif

rm -rf "$FRAMES"
echo "wrote demo/demo.gif ($(du -h demo/demo.gif | cut -f1))" >&2
