//! Small, embedded stroke icons. No font glyphs or network assets are required.
use std::borrow::Cow;

use gpui::{AssetSource, SharedString};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Icon {
    Folder,
    Import,
    Save,
    Search,
    Undo,
    Redo,
    Settings,
    More,
    Close,
    ChevronDown,
    ChevronRight,
    Check,
    Plus,
    Minus,
    Copy,
    Refresh,
    Help,
    Info,
    Warning,
    Download,
    External,
    Chat,
    Send,
    Stop,
    PanelLeft,
    PanelRight,
    Maximize,
    Minimize,
    Grid,
    Layers,
    Eye,
    Sun,
    Moon,
    Lock,
    Unlock,
    History,
    File,
    Trash,
    Play,
    Sliders,
}

impl Icon {
    pub(crate) const ALL: [Self; 40] = [
        Self::Folder,
        Self::Import,
        Self::Save,
        Self::Search,
        Self::Undo,
        Self::Redo,
        Self::Settings,
        Self::More,
        Self::Close,
        Self::ChevronDown,
        Self::ChevronRight,
        Self::Check,
        Self::Plus,
        Self::Minus,
        Self::Copy,
        Self::Refresh,
        Self::Help,
        Self::Info,
        Self::Warning,
        Self::Download,
        Self::External,
        Self::Chat,
        Self::Send,
        Self::Stop,
        Self::PanelLeft,
        Self::PanelRight,
        Self::Maximize,
        Self::Minimize,
        Self::Grid,
        Self::Layers,
        Self::Eye,
        Self::Sun,
        Self::Moon,
        Self::Lock,
        Self::Unlock,
        Self::History,
        Self::File,
        Self::Trash,
        Self::Play,
        Self::Sliders,
    ];

    pub(crate) fn path(self) -> SharedString {
        format!("icons/{self:?}.svg").into()
    }

    fn geometry(self) -> &'static str {
        match self {
            Self::Folder => r#"<path d="M3 7V5h6l2 2h10v12H3V7Z"/>"#,
            Self::Import => r#"<path d="M12 3v12m-4-4 4 4 4-4M4 15v5h16v-5"/>"#,
            Self::Save => r#"<path d="M4 3h13l3 3v15H4V3Z"/><path d="M8 3v6h8V3M8 21v-8h8v8"/>"#,
            Self::Search => r#"<circle cx="10.5" cy="10.5" r="6.5"/><path d="m16 16 5 5"/>"#,
            Self::Undo => r#"<path d="M8 4 3 9l5 5M3 9h10a7 7 0 0 1 7 7v4"/>"#,
            Self::Redo => r#"<path d="m16 4 5 5-5 5m5-5h-10a7 7 0 0 0-7 7v4"/>"#,
            Self::Settings => {
                r#"<path d="m9 3-1 3-3 1-2 3 2 2-1 3 3 2 2 3h4l2-3 3-1 2-3-2-2 1-3-3-2-2-3H9Z"/><circle cx="11.5" cy="11.5" r="3"/>"#
            }
            Self::More => {
                r#"<circle cx="5" cy="12" r="1"/><circle cx="12" cy="12" r="1"/><circle cx="19" cy="12" r="1"/>"#
            }
            Self::Close => r#"<path d="m6 6 12 12M18 6 6 18"/>"#,
            Self::ChevronDown => r#"<path d="m6 9 6 6 6-6"/>"#,
            Self::ChevronRight => r#"<path d="m9 6 6 6-6 6"/>"#,
            Self::Check => r#"<path d="m5 12 4 4L19 6"/>"#,
            Self::Plus => r#"<path d="M12 5v14M5 12h14"/>"#,
            Self::Minus => r#"<path d="M5 12h14"/>"#,
            Self::Copy => {
                r#"<rect x="8" y="8" width="12" height="13" rx="2"/><path d="M16 8V3H3v13h5"/>"#
            }
            Self::Refresh => r#"<path d="M20 10a8 8 0 1 0-1 7M20 4v6h-6"/>"#,
            Self::Help => {
                r#"<circle cx="12" cy="12" r="9"/><path d="M9 9a3 3 0 0 1 6 0c0 2-3 2-3 5m0 3h.01"/>"#
            }
            Self::Info => r#"<circle cx="12" cy="12" r="9"/><path d="M12 11v6m0-10h.01"/>"#,
            Self::Warning => r#"<path d="m12 3 10 18H2L12 3Zm0 6v5m0 3h.01"/>"#,
            Self::Download => r#"<path d="M12 3v13m-5-5 5 5 5-5M4 19v2h16v-2"/>"#,
            Self::External => r#"<path d="M14 3h7v7m0-7L10 14M10 3H3v18h18v-7"/>"#,
            Self::Chat => r#"<path d="M4 4h16v12H9l-5 5V4Z"/>"#,
            Self::Send => r#"<path d="m3 3 19 9-19 9 4-9-4-9Zm4 9h15"/>"#,
            Self::Stop => r#"<rect x="5" y="5" width="14" height="14" rx="2"/>"#,
            Self::PanelLeft => {
                r#"<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M9 4v16"/>"#
            }
            Self::PanelRight => {
                r#"<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M15 4v16"/>"#
            }
            Self::Maximize => r#"<path d="M8 3H3v5m13-5h5v5M3 16v5h5m13-5v5h-5"/>"#,
            Self::Minimize => r#"<path d="M3 8h5V3m8 0v5h5M8 21v-5H3m13 5v-5h5"/>"#,
            Self::Grid => {
                r#"<rect x="3" y="3" width="18" height="18" rx="2"/><path d="M9 3v18m6-18v18M3 9h18M3 15h18"/>"#
            }
            Self::Layers => {
                r#"<path d="m12 3 10 5-10 5L2 8l10-5ZM2 12l10 5 10-5M2 16l10 5 10-5"/>"#
            }
            Self::Eye => {
                r#"<path d="M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12Z"/><circle cx="12" cy="12" r="3"/>"#
            }
            Self::Sun => {
                r#"<circle cx="12" cy="12" r="4"/><path d="M12 2v2m0 16v2M2 12h2m16 0h2M5 5l1.5 1.5m11 11L19 19M5 19l1.5-1.5m11-11L19 5"/>"#
            }
            Self::Moon => r#"<path d="M20 15A9 9 0 0 1 9 4a9 9 0 1 0 11 11Z"/>"#,
            Self::Lock => {
                r#"<rect x="5" y="10" width="14" height="11" rx="2"/><path d="M8 10V7a4 4 0 0 1 8 0v3m-4 5v2"/>"#
            }
            Self::Unlock => {
                r#"<rect x="5" y="10" width="14" height="11" rx="2"/><path d="M8 10V7a4 4 0 0 1 8 0m-4 8v2"/>"#
            }
            Self::History => r#"<path d="M3 11a9 9 0 1 1 2 7M3 4v7h7m2-4v6l4 2"/>"#,
            Self::File => r#"<path d="M5 3h9l5 5v13H5V3Zm9 0v6h5M8 13h8M8 17h6"/>"#,
            Self::Trash => r#"<path d="M3 6h18M9 6V3h6v3M5 6l1 15h12l1-15M10 10v7m4-7v7"/>"#,
            Self::Play => r#"<path d="m7 3 14 9-14 9V3Z"/>"#,
            Self::Sliders => {
                r#"<path d="M4 6h5m4 0h7M4 12h9m4 0h3M4 18h2m4 0h10M9 3v6m4 0v6M6 15v6"/>"#
            }
        }
    }
}

pub(crate) struct IconAssets;

impl AssetSource for IconAssets {
    fn load(&self, path: &str) -> anyhow::Result<Option<Cow<'static, [u8]>>> {
        Ok(Icon::ALL.into_iter().find(|icon| icon.path().as_ref() == path).map(|icon| {
            Cow::Owned(format!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="black" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">{}</svg>"#,
                icon.geometry(),
            ).into_bytes())
        }))
    }

    fn list(&self, path: &str) -> anyhow::Result<Vec<SharedString>> {
        Ok(Icon::ALL
            .into_iter()
            .map(Icon::path)
            .filter(|p| p.starts_with(path))
            .collect())
    }
}
