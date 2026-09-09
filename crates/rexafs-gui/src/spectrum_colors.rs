//! Palette assignments belong to groups, so selection and draw order cannot recolor them.
use ruviz::render::{Color, ColorMap};
use serde::{Deserialize, Serialize};

use crate::theme::Theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Palette {
    Theme,
    Tab10,
    OkabeIto,
    Viridis,
    Plasma,
    Inferno,
}

impl Palette {
    pub const ALL: [Self; 6] = [
        Self::Theme,
        Self::Tab10,
        Self::OkabeIto,
        Self::Viridis,
        Self::Plasma,
        Self::Inferno,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Theme => "Theme",
            Self::Tab10 => "Tab10",
            Self::OkabeIto => "Okabe–Ito",
            Self::Viridis => "Viridis",
            Self::Plasma => "Plasma",
            Self::Inferno => "Inferno",
        }
    }

    pub fn gradient(self) -> bool {
        matches!(self, Self::Viridis | Self::Plasma | Self::Inferno)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Assignment {
    pub palette: Palette,
    pub index: usize,
    pub count: usize,
    #[serde(default)]
    pub reversed: bool,
}

impl Assignment {
    pub fn color(self, theme: &Theme) -> Color {
        match self.palette {
            Palette::Theme => theme.plot_theme().get_color(self.index % 8),
            Palette::Tab10 => Color::from_palette(self.index),
            Palette::OkabeIto => {
                const COLORS: [Color; 8] = [
                    Color::from_rgb(0, 114, 178),
                    Color::from_rgb(230, 159, 0),
                    Color::from_rgb(0, 158, 115),
                    Color::from_rgb(204, 121, 167),
                    Color::from_rgb(86, 180, 233),
                    Color::from_rgb(213, 94, 0),
                    Color::from_rgb(240, 228, 66),
                    Color::from_rgb(0, 0, 0),
                ];
                if self.index % COLORS.len() == 7 {
                    Color::from_rgb(
                        (theme.text.r * 255.) as u8,
                        (theme.text.g * 255.) as u8,
                        (theme.text.b * 255.) as u8,
                    )
                } else {
                    COLORS[self.index % COLORS.len()]
                }
            }
            palette => {
                let position = if self.count <= 1 {
                    0.5
                } else {
                    self.index.min(self.count - 1) as f64 / (self.count - 1) as f64
                };
                let map = match palette {
                    Palette::Viridis => ColorMap::viridis(),
                    Palette::Plasma => ColorMap::plasma(),
                    _ => ColorMap::inferno(),
                };
                map.sample(if self.reversed {
                    1.0 - position
                } else {
                    position
                })
            }
        }
    }
}

pub fn rgba(color: Color) -> gpui::Rgba {
    gpui::Rgba {
        r: color.r as f32 / 255.,
        g: color.g as f32 / 255.,
        b: color.b as f32 / 255.,
        a: color.a as f32 / 255.,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycles_repeat_but_gradients_span_the_selection_and_reverse() {
        let theme = Theme::light();
        let a = |palette, index, count, reversed| {
            Assignment {
                palette,
                index,
                count,
                reversed,
            }
            .color(&theme)
        };
        assert_eq!(
            a(Palette::Tab10, 0, 20, false),
            a(Palette::Tab10, 10, 20, false)
        );
        assert_eq!(
            a(Palette::OkabeIto, 0, 20, false),
            a(Palette::OkabeIto, 8, 20, false)
        );
        let neutral = Assignment {
            palette: Palette::OkabeIto,
            index: 7,
            count: 8,
            reversed: false,
        };
        assert!(neutral.color(&Theme::dark()).r > neutral.color(&Theme::light()).r);
        for palette in [Palette::Viridis, Palette::Plasma, Palette::Inferno] {
            assert_ne!(a(palette, 0, 12, false), a(palette, 11, 12, false));
            assert_eq!(a(palette, 0, 12, false), a(palette, 11, 12, true));
            assert_eq!(a(palette, 0, 1, false), a(palette, 1, 3, false));
            assert_eq!(a(palette, 0, 0, false), a(palette, 0, 1, false));
        }
    }
}
