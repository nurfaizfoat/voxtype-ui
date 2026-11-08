use std::path::PathBuf;

use rune::{Color, TextStyle};
use rune_kit::{Elevation, Kit, PaletteSeed, Radius, Type};

// Strata's default stack, with Omarchy's fontconfig monospace as a fallback.
pub const FONT_FAMILIES: (&str, &str) = (
    "JetBrains Mono, JetBrainsMono Nerd Font, ui-monospace",
    "JetBrains Mono, JetBrainsMono Nerd Font, ui-monospace",
);

/// Fixed logical-point roles for a compact desktop editor. Keep the Strata
/// family, but distinguish reading copy from controls and technical metadata.
pub mod typography {
    use rune::TextStyle;
    use rune_kit::Kit;

    pub const PAGE_TITLE: TextStyle = TextStyle::new(22.0, 600.0).lh(28.0);
    pub const SECTION_TITLE: TextStyle = TextStyle::new(16.0, 600.0).lh(24.0);
    pub const FIELD_LABEL: TextStyle = TextStyle::new(14.0, 500.0).lh(20.0);
    pub const BODY: TextStyle = TextStyle::new(13.0, 400.0).lh(20.0);
    pub const CONTROL: TextStyle = TextStyle::new(13.0, 400.0).lh(18.0);
    pub const ACTION: TextStyle = TextStyle::new(13.0, 500.0).lh(18.0);
    pub const METADATA: TextStyle = TextStyle::new(12.0, 400.0).lh(18.0);
    pub const VALUE: TextStyle = TextStyle::new(14.0, 400.0).lh(20.0);

    // About 72 JetBrains Mono characters at the body size. Field descriptions
    // can narrow with the window; prose must not stretch across wide panels.
    pub const PROSE_WIDTH: f32 = 560.0;

    pub fn scale(style: TextStyle, ratio: f32) -> TextStyle {
        TextStyle {
            size: style.size * ratio,
            line_height: style.line_px() * ratio,
            tracking: style.tracking * ratio,
            ..style
        }
    }

    /// Explicit UI roles use the same desktop scale as the kit's form controls.
    pub fn for_kit(k: &Kit, style: TextStyle) -> TextStyle {
        scale(style, k.ty.ui.size / CONTROL.size)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    pub background: Color,
    pub foreground: Color,
    pub accent: Color,
    pub dark_background: Option<Color>,
    pub lighter_background: Option<Color>,
    pub muted: Option<Color>,
}

fn hex(value: &str) -> Option<Color> {
    // Guard Rune's short-hex parser against non-ASCII input from an external file.
    value
        .is_ascii()
        .then(|| rune::color::hex(value))
        .flatten()
        .filter(|c| c.a == 1.0)
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            background: hex("#111512").unwrap(),
            foreground: hex("#e6e9e5").unwrap(),
            accent: hex("#FFD60A").unwrap(),
            dark_background: None,
            lighter_background: None,
            muted: None,
        }
    }
}

impl Palette {
    pub fn kit_at_scale(self, ratio: f32) -> Kit {
        let mut kit = self.kit();
        for font in [
            &mut kit.ty.ui,
            &mut kit.ty.ui_medium,
            &mut kit.ty.ui_compact,
            &mut kit.ty.small,
            &mut kit.ty.small_medium,
            &mut kit.ty.body,
            &mut kit.ty.body_medium,
            &mut kit.ty.text_small,
            &mut kit.ty.text_small_medium,
            &mut kit.ty.text_xsmall,
            &mut kit.ty.text_xsmall_medium,
            &mut kit.ty.mono,
        ] {
            *font = typography::scale(*font, ratio);
        }
        kit
    }

    pub fn kit(self) -> Kit {
        let mut kit = Kit::from_palette(PaletteSeed {
            bg: self.background,
            fg: self.foreground,
            accent: self.accent,
        });
        use typography as ty;
        kit.ty = Type {
            ui: ty::CONTROL,
            ui_medium: ty::ACTION,
            ui_compact: ty::CONTROL,
            small: ty::METADATA,
            small_medium: TextStyle::new(12.0, 500.0).lh(18.0),
            body: ty::BODY,
            body_medium: TextStyle::new(13.0, 500.0).lh(20.0),
            text_small: ty::BODY,
            text_small_medium: TextStyle::new(13.0, 500.0).lh(20.0),
            text_xsmall: ty::METADATA,
            text_xsmall_medium: TextStyle::new(12.0, 500.0).lh(18.0),
            mono: ty::CONTROL.mono(),
        };
        kit.radius = Radius {
            xs: 3.0,
            sm: 4.0,
            md: 5.0,
            lg: 6.0,
            xl: 10.0,
        };
        kit.elevation = Elevation::default();
        kit.c.border_focus = self.accent;
        kit.c.bg_deep = self.dark_background.unwrap_or(self.background);
        // Strata mixes a little of the lighter token into its background for
        // settings groups; it does not paint whole cards with that lighter token.
        {
            let muted = self
                .lighter_background
                .unwrap_or_else(|| blend(self.background, self.foreground, 0.10));
            let surface = blend(self.background, muted, 0.25);
            let raised = blend(self.background, muted, 0.5);
            kit.c.layer_01 = surface;
            kit.c.layer_02 = surface;
            kit.c.layer_03 = raised;
            kit.c.layer_04 = muted;
            kit.c.bg_button_neutral = surface;
            kit.c.surface_base = surface;
            kit.c.surface_weak = surface;
            kit.c.surface_raised = surface;
            kit.c.surface_raised_base = surface;
            kit.c.surface_raised_hover = raised;
            kit.c.surface_disabled = surface;
            kit.c.surface_float = surface;
        }
        let border = self
            .muted
            .unwrap_or_else(|| blend(self.background, self.foreground, 0.36));
        kit.c.border_base = border.alpha(0.55);
        kit.c.border_muted = border.alpha(0.35);
        kit.c.border_strong = border.alpha(0.8);
        kit.c.text_muted = blend(self.background, self.foreground, 0.62);
        kit.c.text_faint = kit.c.text_muted;
        kit.c.icon_muted = kit.c.text_muted;
        kit.c.hover = self.accent.alpha(0.15);
        kit.c.pressed = self.accent.alpha(0.22);
        kit.c.bg_icon_button_contrast = self.accent.alpha(0.10);
        kit.c.submit_fg = self.accent;
        kit.c.submit_icon = self.accent;
        kit
    }
}

// Match GTK/CSS color mixing in encoded sRGB, not Rune's linear-light mix.
fn blend(a: Color, b: Color, t: f32) -> Color {
    let channel = |a, b| {
        rune::color::srgb_decode(
            rune::color::srgb_encode(a) * (1.0 - t) + rune::color::srgb_encode(b) * t,
        )
    };
    Color {
        r: channel(a.r, b.r),
        g: channel(a.g, b.g),
        b: channel(a.b, b.b),
        a: 1.0,
    }
}

// Omarchy's palette is a flat file of quoted hex tokens. This intentionally reads
// only those tokens, not the user's Voxtype TOML or a general configuration file.
pub fn parse_palette(source: &str) -> Result<Palette, &'static str> {
    let mut palette = [None; 6];
    for line in source.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            break;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let index = match key.trim() {
            "background" => 0,
            "foreground" => 1,
            "accent" => 2,
            "dark_background" => 3,
            "lighter_background" => 4,
            "muted" => 5,
            _ => continue,
        };
        let value = value.trim();
        let quote = value.chars().next().ok_or("Incomplete theme palette")?;
        if quote != '\'' && quote != '"' {
            return Err("Invalid theme color");
        }
        let tail = value.get(1..).ok_or("Invalid theme color")?;
        let end = tail.find(quote).ok_or("Invalid theme color")?;
        let rest = tail[end + 1..].trim();
        if !rest.is_empty() && !rest.starts_with('#') {
            return Err("Invalid theme color");
        }
        let value = &tail[..end];
        palette[index] = Some(hex(value).ok_or("Invalid theme color")?);
    }
    Ok(Palette {
        background: palette[0].ok_or("Incomplete theme palette")?,
        foreground: palette[1].ok_or("Incomplete theme palette")?,
        accent: palette[2].ok_or("Incomplete theme palette")?,
        dark_background: palette[3],
        lighter_background: palette[4],
        muted: palette[5],
    })
}

pub struct Theme {
    pub palette: Palette,
    pub path: Option<PathBuf>,
    pub loaded: bool,
    pub issue: Option<&'static str>,
    pub text_scale: crate::text_scale::DesktopTextScale,
}

impl Theme {
    pub fn preview() -> Self {
        Self {
            palette: Palette::default(),
            path: None,
            loaded: false,
            issue: None,
            text_scale: crate::text_scale::DesktopTextScale::fixed(1.0),
        }
    }

    pub fn from_path(path: PathBuf) -> Self {
        let mut theme = Self {
            path: Some(path),
            ..Self::preview()
        };
        theme.refresh();
        theme
    }

    pub fn for_app() -> Self {
        let mut theme = Self::for_palette();
        theme.text_scale = crate::text_scale::DesktopTextScale::for_app();
        theme
    }

    fn for_palette() -> Self {
        if let Some(path) = std::env::var_os("VOXTYPE_SETTINGS_THEME_FILE") {
            return Self::from_path(path.into());
        }
        // Headless fixtures are deterministic and never depend on personal data.
        if rune::is_headless() {
            return Self::preview();
        }
        let state = std::env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state"))
            });
        state.map_or_else(Self::preview, |dir| {
            Self::from_path(dir.join("omarchy/current/theme/colors.toml"))
        })
    }

    pub fn kit(&self) -> Kit {
        self.palette.kit_at_scale(self.text_scale.ratio())
    }

    pub fn refresh(&mut self) -> bool {
        let Some(path) = &self.path else {
            return false;
        };
        let before = (self.palette, self.loaded, self.issue);
        match std::fs::read_to_string(path)
            .map_err(|_| "Theme file unavailable")
            .and_then(|text| parse_palette(&text))
        {
            Ok(palette) => {
                self.palette = palette;
                self.loaded = true;
                self.issue = None;
            }
            Err(issue) => {
                // Keep the last valid colors during an atomic theme/symlink swap.
                self.issue = Some(issue);
            }
        }
        before != (self.palette, self.loaded, self.issue)
    }

    pub fn label(&self) -> &'static str {
        match (self.path.is_some(), self.loaded, self.issue.is_some()) {
            (_, true, true) => "Theme reconnecting",
            (true, false, _) => "Theme unavailable",
            (_, true, false) => "Omarchy theme",
            _ => "Preview palette",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);

    fn palette(accent: &str) -> String {
        format!(
            "# palette\nbackground = '#111512'\nforeground = \"#e6e9e5\"\naccent = \"{accent}\" # comment\n"
        )
    }

    #[test]
    fn reads_quoted_opaque_palette_and_ignores_unknown_tokens() {
        let parsed = parse_palette(&palette("#6e9cf7")).unwrap();
        assert_eq!(parsed.accent.hex(), "#6e9cf7ff");
        for bad in ["#gggggg", "#12345600", "#ééé", "blue"] {
            assert!(parse_palette(&palette(bad)).is_err());
        }
        assert!(parse_palette("accent = '#fff'").is_err());
        assert!(
            parse_palette("background = '#111512\nforeground = '#e6e9e5'\naccent = '#fff'")
                .is_err()
        );
    }

    #[test]
    fn strata_surfaces_and_controls_use_theme_tokens_without_native_kit_sheen() {
        let source = include_str!("../fixtures/forest.toml");
        let palette = parse_palette(source).unwrap();
        let kit = palette.kit();
        assert_eq!(kit.c.bg_deep.hex(), "#000700ff");
        assert_eq!(kit.c.layer_01.hex(), "#070f07ff");
        assert_eq!(kit.c.border_base, palette.muted.unwrap().alpha(0.55));
        assert_eq!(kit.c.submit_fg, palette.accent);
        assert_eq!(kit.ty.ui.size, 13.0);
        assert_eq!(kit.ty.small.size, 12.0);
        assert!(kit.elevation.elements.is_empty());
        assert!(
            parse_palette(&format!(
                "{}\nlighter_background='invalid'",
                self::palette("#fff")
            ))
            .is_err()
        );
    }

    #[test]
    fn type_roles_keep_headings_distinct_and_reading_copy_open() {
        use typography as ty;
        let kit = Palette::default().kit();
        const {
            assert!(ty::PAGE_TITLE.size > ty::SECTION_TITLE.size);
            assert!(ty::SECTION_TITLE.size > ty::FIELD_LABEL.size);
            assert!(ty::FIELD_LABEL.size > ty::BODY.size);
            assert!(ty::BODY.size > ty::METADATA.size);
        }
        assert!(ty::BODY.line_px() >= ty::BODY.size * 1.5);
        assert_eq!(kit.ty.body, ty::BODY);
        assert_eq!(kit.ty.ui_compact, ty::CONTROL);
        assert_eq!(kit.ty.ui_medium, ty::ACTION);
        assert_eq!(kit.ty.small, ty::METADATA);
    }

    #[test]
    fn desktop_scale_reaches_both_explicit_roles_and_kit_controls_once() {
        use typography as ty;
        let ratio = crate::text_scale::DesktopTextScale::fixed(0.8182).ratio();
        let kit = Palette::default().kit_at_scale(ratio);
        for font in [kit.ty.ui, kit.ty.ui_compact, kit.ty.body, kit.ty.mono] {
            assert!((font.size - 11.0).abs() < 0.001);
        }
        for role in [
            ty::PAGE_TITLE,
            ty::SECTION_TITLE,
            ty::FIELD_LABEL,
            ty::METADATA,
            ty::VALUE,
        ] {
            let scaled = ty::for_kit(&kit, role);
            assert!((scaled.size / role.size - ratio).abs() < 0.001);
            assert!((scaled.line_px() / role.line_px() - ratio).abs() < 0.001);
            assert_eq!(scaled.weight, role.weight);
        }
    }

    #[test]
    fn reading_and_metadata_ink_meet_aa_on_both_panel_surfaces() {
        let luminance = |c: Color| 0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b;
        for palette in [
            Palette::default(),
            parse_palette(include_str!("../fixtures/forest.toml")).unwrap(),
            parse_palette("background='#f7f7f7'\nforeground='#171717'\naccent='#0044bb'").unwrap(),
        ] {
            let kit = palette.kit();
            for ink in [kit.c.text_base, kit.c.text_muted, kit.c.text_faint] {
                for surface in [kit.c.bg_base, kit.c.layer_01, kit.c.bg_deep] {
                    let a = luminance(ink);
                    let b = luminance(surface);
                    let contrast = (a.max(b) + 0.05) / (a.min(b) + 0.05);
                    assert!(
                        contrast >= 4.5,
                        "{} on {}: {contrast}",
                        ink.hex(),
                        surface.hex()
                    );
                }
            }
        }
    }

    #[test]
    fn reloads_replaced_file_and_retains_palette_across_missing_file() {
        let dir = std::env::temp_dir().join(format!(
            "voxtype-theme-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("colors.toml");
        std::fs::write(&path, palette("#6e9cf7")).unwrap();
        let mut theme = Theme::from_path(path.clone());
        assert!(theme.loaded);
        let old = theme.palette;
        std::fs::remove_file(&path).unwrap();
        assert!(theme.refresh());
        assert_eq!(theme.palette, old);
        assert!(theme.issue.is_some());
        let replacement = dir.join("replacement");
        std::fs::write(&replacement, palette("#ff6688")).unwrap();
        std::fs::rename(replacement, &path).unwrap();
        assert!(theme.refresh());
        assert_eq!(theme.palette.accent.hex(), "#ff6688ff");
        assert!(theme.issue.is_none());
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn follows_replaced_theme_symlink_without_writing_it() {
        let dir = std::env::temp_dir().join(format!(
            "voxtype-theme-link-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let first = dir.join("first.toml");
        let second = dir.join("second.toml");
        let link = dir.join("current.toml");
        std::fs::write(&first, palette("#6e9cf7")).unwrap();
        std::fs::write(&second, palette("#ff6688")).unwrap();
        std::os::unix::fs::symlink(&first, &link).unwrap();
        let mut theme = Theme::from_path(link.clone());
        let next = dir.join("next");
        std::os::unix::fs::symlink(&second, &next).unwrap();
        std::fs::rename(next, &link).unwrap();
        assert!(theme.refresh());
        assert_eq!(theme.palette.accent.hex(), "#ff6688ff");
        for path in [link, first, second] {
            std::fs::remove_file(path).unwrap();
        }
        std::fs::remove_dir(dir).unwrap();
    }
}
