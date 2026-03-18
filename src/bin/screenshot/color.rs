//! Color conversion: ratatui Color → CSS hex string.

use ratatui::style::Color;

/// Default terminal foreground (Catppuccin Mocha text).
const DEFAULT_FG: &str = "#cdd6f4";
/// Default terminal background (Catppuccin Mocha base).
pub const DEFAULT_BG: &str = "#1e1e2e";

/// Convert a ratatui `Color` to a CSS hex string.
///
/// `is_fg` selects which default to use for `Color::Reset`.
pub fn to_hex(color: Color, is_fg: bool) -> String {
    match color {
        Color::Reset => {
            if is_fg {
                DEFAULT_FG.to_string()
            } else {
                DEFAULT_BG.to_string()
            }
        }
        Color::Black => "#45475a".to_string(),
        Color::Red => "#f38ba8".to_string(),
        Color::Green => "#a6e3a1".to_string(),
        Color::Yellow => "#f9e2af".to_string(),
        Color::Blue => "#89b4fa".to_string(),
        Color::Magenta => "#f5c2e7".to_string(),
        Color::Cyan => "#94e2d5".to_string(),
        Color::Gray => "#bac2de".to_string(),
        Color::DarkGray => "#585b70".to_string(),
        Color::LightRed => "#f38ba8".to_string(),
        Color::LightGreen => "#a6e3a1".to_string(),
        Color::LightYellow => "#f9e2af".to_string(),
        Color::LightBlue => "#89b4fa".to_string(),
        Color::LightMagenta => "#f5c2e7".to_string(),
        Color::LightCyan => "#94e2d5".to_string(),
        Color::White => "#cdd6f4".to_string(),
        Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
        Color::Indexed(n) => indexed_to_hex(n),
    }
}

fn indexed_to_hex(n: u8) -> String {
    // ANSI 16 colors — Catppuccin Mocha palette
    const ANSI16: [&str; 16] = [
        "#45475a", "#f38ba8", "#a6e3a1", "#f9e2af", "#89b4fa", "#f5c2e7", "#94e2d5", "#bac2de",
        "#585b70", "#f38ba8", "#a6e3a1", "#f9e2af", "#89b4fa", "#f5c2e7", "#94e2d5", "#a6adc8",
    ];
    if (n as usize) < ANSI16.len() {
        return ANSI16[n as usize].to_string();
    }

    if n >= 232 {
        // Grayscale ramp: 232 → #080808, 255 → #eeeeee
        let v = 8u8.saturating_add(10 * (n - 232));
        return format!("#{v:02x}{v:02x}{v:02x}");
    }

    // 6×6×6 RGB cube (indices 16–231)
    let idx = n - 16;
    let b = idx % 6;
    let g = (idx / 6) % 6;
    let r = idx / 36;
    let c = |v: u8| -> u8 {
        if v == 0 {
            0
        } else {
            55 + 40 * v
        }
    };
    format!("#{:02x}{:02x}{:02x}", c(r), c(g), c(b))
}
