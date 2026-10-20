//! Pages of the setup / settings window.

use crate::app::App;
use crate::config::{ColorScheme, EngineChoice, ThemeSource};
use crate::profile::ExtensionMode;
use crate::sys::paths;
use crate::theme::palette::{parse_hex, to_hex};
use crate::ui::common::{self, card, row};
use crate::ui::widgets::{self, segmented, switch, ButtonKind};
use crate::ui::{Action, Page, ToastKind};
use crate::{engine, ublock};
use egui::{vec2, Color32, RichText, Ui};
use egui_material_icons::icons::*;

const SEED_PRESETS: &[&str] = &["#4285F4", "#6750A4", "#B3261E", "#E8710A", "#1E8E3E", "#00897B", "#C2185B", "#795548", "#607D8B"];

pub fn show(ui: &mut Ui, app: &mut App) {
    match app.page {
        Page::Overview => overview(ui, app),
        Page::Engine => engine_page(ui, app),
        Page::Extension => extension(ui, app),
        Page::Import => import(ui, app),
        Page::Appearance => appearance(ui, app),
        Page::About => about(ui, app),
    }
}

fn mode_text(mode: &ExtensionMode) -> (&'static str, String, bool) {
    match mode {
        ExtensionMode::Sideloaded { signed: true } => ("Installed", "Signed by Mozilla, installed in the profile. Website theming, the new tab page and the palette are active.".into(), true),
        ExtensionMode::Sideloaded { signed: false } => ("Installed (unsigned)", "This engine allows unsigned add-ons, so the extension is installed permanently.".into(), true),
        ExtensionMode::DevLoad(_) => ("Development loading", "Loaded temporarily at each start through a private debugging socket (see Extension & signing).".into(), true),
        ExtensionMode::NeedsSigning => ("Needs a signed build", "This engine only runs signed add-ons. Until a signed release exists, turn on development loading or use LibreWolf.".into(), false),
        ExtensionMode::Missing => ("Not found", "The built extension wasn't found next to m3-browser. Reinstall the package.".into(), false),
    }
}

fn overview(ui: &mut Ui, app: &mut App) {
    let p = app.palette.clone();
    let setup = app.setup;
    let subtitle = if setup {
        "Welcome! Check the setup below, then open the browser. You can come back here any time with m3-browser --settings."
    } else {
        "Your Material 3 browser: Zen (or Firefox) with your own profile, palette and ad blocking."
    };
    let mut actions = Vec::new();
    common::page(ui, ICON_PUBLIC, if setup { "Set up Material 3 Browser" } else { "Material 3 Browser" }, subtitle, |ui| {
        let theme_file = app.theme_file();
        match app.plan.clone() {
            Err(e) => common::banner(ui, ICON_ERROR, &e, true, |_| {}),
            Ok(plan) => {
                card(ui, "Status", |ui| {
                    row(ui, "Browser engine", &format!("{}{}", plan.engine.label, if plan.engine.requires_signing { " (runs signed add-ons only)" } else { "" }), |ui| {
                        if widgets::button(ui, ButtonKind::Text, None, "Change").clicked() {
                            app.page = Page::Engine;
                        }
                    });
                    ui.separator();
                    let (title, desc, ok) = mode_text(&plan.mode);
                    row(ui, &format!("Extension: {title}"), &desc, |ui| {
                        widgets::icon(ui, if ok { ICON_CHECK_CIRCLE } else { ICON_WARNING }, 22.0, if ok { p.primary } else { p.error });
                    });
                    ui.separator();
                    let ub = ublock::installed(&plan.profile_dir);
                    row(ui, "uBlock Origin", if !app.config.ublock { "Turned off." } else if ub { "Installed; it keeps itself updated." } else { "Downloaded from addons.mozilla.org when the browser first opens." }, |ui| {
                        if switch(ui, &mut app.config.ublock).changed() {
                            actions.push(Action::ConfigChanged);
                        }
                    });
                    ui.separator();
                    let palette = match &theme_file {
                        Some(f) => format!("Following {} live (end4-pC / illogical-impulse).", paths::tilde(f)),
                        None => format!("Seed color {} (no end-4 colors found).", app.config.seed_color),
                    };
                    row(ui, "Palette", &palette, |ui| {
                        if widgets::button(ui, ButtonKind::Text, None, "Appearance").clicked() {
                            app.page = Page::Appearance;
                        }
                    });
                    ui.separator();
                    row(ui, "Profile", &paths::tilde(&plan.profile_dir), |ui| {
                        if widgets::button(ui, ButtonKind::Text, Some(ICON_FOLDER_OPEN), "Show").clicked() {
                            let _ = crate::sys::cmd::spawn_detached("xdg-open", &[&plan.profile_dir.to_string_lossy()]);
                        }
                    });
                });
                if matches!(plan.mode, ExtensionMode::NeedsSigning) {
                    common::banner(ui, ICON_INFO, "Without the extension you still get the profile, the Material 3 shapes (in neutral colors) and uBlock Origin, but not website theming or the live palette.", false, |ui| {
                        if widgets::button(ui, ButtonKind::Tonal, None, "Options").clicked() {
                            app.page = Page::Extension;
                        }
                    });
                }
            }
        }
        card(ui, "What you get", |ui| {
            common::note(ui, "• Websites themed with your Material 3 palette, with a blacklist, whitelist mode and per-site options.\n• Zen's interface recolored live from your wallpaper, with Material 3 shapes.\n• A Material 3 new tab page with search, speed dial and your wallpaper.\n• uBlock Origin, ready to go.\n• Its own window class and dock icon, separate from your normal Zen or Firefox.");
        });
    });
    app.actions.extend(actions);
}

fn engine_page(ui: &mut Ui, app: &mut App) {
    let mut actions = Vec::new();
    common::page(ui, ICON_PUBLIC, "Browser engine", "Which installed browser runs your Material 3 profile.", |ui| {
        let engines = app.engines.value.clone().unwrap_or_default();
        card(ui, "Installed", |ui| {
            if app.engines.running() && engines.is_empty() {
                common::loading(ui, "Looking for browsers…");
                return;
            }
            let auto_label = engines.first().map(|e| format!("Use the best available ({}).", e.label)).unwrap_or_else(|| "Nothing installed yet.".into());
            row(ui, "Automatic", &auto_label, |ui| {
                let mut on = app.config.engine == EngineChoice::Auto;
                if switch(ui, &mut on).changed() && on {
                    app.config.engine = EngineChoice::Auto;
                    actions.push(Action::ConfigChanged);
                }
            });
            for e in &engines {
                ui.separator();
                let desc = format!("{}{}", e.command.join(" "), if e.requires_signing { " · signed add-ons only" } else { " · allows unsigned add-ons" });
                row(ui, e.label, &desc, |ui| {
                    let selected = app.config.engine == e.choice;
                    if widgets::button_enabled(ui, if selected { ButtonKind::Tonal } else { ButtonKind::Outlined }, selected.then_some(ICON_CHECK), if selected { "In use" } else { "Use" }, !selected).clicked() {
                        app.config.engine = e.choice;
                        actions.push(Action::ConfigChanged);
                    }
                });
            }
        });
        if engines.iter().all(|e| !matches!(e.choice, EngineChoice::Zen | EngineChoice::ZenFlatpak)) {
            common::banner(ui, ICON_INFO, "Zen isn't installed. It's the recommended base: paru -S zen-browser-bin, or flatpak install flathub app.zen_browser.zen.", false, |_| {});
        }
        card(ui, "Profile folder", |ui| {
            row(ui, "Location", "Leave empty for the default. The Zen Flatpak can only use a folder inside its own data folder.", |ui| {
                let r = ui.add(egui::TextEdit::singleline(&mut app.config.profile_dir).hint_text("default").desired_width(260.0));
                if r.lost_focus() {
                    actions.push(Action::ConfigChanged);
                }
            });
        });
        if let Ok(plan) = app.plan.clone() {
            if plan.engine.flatpak {
                card(ui, "Flatpak", |ui| {
                    row(ui, "Live palette inside the Flatpak", "The palette host runs outside the sandbox through flatpak-spawn, which needs the org.freedesktop.Flatpak permission for Zen. Without it the extension uses the seed color.", |ui| {
                        if switch(ui, &mut app.config.flatpak_host_access).changed() {
                            let grant = app.config.flatpak_host_access;
                            let flag = if grant { "--talk-name=org.freedesktop.Flatpak" } else { "--no-talk-name=org.freedesktop.Flatpak" };
                            match crate::sys::cmd::run("flatpak", &["override", "--user", flag, engine::ZEN_FLATPAK_ID]) {
                                Ok(_) => actions.push(Action::ConfigChanged),
                                Err(e) => actions.push(Action::Toast(e, ToastKind::Error)),
                            }
                        }
                    });
                });
            }
        }
    });
    app.actions.extend(actions);
}

fn extension(ui: &mut Ui, app: &mut App) {
    let p = app.palette.clone();
    let mut actions = Vec::new();
    common::page(ui, ICON_EXTENSION, "Extension & signing", "How the Material 3 extension gets into the browser.", |ui| {
        if let Ok(plan) = app.plan.clone() {
            let (title, desc, _) = mode_text(&plan.mode);
            card(ui, "Now", |ui| {
                row(ui, title, &desc, |_| {});
            });
        }
        card(ui, "Why signing matters", |ui| {
            common::note(ui, "Zen and Firefox's regular releases only run add-ons signed by Mozilla. Material 3 Browser's releases are signed automatically once the project's addons.mozilla.org keys are set up; the signed extension then installs permanently with no extra steps.\n\nLibreWolf and Firefox Developer Edition, Nightly and ESR can run unsigned add-ons, so they work with any build.");
        });
        card(ui, "Development loading", |ui| {
            row(ui, "Load the extension at every start", "Installs the extension temporarily each time the browser starts, the way developers test add-ons. Off by default: while the browser runs, it listens on a debugging socket in your private runtime folder, which programs running as you could use to control the browser. Not available for the Zen Flatpak.", |ui| {
                if switch(ui, &mut app.config.dev_load).changed() {
                    actions.push(Action::ConfigChanged);
                }
            });
            if app.config.dev_load {
                ui.horizontal(|ui| {
                    widgets::icon(ui, ICON_WARNING, 18.0, p.tertiary);
                    ui.label(RichText::new("Turn this off once a signed release is installed.").color(p.tertiary));
                });
                ui.add_space(8.0);
            }
        });
    });
    app.actions.extend(actions);
}

fn import(ui: &mut Ui, app: &mut App) {
    let p = app.palette.clone();
    common::page(ui, ICON_DOWNLOAD, "Import", "Copy bookmarks, history, favicons and saved passwords from another profile. Close both browsers first.", |ui| {
        card(ui, "Profiles found", |ui| {
            if app.import_sources.is_empty() {
                common::note(ui, "No Zen, Firefox or LibreWolf profiles were found.");
                return;
            }
            let target = app.plan.as_ref().ok().map(|p| p.profile_dir.clone());
            let busy = app.import.running();
            let sources = app.import_sources.clone();
            for (i, s) in sources.iter().enumerate() {
                if i > 0 {
                    ui.separator();
                }
                row(ui, &format!("{} · {}", s.browser, s.name), &paths::tilde(&s.path), |ui| {
                    if widgets::button_enabled(ui, ButtonKind::Tonal, Some(ICON_DOWNLOAD), "Import", !busy && target.is_some()).clicked() {
                        if let Some(to) = target.clone() {
                            let from = s.path.clone();
                            app.import.spawn(&app.ctx, move || crate::profile::import(&from, &to));
                        }
                    }
                });
            }
            ui.add_space(4.0);
            ui.label(RichText::new("Importing replaces the Material 3 profile's bookmarks, history and passwords.").size(12.5).color(p.on_surface_variant));
            ui.add_space(8.0);
        });
    });
}

fn appearance(ui: &mut Ui, app: &mut App) {
    let p = app.palette.clone();
    let theme_file = app.theme_file();
    let mut changed = false;
    common::page(ui, ICON_PALETTE, "Appearance", "The palette used for websites, the browser and this window.", |ui| {
        card(ui, "Palette", |ui| {
            let desc = match (&app.config.theme_source, &theme_file) {
                (ThemeSource::Auto, Some(f)) => format!("Following {} (updates live with your wallpaper).", paths::tilde(f)),
                (ThemeSource::Auto, None) => "No end-4 colors found, so the seed color below is used.".into(),
                (ThemeSource::Seed, _) => "Using a palette generated from the seed color below.".into(),
            };
            row(ui, "Color source", &desc, |ui| {
                changed |= segmented(ui, &mut app.config.theme_source, &[(ThemeSource::Auto, ICON_WALLPAPER, "Auto"), (ThemeSource::Seed, ICON_PALETTE, "Seed")]);
            });
            ui.separator();
            ui.add_enabled_ui(theme_file.is_none(), |ui| {
                row(ui, "Color scheme", "Light, dark, or follow the system.", |ui| {
                    changed |= segmented(ui, &mut app.config.color_scheme, &[
                        (ColorScheme::System, ICON_BRIGHTNESS_AUTO, "System"),
                        (ColorScheme::Light, ICON_LIGHT_MODE, "Light"),
                        (ColorScheme::Dark, ICON_DARK_MODE, "Dark"),
                    ]);
                });
                ui.separator();
                row(ui, "Seed color", "Every color is derived from this one.", |ui| {
                    let text = app.seed_text.get_or_insert_with(|| app.config.seed_color.clone());
                    let resp = ui.add(egui::TextEdit::singleline(text).desired_width(92.0).char_limit(7));
                    if resp.changed() {
                        if let Some(c) = parse_hex(text) {
                            app.config.seed_color = to_hex(c);
                            changed = true;
                        }
                    }
                    let mut color = parse_hex(&app.config.seed_color).unwrap_or(Color32::from_rgb(0x42, 0x85, 0xF4));
                    if ui.color_edit_button_srgba(&mut color).changed() {
                        app.config.seed_color = to_hex(color);
                        *text = app.config.seed_color.clone();
                        changed = true;
                    }
                    for hex in SEED_PRESETS.iter().rev() {
                        let c = parse_hex(hex).expect("valid preset");
                        let (rect, resp) = ui.allocate_exact_size(vec2(24.0, 24.0), egui::Sense::click());
                        ui.painter().circle_filled(rect.center(), 11.0, c);
                        if app.config.seed_color.eq_ignore_ascii_case(hex) {
                            ui.painter().circle_stroke(rect.center(), 12.5, egui::Stroke::new(2.0, p.on_surface));
                        }
                        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            app.config.seed_color = hex.to_string();
                            *text = hex.to_string();
                            changed = true;
                        }
                    }
                });
            });
        });
        card(ui, "Website theming", |ui| {
            common::note(ui, "Theming modes, the blacklist and per-site options live in the browser: click the Material 3 button in the toolbar, or open the extension's settings from about:addons.");
        });
        card(ui, "This window", |ui| {
            row(ui, "Interface scale", "Size of everything in this window.", |ui| {
                let r = ui.add(egui::Slider::new(&mut app.config.ui_scale, 0.75..=1.75).step_by(0.05).custom_formatter(|v, _| format!("{:.0}%", v * 100.0)));
                if r.drag_stopped() || (r.changed() && !r.dragged()) {
                    changed = true;
                }
            });
        });
    });
    if changed {
        app.actions.push(Action::ConfigChanged);
    }
}

fn about(ui: &mut Ui, app: &mut App) {
    let _ = app;
    common::page(ui, ICON_INFO, "About", "", |ui| {
        card(ui, "Material 3 Browser", |ui| {
            common::facts(ui, "about", &[
                ("Version", env!("CARGO_PKG_VERSION").to_string()),
                ("License", "MIT".into()),
                ("Settings", crate::config::Config::path().map(|p| paths::tilde(&p)).unwrap_or_default()),
                ("Website theming", "Dark Reader's engine (MIT)".into()),
                ("Ad blocking", "uBlock Origin (GPLv3), downloaded from addons.mozilla.org".into()),
                ("Fonts", "Inter (SIL OFL 1.1)".into()),
            ]);
        });
    });
}
