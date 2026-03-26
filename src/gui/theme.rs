use iced::Theme;

/// Application theme.
pub fn app_theme() -> Theme {
    Theme::TokyoNight
}

/// Spacing constants for consistent layout.
pub mod spacing {
    pub const TINY: f32 = 4.0;
    pub const SMALL: f32 = 8.0;
    pub const MEDIUM: f32 = 12.0;
    pub const LARGE: f32 = 16.0;
    pub const XLARGE: f32 = 24.0;
    pub const SECTION: f32 = 32.0;
}

/// Font sizes.
pub mod font_size {
    pub const SMALL: f32 = 12.0;
    pub const NORMAL: f32 = 14.0;
    pub const LARGE: f32 = 18.0;
    pub const TITLE: f32 = 24.0;
}
