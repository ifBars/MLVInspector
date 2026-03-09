use dioxus::prelude::*;

use crate::shortcuts::{binding_from_keyboard_event, ShortcutKey, ShortcutSettings};

use super::commands::{command_definition, command_definitions, CommandId};
use super::overlay::OverlayKind;
use super::theme::{C_ACCENT_AMBER, C_TEXT_MUTED, C_TEXT_PRIMARY};

#[component]
pub fn SettingsOverlay(
    active_overlay: Signal<Option<OverlayKind>>,
    shortcut_settings: Signal<ShortcutSettings>,
    editing_command: Signal<Option<CommandId>>,
) -> Element {
    if active_overlay() != Some(OverlayKind::Settings) {
        return rsx! {};
    }

    let footer_message = editing_command()
        .map(|command_id| {
            let definition = command_definition(command_id);
            format!("Recording shortcut for {}", definition.title)
        })
        .unwrap_or_else(|| "Click Record, then press the new shortcut".to_string());

    rsx! {
        div {
            class: "command-palette-overlay",
            onclick: move |_| {
                editing_command.set(None);
                active_overlay.set(None);
            },

            div {
                class: "command-palette settings-overlay no-drag",
                onclick: move |evt| evt.stop_propagation(),

                div {
                    class: "command-palette-search settings-header",
                    div {
                        style: "display: grid; gap: 4px; min-width: 0;",
                        div {
                            style: format!("font-size: 13px; font-weight: 600; color: {C_TEXT_PRIMARY};"),
                            "Keyboard shortcuts"
                        }
                        div {
                            style: format!("font-size: 11px; color: {C_TEXT_MUTED};"),
                            "Capture new bindings, reset individual commands, or restore the defaults."
                        }
                    }

                    div {
                        style: "display: flex; align-items: center; gap: 8px;",
                        button {
                            class: "command-palette-close",
                            onclick: move |_| {
                                shortcut_settings.write().restore_defaults();
                                editing_command.set(None);
                            },
                            "Restore defaults"
                        }
                        button {
                            class: "command-palette-close",
                            onclick: move |_| {
                                editing_command.set(None);
                                active_overlay.set(None);
                            },
                            "Close"
                        }
                    }
                }

                div {
                    class: "command-palette-results settings-results",
                    for definition in command_definitions() {
                        {
                            let command_id = definition.id;
                            let is_recording = editing_command() == Some(command_id);
                            let current_shortcut = shortcut_settings()
                                .binding_for(command_id)
                                .map(|binding| binding.display_label())
                                .unwrap_or_else(|| "Unassigned".to_string());
                            let default_shortcut = command_definition(command_id)
                                .default_shortcut
                                .display_label();

                            rsx! {
                                div {
                                    key: "shortcut-setting-{definition.title}",
                                    class: "settings-row",

                                    div {
                                        style: "display: grid; gap: 4px; min-width: 0;",
                                        div {
                                            style: format!("font-size: 12px; font-weight: 600; color: {C_TEXT_PRIMARY};"),
                                            "{definition.title}"
                                        }
                                        div {
                                            style: format!("font-size: 10px; color: {C_TEXT_MUTED};"),
                                            "{definition.description}"
                                        }
                                        div {
                                            style: format!("font-size: 10px; color: {C_ACCENT_AMBER};"),
                                            "Default: {default_shortcut}"
                                        }
                                    }

                                    div {
                                        class: "settings-row-actions",
                                        div {
                                            class: if is_recording { "shortcut-badge shortcut-badge-live" } else { "shortcut-badge" },
                                            "{current_shortcut}"
                                        }
                                        button {
                                            class: if is_recording { "command-palette-close shortcut-capture active" } else { "command-palette-close shortcut-capture" },
                                            onclick: move |_| {
                                                if is_recording {
                                                    editing_command.set(None);
                                                } else {
                                                    editing_command.set(Some(command_id));
                                                }
                                            },
                                            onkeydown: move |event| {
                                                if !is_recording {
                                                    return;
                                                }

                                                event.prevent_default();
                                                event.stop_propagation();

                                                if event.is_auto_repeating() {
                                                    return;
                                                }

                                                let Some(binding) = binding_from_keyboard_event(&event) else {
                                                    return;
                                                };

                                                if matches!(binding.key, ShortcutKey::Escape)
                                                    && !binding.ctrl
                                                    && !binding.alt
                                                    && !binding.shift
                                                    && !binding.super_key
                                                {
                                                    editing_command.set(None);
                                                    return;
                                                }

                                                shortcut_settings.write().set_binding(command_id, binding);
                                                editing_command.set(None);
                                            },
                                            if is_recording {
                                                "Press shortcut..."
                                            } else {
                                                "Record"
                                            }
                                        }
                                        button {
                                            class: "command-palette-close shortcut-capture",
                                            onclick: move |_| {
                                                shortcut_settings.write().reset_binding(command_id);
                                                editing_command.set(None);
                                            },
                                            "Reset"
                                        }
                                        button {
                                            class: "command-palette-close shortcut-capture",
                                            onclick: move |_| {
                                                shortcut_settings.write().clear_binding(command_id);
                                                editing_command.set(None);
                                            },
                                            "Clear"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                div {
                    class: "command-palette-footer",
                    span { "{footer_message}" }
                    span { "Duplicate bindings automatically move to the latest command" }
                }
            }
        }
    }
}
