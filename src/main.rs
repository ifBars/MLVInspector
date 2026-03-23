//! MLVInspector — Dioxus Desktop Edition
//!
//! Backend-first port of MLVInspector.Tauri.
//! Provides .NET assembly static analysis via ILInspector subprocess.

use dioxus::prelude::*;
use image::GenericImageView;

mod app;
mod components;
mod error;
mod ipc;
mod services;
mod shortcuts;
mod state;
mod types;

use app::App;

fn load_window_icon() -> Option<dioxus_desktop::tao::window::Icon> {
    #[cfg(target_os = "windows")]
    {
        let png_bytes = include_bytes!("../assets/icon.png");
        let icon_image = image::load_from_memory(png_bytes).ok()?;
        let (width, height) = icon_image.dimensions();
        let rgba = icon_image.to_rgba8().into_raw();
        dioxus_desktop::tao::window::Icon::from_rgba(rgba, width, height).ok()
    }
    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

fn main() {
    // Initialize tracing subscriber for structured logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "mlvinspector=debug,info".parse().expect("valid filter")),
        )
        .init();

    tracing::info!("MLVInspector Dioxus starting...");

    let icon = load_window_icon();
    let window = dioxus::desktop::WindowBuilder::new()
        .with_title("MLVInspector")
        .with_inner_size(dioxus::desktop::LogicalSize::new(1400.0, 900.0))
        .with_min_inner_size(dioxus::desktop::LogicalSize::new(1100.0, 700.0))
        .with_resizable(true)
        .with_decorations(false)
        .with_window_icon(icon.clone());
    let mut cfg = dioxus::desktop::Config::new().with_window(window);
    if let Some(icon) = icon {
        cfg = cfg.with_icon(icon);
    }

    LaunchBuilder::new().with_cfg(cfg).launch(App);
}
