//! GitHub Dark Dimmed theme (from Helix) applied to the whole TUI.
//!
//! Colors are taken from Helix's `github_dark_dimmed` theme, resolved through
//! the `github_dark` capture mappings (syntax = palette variable references).
use ratatui::style::Color;

const fn rgb(hex: u32) -> Color {
    Color::Rgb(
        ((hex >> 16) & 0xff) as u8,
        ((hex >> 8) & 0xff) as u8,
        (hex & 0xff) as u8,
    )
}

// Foreground
pub const FG: Color = rgb(0xadbac7); // fg.default
pub const FG_MUTED: Color = rgb(0x768390); // fg.muted
pub const FG_SUBTLE: Color = rgb(0x636e7b); // fg.subtle

// Backgrounds
pub const BG: Color = rgb(0x22272e); // canvas.default
pub const BG_OVERLAY: Color = rgb(0x2d333b); // canvas.overlay
pub const BG_SUBTLE: Color = rgb(0x2d333b); // canvas.subtle

// Borders
pub const BORDER: Color = rgb(0x444c56); // border.default

// Syntax (github_dark captures, github_dark_dimmed palette)
pub const KEYWORD: Color = rgb(0xf47067); // scale.red.3
pub const STRING: Color = rgb(0x96d0ff); // scale.blue.1
pub const OPERATOR: Color = rgb(0x96d0ff); // scale.blue.1
pub const MEMBER: Color = rgb(0x96d0ff); // scale.blue.1
pub const NUMBER: Color = rgb(0x6cb6ff); // scale.blue.2
pub const CONSTANT: Color = rgb(0x6cb6ff); // scale.blue.2
pub const TYPE: Color = rgb(0xf69d50); // scale.orange.2
pub const FUNCTION: Color = rgb(0xdcbdfb); // scale.purple.2
pub const TAG: Color = rgb(0x8ddb8c); // scale.green.1
pub const COMMENT: Color = rgb(0x768390); // fg.muted

// Difficulty (open/attention/closed)
pub const EASY: Color = rgb(0x57ab5a); // open.fg / scale.green.3
pub const MEDIUM: Color = rgb(0xc69026); // attention.fg / scale.yellow.3
pub const HARD: Color = rgb(0xe5534b); // closed.fg / scale.red.4

// UI accents
pub const ACCENT: Color = rgb(0x4184e4); // accent.muted
pub const SELECTION: Color = rgb(0x143d79); // scale.blue.8
pub const SELECTION_PRIMARY: Color = rgb(0x1b4b91); // scale.blue.7
pub const CURSOR_LINE: Color = rgb(0x2d333b); // canvas.subtle

// Status bar mode backgrounds
pub const NORMAL_BG: Color = rgb(0x4184e4); // accent.muted
pub const INSERT_BG: Color = rgb(0xae7c14); // attention.muted
pub const VISUAL_BG: Color = rgb(0xc96198); // sponsors.muted

// Status bar / menu
pub const STATUSLINE_BG: Color = rgb(0x373e47); // scale.gray.7
pub const POPUP_BG: Color = rgb(0x2d333b); // scale.gray.8
pub const MENU_SELECTED_BG: Color = rgb(0x636e7b); // scale.gray.4
