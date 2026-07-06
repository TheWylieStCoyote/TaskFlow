# Demo

`demo.gif` (shown at the top of the [root README](../README.md)) is generated
from **live** `codemmet expand` output by [`render.sh`](render.sh), so it can
never drift from real behavior — regenerate it whenever the packs or rendering
change.

## Regenerate

```sh
just demo
# or directly:
bash demo/render.sh
```

Requirements: `ffmpeg` and a DejaVu Sans Mono font (both preinstalled on most
Linux dev boxes). The script builds `target/release/codemmet` if it is missing.

Overrides via environment variables:

- `CODEMMET_BIN` — path to the binary (default `target/release/codemmet`).
- `DEMO_FONT` — path to a monospace `.ttf` (default DejaVu Sans Mono).

## Editing the storyboard

The frames and their durations are the `frame`/`session` calls near the bottom
of `render.sh`. Each `session <lang> <abbr>` screen shows the command and then
runs the binary to capture the real expansion, so you only ever edit the
abbreviations — the output stays authoritative.

## A higher-fidelity alternative

For a recording with real typing animation, [VHS](https://github.com/charmbracelet/vhs)
renders a `.tape` script to a GIF. It is not required here (the ffmpeg script is
self-contained and dependency-light), but it is a good option if you want the
keystroke-by-keystroke look.
