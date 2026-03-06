//! MLVInspector — Dioxus Desktop Edition
//!
//! Backend-first port of MLVInspector.Tauri.
//! Provides .NET assembly static analysis via ILInspector subprocess.

use dioxus::prelude::*;

mod app;
mod components;
mod error;
mod ipc;
mod services;
mod state;
mod types;

use app::App;

fn main() {
    // Initialize tracing subscriber for structured logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "mlvinspector=debug,info".parse().expect("valid filter")),
        )
        .init();

    tracing::info!("MLVInspector Dioxus starting...");

    LaunchBuilder::new()
        .with_cfg(
            dioxus::desktop::Config::new().with_window(
                dioxus::desktop::WindowBuilder::new()
                    .with_title("MLVInspector")
                    .with_inner_size(dioxus::desktop::LogicalSize::new(1400.0, 900.0))
                    .with_min_inner_size(dioxus::desktop::LogicalSize::new(1100.0, 700.0))
                    .with_resizable(true)
                    .with_decorations(false),
            ),
        )
        .launch(App);
}
