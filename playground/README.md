# codemmet playground

Two static pages that run the **real** codemmet engine in the browser via
WebAssembly — no build step at runtime and no server-side anything:

- **`index.html`** — the expansion playground: pick a language, type an
  abbreviation, see it expand live.
- **`train.html`** — the full trainer: guided lessons, drills, spaced-repetition
  review, and a sandbox, driving the same `Trainer` state machine as the CLI.
  The spaced-repetition schedule persists in `localStorage`.

`crates/codemmet-wasm` compiles `codemmet-core` and `codemmet-train` (without
the native `dirs` feature) to `wasm32-unknown-unknown` and exposes a tiny
hand-rolled C-ABI, so no `wasm-bindgen` toolchain is needed. Both pages share
the one `codemmet.wasm`.

## Build the wasm

```sh
just playground        # or: bash playground/build.sh
```

This writes `playground/codemmet.wasm` (a committed copy is kept so the page
works from a fresh checkout; regenerate it whenever the packs or renderer
change).

## Try it locally

`fetch()` won't load the wasm over `file://`, so serve the folder:

```sh
python3 -m http.server -d playground 8000
# open http://localhost:8000
```

## Deploy to GitHub Pages

The [`playground` workflow](../.github/workflows/playground.yml) builds the wasm
and deploys this folder. It is **manual** (`workflow_dispatch`) so it never
fails CI before Pages is set up:

1. Repo → Settings → Pages → Build and deployment → Source: **GitHub Actions**.
2. Actions → **playground** → **Run workflow**.

It then publishes to `https://thewyliestcoyote.github.io/codemmit/`.
