/// Custom title bar with drag region, native-feel menu bar, and window controls.
use dioxus::prelude::*;
use dioxus_desktop::window;

use crate::shortcuts::ShortcutSettings;
use crate::state::AppState;

use super::commands::{execute_command, is_command_enabled, CommandContext, CommandId};
use super::overlay::OverlayKind;
use super::theme::{C_ACCENT_GREEN, C_BORDER};
use super::title_bar_actions::{command_shortcut_label, MENUS};
use super::view_models::IlTab;

const APP_ICON: Asset = asset!("/assets/icon.png");

fn menu_entry(
    text: &'static str,
    command_id: CommandId,
    command_context: CommandContext,
    state: AppState,
    active_overlay: Signal<Option<OverlayKind>>,
    shortcut_settings: Signal<ShortcutSettings>,
    mut close_menus: Signal<Option<&'static str>>,
) -> Element {
    let enabled = is_command_enabled(state, active_overlay(), command_id);
    let shortcut_label = {
        let shortcuts = shortcut_settings.read();
        command_shortcut_label(&shortcuts, command_id)
    };

    rsx! {
        button {
            class: if enabled { "menu-entry" } else { "menu-entry disabled" },
            disabled: !enabled,
            onclick: move |_| {
                if enabled {
                    execute_command(command_context, command_id);
                    close_menus.set(None);
                }
            },
            span {
                class: "menu-entry-label",
                "{text}"
            }
            if let Some(shortcut_label) = shortcut_label {
                span {
                    class: "menu-shortcut",
                    "{shortcut_label}"
                }
            }
        }
    }
}

#[component]
pub fn TitleBar(
    active_overlay: Signal<Option<OverlayKind>>,
    shortcut_settings: Signal<ShortcutSettings>,
    show_scan_panel: Signal<bool>,
    last_error: Signal<String>,
    open_tabs: Signal<Vec<IlTab>>,
    active_tab_id: Signal<Option<String>>,
    selected_finding: Signal<Option<usize>>,
) -> Element {
    let state = use_context::<AppState>();
    let desktop_window = window();
    let initial_maximized = desktop_window.is_maximized();
    let mut is_maximized = use_signal(move || initial_maximized);
    let mut open_menu = use_signal(|| None::<&'static str>);
    let is_running = *state.is_running.read();

    let desktop_window_min = desktop_window.clone();
    let desktop_window_caption_toggle = desktop_window.clone();
    let desktop_window_button_toggle = desktop_window.clone();
    let desktop_window_close = desktop_window.clone();

    let command_context = CommandContext {
        state,
        active_overlay,
        show_scan_panel,
        last_error,
        open_tabs,
        active_tab_id,
        selected_finding,
    };

    rsx! {
        div {
            class: "drag-region title-shell",
            style: format!("border-bottom: 1px solid {C_BORDER};"),

            div {
                class: "caption-row",
                ondoubleclick: move |_| {
                    let next = !is_maximized();
                    is_maximized.set(next);
                    desktop_window_caption_toggle.set_maximized(next);
                },

                div {
                    class: "caption-left",

                    div {
                        class: "title-identity",

                        img {
                            src: APP_ICON,
                            alt: "MLVInspector icon",
                            width: "18",
                            height: "18",
                            style: "width: 18px; height: 18px; object-fit: contain; display: block;"
                        }
                        span {
                            class: "title-product",
                            "MLVInspector"
                        }
                        if is_running {
                            span {
                                class: "title-live-indicator",
                                style: format!("color: {C_ACCENT_GREEN};"),
                                "ANALYZING"
                            }
                        }
                    }

                    div {
                        class: "no-drag menu-bar",
                        onmouseleave: move |_| open_menu.set(None),
                        for section in MENUS.iter() {
                            div {
                                class: "menu-column",
                                button {
                                    class: if open_menu() == Some(section.title) {
                                        "menu-trigger active"
                                    } else {
                                        "menu-trigger"
                                    },
                                    onmouseenter: {
                                        let title = section.title;
                                        move |_| {
                                            if open_menu().is_some() && open_menu() != Some(title) {
                                                open_menu.set(Some(title));
                                            }
                                        }
                                    },
                                    onclick: {
                                        let title = section.title;
                                        move |_| {
                                            if open_menu() == Some(title) {
                                                open_menu.set(None);
                                            } else {
                                                open_menu.set(Some(title));
                                            }
                                        }
                                    },
                                    "{section.title}"
                                }

                                if open_menu() == Some(section.title) {
                                    div {
                                        class: "menu-popover",
                                        for item in section.items {
                                            {menu_entry(
                                                item.label,
                                                item.command_id,
                                                command_context,
                                                state,
                                                active_overlay,
                                                shortcut_settings,
                                                open_menu,
                                            )}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                div {
                    class: "no-drag caption-controls",

                    button {
                        class: "caption-btn",
                        title: "Minimize",
                        "aria-label": "Minimize window",
                        onclick: move |_| desktop_window_min.set_minimized(true),
                        svg {
                            width: "10",
                            height: "10",
                            view_box: "0 0 10 10",
                            fill: "none",
                            stroke: "currentColor",
                            stroke_width: "1",
                            path { d: "M1 5.5h8" }
                        }
                    }

                    button {
                        class: "caption-btn",
                        title: if is_maximized() { "Restore" } else { "Maximize" },
                        "aria-label": if is_maximized() {
                            "Restore window"
                        } else {
                            "Maximize window"
                        },
                        onclick: move |_| {
                            let next = !is_maximized();
                            is_maximized.set(next);
                            desktop_window_button_toggle.set_maximized(next);
                        },
                        if is_maximized() {
                            svg {
                                width: "10",
                                height: "10",
                                view_box: "0 0 10 10",
                                fill: "none",
                                stroke: "currentColor",
                                stroke_width: "1",
                                path { d: "M2.5 1.5h5v5h-5z" }
                                path { d: "M1.5 3.5v5h5" }
                            }
                        } else {
                            svg {
                                width: "10",
                                height: "10",
                                view_box: "0 0 10 10",
                                fill: "none",
                                stroke: "currentColor",
                                stroke_width: "1",
                                rect { x: "1.5", y: "1.5", width: "7", height: "7" }
                            }
                        }
                    }

                    button {
                        class: "caption-btn close",
                        title: "Close",
                        "aria-label": "Close window",
                        onclick: move |_| desktop_window_close.close(),
                        svg {
                            width: "10",
                            height: "10",
                            view_box: "0 0 10 10",
                            fill: "none",
                            stroke: "currentColor",
                            stroke_width: "1",
                            path { d: "M2 2l6 6" }
                            path { d: "M8 2 2 8" }
                        }
                    }
                }
            }
        }
    }
}
