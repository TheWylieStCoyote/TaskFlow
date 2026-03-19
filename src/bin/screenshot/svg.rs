//! Convert a ratatui Buffer into an SVG terminal screenshot.

use ratatui::{buffer::Buffer, style::Modifier};

use crate::color;

const CHAR_W: f64 = 8.4;
const CHAR_H: f64 = 16.0;
const FONT_SIZE: u32 = 13;
const PADDING: f64 = 12.0;
const CHROME_H: f64 = 30.0;

/// Convert `buffer` into a self-contained SVG string styled as a terminal window.
///
/// `title` is displayed in the title bar.
pub fn buffer_to_svg(buffer: &Buffer, title: &str) -> String {
    let cols = f64::from(buffer.area.width);
    let rows = f64::from(buffer.area.height);

    let term_w = cols * CHAR_W;
    let term_h = rows * CHAR_H;
    let svg_w = term_w + PADDING * 2.0;
    let svg_h = term_h + CHROME_H + PADDING;

    let term_x = PADDING;
    let term_y = CHROME_H;

    let cap = (buffer.area.width as usize * buffer.area.height as usize * 40).max(65_536);
    let mut out = String::with_capacity(cap);

    // ── SVG header ────────────────────────────────────────────────────────────
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" \
         width=\"{w:.0}\" height=\"{h:.0}\" \
         viewBox=\"0 0 {w:.0} {h:.0}\">\n",
        w = svg_w,
        h = svg_h,
    ));

    // Monospace font applied to all text elements.
    out.push_str(&format!(
        "<style>text {{ font-family: 'Courier New', Courier, monospace; font-size: {fs}px; }}</style>\n",
        fs = FONT_SIZE,
    ));

    // ── Window chrome ─────────────────────────────────────────────────────────

    // Outer window rect (rounded corners, terminal bg colour).
    push_rect(&mut out, 0.0, 0.0, svg_w, svg_h, 8.0, "#1e1e2e");

    // Title bar — rounded top, square bottom via overlay rect.
    push_rect_rx(&mut out, 0.0, 0.0, svg_w, CHROME_H, 8.0, "#313244");
    push_rect(&mut out, 0.0, CHROME_H - 8.0, svg_w, 8.0, 0.0, "#313244");

    // Traffic-light circles.
    let dot_y = CHROME_H / 2.0;
    push_circle(&mut out, 20.0, dot_y, 6.0, "#f38ba8");
    push_circle(&mut out, 38.0, dot_y, 6.0, "#f9e2af");
    push_circle(&mut out, 56.0, dot_y, 6.0, "#a6e3a1");

    // Title text (centred in chrome bar).
    out.push_str(&format!(
        "<text x=\"{x:.1}\" y=\"{y:.1}\" text-anchor=\"middle\" \
         fill=\"#cdd6f4\" font-size=\"{fs}px\">{title}</text>\n",
        x = svg_w / 2.0,
        y = dot_y + 5.0,
        fs = FONT_SIZE,
        title = xml_escape(title),
    ));

    // ── Terminal background ───────────────────────────────────────────────────
    push_rect(
        &mut out,
        term_x,
        term_y,
        term_w,
        term_h,
        0.0,
        color::DEFAULT_BG,
    );

    // ── Background pass: non-default bg cells ─────────────────────────────────
    for row in 0..buffer.area.height {
        let mut col = 0u16;
        while col < buffer.area.width {
            let Some(cell) = buffer.cell((col, row)) else {
                col += 1;
                continue;
            };
            let bg = color::to_hex(cell.bg, false);
            if bg == color::DEFAULT_BG {
                col += 1;
                continue;
            }
            // Extend run of identical bg.
            let start = col;
            col += 1;
            while col < buffer.area.width {
                let run_bg = buffer.cell((col, row)).map_or_else(
                    || color::DEFAULT_BG.to_string(),
                    |c| color::to_hex(c.bg, false),
                );
                if run_bg == bg {
                    col += 1;
                } else {
                    break;
                }
            }
            let x = term_x + f64::from(start) * CHAR_W;
            let y = term_y + f64::from(row) * CHAR_H;
            let w = f64::from(col - start) * CHAR_W;
            push_rect(&mut out, x, y, w, CHAR_H, 0.0, &bg);
        }
    }

    // ── Text pass: group cells with identical (fg, modifier) into runs ────────
    for row in 0..buffer.area.height {
        // Baseline: ~78% down the cell height works well for most fonts.
        let baseline = term_y + f64::from(row) * CHAR_H + CHAR_H * 0.78;

        let mut col = 0u16;
        while col < buffer.area.width {
            let Some(cell) = buffer.cell((col, row)) else {
                col += 1;
                continue;
            };

            // Wide-char placeholders have an empty symbol — skip them.
            if cell.symbol().is_empty() {
                col += 1;
                continue;
            }

            let fg = color::to_hex(cell.fg, true);
            let bold = cell.modifier.contains(Modifier::BOLD);
            let italic = cell.modifier.contains(Modifier::ITALIC);
            let underline = cell.modifier.contains(Modifier::UNDERLINED);

            let start = col;
            let mut text = String::new();
            text.push_str(&xml_escape(cell.symbol()));
            col += 1;

            // Extend run while style is identical.
            while col < buffer.area.width {
                let Some(next) = buffer.cell((col, row)) else {
                    break;
                };
                if next.symbol().is_empty() {
                    // Wide-char placeholder — skip without adding to text.
                    col += 1;
                    continue;
                }
                if color::to_hex(next.fg, true) != fg
                    || next.modifier.contains(Modifier::BOLD) != bold
                    || next.modifier.contains(Modifier::ITALIC) != italic
                    || next.modifier.contains(Modifier::UNDERLINED) != underline
                {
                    break;
                }
                text.push_str(&xml_escape(next.symbol()));
                col += 1;
            }

            // Skip purely whitespace runs — they're invisible.
            if text.chars().all(char::is_whitespace) {
                continue;
            }

            let x = term_x + f64::from(start) * CHAR_W;
            let run_chars = f64::from(col - start);
            let text_len = run_chars * CHAR_W;

            let font_weight = if bold { " font-weight=\"bold\"" } else { "" };
            let font_style = if italic { " font-style=\"italic\"" } else { "" };
            let text_deco = if underline {
                " text-decoration=\"underline\""
            } else {
                ""
            };

            out.push_str(&format!(
                "<text x=\"{x:.1}\" y=\"{y:.1}\" fill=\"{fg}\" \
                 textLength=\"{tl:.1}\" lengthAdjust=\"spacingAndGlyphs\" \
                 xml:space=\"preserve\"{fw}{fs}{td}>{text}</text>\n",
                y = baseline,
                tl = text_len,
                fw = font_weight,
                fs = font_style,
                td = text_deco,
            ));
        }
    }

    out.push_str("</svg>\n");
    out
}

// ── SVG primitive helpers ─────────────────────────────────────────────────────

fn push_rect(out: &mut String, x: f64, y: f64, w: f64, h: f64, rx: f64, fill: &str) {
    if rx > 0.0 {
        push_rect_rx(out, x, y, w, h, rx, fill);
    } else {
        out.push_str(&format!(
            "<rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"{w:.1}\" height=\"{h:.1}\" fill=\"{fill}\"/>\n",
        ));
    }
}

fn push_rect_rx(out: &mut String, x: f64, y: f64, w: f64, h: f64, rx: f64, fill: &str) {
    out.push_str(&format!(
        "<rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"{w:.1}\" height=\"{h:.1}\" rx=\"{rx:.1}\" fill=\"{fill}\"/>\n",
    ));
}

fn push_circle(out: &mut String, cx: f64, cy: f64, r: f64, fill: &str) {
    out.push_str(&format!(
        "<circle cx=\"{cx:.1}\" cy=\"{cy:.1}\" r=\"{r:.1}\" fill=\"{fill}\"/>\n",
    ));
}

fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
    out
}
