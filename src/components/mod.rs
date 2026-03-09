/// Component modules for the MLVInspector UI.
///
/// Each module is a focused, independently maintainable unit:
/// - `theme`            — design tokens and global CSS
/// - `view_models`      — UI-facing data types derived from IPC payloads
/// - `resize`           — panel resize types and helpers
/// - `helpers`          — pure data-transformation functions
/// - `analysis`         — background analysis runner
/// - `title_bar`        — custom draggable title bar
/// - `status_bar`       — bottom status / metrics bar
/// - `explorer_panel`   — left panel: assemblies plus namespace/type/method tree
/// - `il_view_panel`    — center panel: IL instruction viewer + C# decompiler
/// - `findings_panel`   — right panel: scan findings list
pub mod analysis;
pub mod command_palette;
pub mod commands;
pub mod csharp_highlight;
pub mod explorer_panel;
pub mod explorer_tools;
pub mod findings_panel;
pub mod helpers;
pub mod il_view_panel;
pub mod overlay;
pub mod resize;
pub mod settings_overlay;
pub mod status_bar;
pub mod theme;
pub mod title_bar;
pub mod view_models;

pub use analysis::run_analysis;
pub use command_palette::CommandPalette;
pub use commands::{dispatch_shortcut_binding, CommandContext, CommandId};
pub use explorer_panel::ExplorerPanel;
pub use findings_panel::FindingsPanel;
pub use helpers::extract_findings;
pub use il_view_panel::IlViewPanel;
pub use overlay::OverlayKind;
pub use resize::{clamp_panel_width, ActiveResize, ResizeTarget};
pub use settings_overlay::SettingsOverlay;
pub use status_bar::StatusBar;
pub use theme::{global_css, C_ACCENT_BLUE, C_BG_BASE, C_TEXT_PRIMARY, FONT_SANS};
pub use title_bar::TitleBar;
pub use view_models::IlTab;
