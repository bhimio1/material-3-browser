//! The setup / settings window.

pub mod pages;

pub use material_3_ui::toasts::ToastKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Overview,
    Engine,
    Extension,
    Import,
    Appearance,
    About,
}

impl Page {
    pub const ALL: [Page; 6] = [Page::Overview, Page::Engine, Page::Extension, Page::Import, Page::Appearance, Page::About];

    pub fn title(self) -> &'static str {
        match self {
            Page::Overview => "Overview",
            Page::Engine => "Browser engine",
            Page::Extension => "Extension & signing",
            Page::Import => "Import",
            Page::Appearance => "Appearance",
            Page::About => "About",
        }
    }

    pub fn icon(self) -> material_3_ui::icons::MaterialIcon {
        use material_3_ui::icons::icons::*;
        match self {
            Page::Overview => ICON_DASHBOARD,
            Page::Engine => ICON_PUBLIC,
            Page::Extension => ICON_EXTENSION,
            Page::Import => ICON_DOWNLOAD,
            Page::Appearance => ICON_PALETTE,
            Page::About => ICON_INFO,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Action {
    Toast(String, ToastKind),
    /// Config changed: save, re-plan, re-theme.
    ConfigChanged,
    /// Prepare the profile and start the browser.
    Launch,
}
