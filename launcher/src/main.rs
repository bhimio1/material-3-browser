//! Material 3 Browser: runs Zen (or Firefox/LibreWolf) with a Material 3
//! profile, the palette host and the theming extension.

mod app;
mod config;
mod devload;
mod engine;
mod host;
mod launch;
mod profile;
mod sys;
mod theme;
mod ublock;
mod ui;

const USAGE: &str = "Usage: m3-browser [OPTIONS] [URL…]

Starts the browser with the Material 3 profile. The first run opens the setup
window instead.

Options:
  --settings      Open the setup / settings window
  --status        Print what would run, as JSON
  --native-host   Run as the extension's native messaging host (used by the browser)
  -h, --help      Show this help
  -V, --version";

fn run_settings(setup: bool) -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Material 3 Browser")
            .with_app_id("material_3_browser_settings")
            .with_inner_size([1040.0, 720.0])
            .with_min_inner_size([720.0, 480.0]),
        ..Default::default()
    };
    eframe::run_native(
        "material_3_browser_settings",
        options,
        Box::new(move |cc| Ok(Box::new(app::App::new(cc, setup)))),
    )
}

fn status_json() -> serde_json::Value {
    let config = config::Config::load();
    match launch::plan(&config) {
        Ok(plan) => serde_json::json!({
            "engine": plan.engine.label,
            "command": plan.engine.command,
            "profile": plan.profile_dir,
            "extension": format!("{:?}", plan.mode),
            "ublock": ublock::installed(&plan.profile_dir),
        }),
        Err(e) => serde_json::json!({"error": e}),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // Firefox starts native hosts with the manifest path and extension id.
    let native = args.first().is_some_and(|a| a == "--native-host")
        || (args.len() >= 2 && args[0].ends_with(".json") && args[1] == profile::EXTENSION_ID);
    if native {
        if let Err(e) = host::run() {
            eprintln!("m3-browser host: {e}");
            std::process::exit(1);
        }
        return;
    }
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    let mut urls = Vec::new();
    let mut settings = false;
    for a in &args {
        match a.as_str() {
            "-h" | "--help" => {
                println!("{USAGE}");
                return;
            }
            "-V" | "--version" => {
                println!("m3-browser {}", env!("CARGO_PKG_VERSION"));
                return;
            }
            "--settings" => settings = true,
            "--status" => {
                println!("{}", serde_json::to_string_pretty(&status_json()).unwrap_or_default());
                return;
            }
            other if other.starts_with('-') && !other.contains("://") => {
                eprintln!("Unknown option: {other}\n\n{USAGE}");
                std::process::exit(2);
            }
            url => urls.push(url.to_string()),
        }
    }

    let config = config::Config::load();
    if settings || !config.setup_done {
        if let Err(e) = run_settings(!config.setup_done) {
            eprintln!("{e}");
            std::process::exit(1);
        }
        return;
    }
    let result = launch::plan(&config).and_then(|plan| {
        for w in launch::prepare(&plan, &config)? {
            eprintln!("m3-browser: {w}");
        }
        launch::launch(&plan, &urls)
    });
    if let Err(e) = result {
        eprintln!("m3-browser: {e}");
        let _ = sys::cmd::spawn_detached("notify-send", &["-a", "Material 3 Browser", "Couldn't start the browser", &e]);
        std::process::exit(1);
    }
}
