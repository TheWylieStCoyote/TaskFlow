//! Screenshot generator for TaskFlow.
//!
//! Renders every key view into an SVG file using a TestBackend and realistic
//! sample data. Output goes to `screenshots/` at the repo root.
//!
//! Run with:
//!   cargo run --bin screenshot --release

use std::{fs, path::PathBuf};

mod color;
mod render;
mod specs;
mod svg;

fn main() {
    let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("screenshots");
    fs::create_dir_all(&out_dir).unwrap();

    let theme = taskflow::config::Theme::default();

    for spec in specs::all_specs() {
        let mut model = taskflow::app::Model::new().with_sample_data();
        model.current_view = spec.view_id;
        model.terminal_size = (spec.width, spec.height);
        (spec.setup)(&mut model);

        let buffer = render::render_view(&mut model, &theme, spec.width, spec.height);
        let svg = svg::buffer_to_svg(&buffer, spec.filename);
        let path = out_dir.join(spec.filename);
        fs::write(&path, svg).unwrap();
        println!("  \u{2713}  {}", path.display());
    }
}
