//! The setup / settings window: shows what will run and how, and lets the
//! user pick the engine, signing workaround, uBlock Origin and import.

use crate::config::Config;
use crate::engine::{self, Engine};
use crate::launch::{self, Plan};
use crate::ui::{self, Action, Page, ToastKind};
use egui::{CornerRadius, Frame, Margin};
use material_3_ui::sys::task::Task;
use material_3_ui::theme::{M3Palette, ThemeManager};
use material_3_ui::toasts::{Toast, Toasts};
use material_3_ui::widgets;

pub struct App {
    pub ctx: egui::Context,
    /// The theme's current palette.
    pub palette: M3Palette,
    pub theme: ThemeManager,
    pub config: Config,
    pub page: Page,
    pub toasts: Toasts<Action>,
    pub actions: Vec<Action>,
    pub engines: Task<Vec<Engine>>,
    pub plan: Result<Plan, String>,
    /// Preparing the profile and starting the browser.
    pub launching: Task<Result<Vec<String>, String>>,
    pub import: Task<Result<Vec<&'static str>, String>>,
    pub import_sources: Vec<crate::profile::SourceProfile>,
    /// First run: this window is the setup assistant.
    pub setup: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, setup: bool) -> Self {
        let ctx = cc.egui_ctx.clone();
        let config = Config::load();
        let theme = ThemeManager::new(&ctx, &config.appearance);
        let mut engines = Task::default();
        engines.spawn(&ctx, engine::available);
        let mut app = Self {
            palette: theme.palette().clone(),
            theme,
            plan: launch::plan(&config),
            config,
            page: Page::Overview,
            toasts: Toasts::default(),
            actions: Vec::new(),
            engines,
            launching: Task::default(),
            import: Task::default(),
            import_sources: crate::profile::import_sources(),
            setup,
            ctx,
        };
        app.theme_applied();
        app
    }

    pub fn toast(&mut self, text: impl Into<String>, kind: ToastKind) {
        self.toasts.push(Toast::new(text, kind));
    }

    /// Apply the appearance settings after they changed.
    pub fn apply_theme(&mut self) {
        self.theme.apply(&self.config.appearance);
        self.theme_applied();
    }

    fn theme_applied(&mut self) {
        if let Some(e) = self.theme.take_error() {
            self.toast(e, ToastKind::Error);
        }
        self.palette = self.theme.palette().clone();
    }

    fn process(&mut self, action: Action) {
        match action {
            Action::Toast(t, k) => self.toast(t, k),
            Action::ConfigChanged => {
                if let Err(e) = self.config.save() {
                    self.toast(format!("Couldn't save settings: {e}"), ToastKind::Error);
                }
                self.plan = launch::plan(&self.config);
                self.apply_theme();
            }
            Action::Launch => {
                if self.launching.running() {
                    return;
                }
                self.config.setup_done = true;
                let _ = self.config.save();
                let config = self.config.clone();
                self.launching.spawn(&self.ctx, move || {
                    let plan = launch::plan(&config)?;
                    let warnings = launch::prepare(&plan, &config)?;
                    launch::launch(&plan, &[])?;
                    Ok(warnings)
                });
            }
        }
    }

    fn poll(&mut self) {
        // Palette file changes and system light/dark switches.
        if self.theme.poll(&self.config.appearance) {
            self.theme_applied();
        }
        self.engines.poll();
        if let Some(result) = self.launching.take() {
            match result {
                Ok(warnings) => {
                    for w in warnings {
                        self.toast(w, ToastKind::Error);
                    }
                    self.toast("Browser started", ToastKind::Success);
                    self.plan = launch::plan(&self.config);
                }
                Err(e) => self.toast(e, ToastKind::Error),
            }
        }
        if let Some(result) = self.import.take() {
            match result {
                Ok(files) if files.is_empty() => self.toast("Nothing to import in that profile", ToastKind::Info),
                Ok(files) => self.toast(format!("Imported {}", files.join(", ")), ToastKind::Success),
                Err(e) => self.toast(e, ToastKind::Error),
            }
        }
    }

    fn sidebar(&mut self, ui: &mut egui::Ui) {
        use material_3_ui::icons::icons::*;
        let p = self.palette.clone();
        ui.horizontal(|ui| {
            ui.add_space(12.0);
            widgets::icon(ui, ICON_PUBLIC, 26.0, p.primary);
            ui.label(widgets::title_text("Browser", 20.0).color(p.on_surface));
        });
        ui.add_space(14.0);
        let busy = self.launching.running();
        let fab = egui::vec2(ui.available_width().min(170.0), 52.0);
        let (rect, resp) = ui.allocate_exact_size(fab, egui::Sense::click());
        let fill = widgets::state_layer(p.primary_container, p.on_primary_container, &resp);
        ui.painter().rect_filled(rect, CornerRadius::same(16), fill);
        widgets::paint_icon(ui, ICON_OPEN_IN_NEW, egui::pos2(rect.left() + 26.0, rect.center().y), 22.0, p.on_primary_container);
        ui.painter().text(
            egui::pos2(rect.left() + 48.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            if busy { "Starting…" } else { "Open browser" },
            widgets::title_font(15.0),
            p.on_primary_container,
        );
        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            self.actions.push(Action::Launch);
        }
        ui.add_space(14.0);
        for page in Page::ALL {
            if widgets::nav_item(ui, page.icon(), page.title(), self.page == page, None).clicked() {
                self.page = page;
            }
        }
    }
}

impl eframe::App for App {
    fn logic(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll();
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let p = self.palette.clone();
        let surface = Frame::new().fill(p.surface);
        egui::Panel::left("sidebar")
            .resizable(false)
            .default_size(240.0)
            .show_separator_line(false)
            .frame(surface.inner_margin(Margin { left: 12, right: 4, top: 12, bottom: 12 }))
            .show(ui, |ui| self.sidebar(ui));
        egui::CentralPanel::default()
            .frame(surface.inner_margin(Margin { left: 8, right: 12, top: 12, bottom: 12 }))
            .show(ui, |ui| {
                Frame::new()
                    .fill(p.surface_container_low)
                    .corner_radius(CornerRadius::same(24))
                    .show(ui, |ui| {
                        ui.set_min_size(ui.available_size());
                        ui::pages::show(ui, self);
                    });
            });
        let fired = self.toasts.show(&self.ctx.clone(), &p, 20.0);
        self.actions.extend(fired);
        while !self.actions.is_empty() {
            for a in std::mem::take(&mut self.actions) {
                self.process(a);
            }
        }
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        self.palette.surface.to_normalized_gamma_f32()
    }
}
