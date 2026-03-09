use std::collections::HashMap;
use std::str::FromStr;

use dioxus::prelude::{KeyboardEvent, ModifiersInteraction};
use keyboard_types::{Key as WebKey, Modifiers};
use serde::{Deserialize, Serialize};

use crate::components::commands::{command_definitions, CommandId};

pub const APP_SHORTCUT_LISTENER_SCRIPT: &str = r#"
const serializeTarget = (target) => ({
    targetTag: target?.tagName ?? null,
    isContentEditable: Boolean(target?.isContentEditable),
});

window.addEventListener("keydown", (event) => {
    dioxus.send({
        key: event.key,
        ctrl: event.ctrlKey,
        alt: event.altKey,
        shift: event.shiftKey,
        meta: event.metaKey,
        repeat: event.repeat,
        ...serializeTarget(event.target),
    });
}, true);

await new Promise(() => {});
"#;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum ShortcutKey {
    Character(String),
    Escape,
    Enter,
    Space,
    Tab,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutBinding {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub super_key: bool,
    pub key: ShortcutKey,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ShortcutSettings {
    pub bindings: HashMap<CommandId, ShortcutBinding>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutKeyEventPayload {
    pub key: String,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
    pub repeat: bool,
    pub target_tag: Option<String>,
    pub is_content_editable: bool,
}

impl ShortcutKeyEventPayload {
    pub fn is_editable_target(&self) -> bool {
        self.is_content_editable
            || matches!(
                self.target_tag.as_deref(),
                Some("INPUT") | Some("TEXTAREA") | Some("SELECT")
            )
    }
}

impl ShortcutBinding {
    pub fn new(key: ShortcutKey) -> Self {
        Self {
            ctrl: false,
            alt: false,
            shift: false,
            super_key: false,
            key,
        }
    }

    pub fn with_ctrl(mut self) -> Self {
        self.ctrl = true;
        self
    }

    pub fn with_shift(mut self) -> Self {
        self.shift = true;
        self
    }

    pub fn display_label(&self) -> String {
        let mut parts = Vec::new();

        if self.ctrl {
            parts.push("Ctrl".to_string());
        }
        if self.shift {
            parts.push("Shift".to_string());
        }
        if self.alt {
            parts.push("Alt".to_string());
        }
        if self.super_key {
            parts.push("Super".to_string());
        }

        parts.push(match &self.key {
            ShortcutKey::Character(value) => match value.as_str() {
                "," => ",".to_string(),
                _ => value.to_ascii_uppercase(),
            },
            ShortcutKey::Escape => "Esc".to_string(),
            ShortcutKey::Enter => "Enter".to_string(),
            ShortcutKey::Space => "Space".to_string(),
            ShortcutKey::Tab => "Tab".to_string(),
        });

        parts.join("+")
    }
}

impl ShortcutSettings {
    pub fn with_defaults() -> Self {
        let bindings = command_definitions()
            .iter()
            .map(|definition| (definition.id, definition.default_shortcut.clone()))
            .collect();

        Self { bindings }
    }

    pub fn merged_with_defaults(mut self) -> Self {
        for definition in command_definitions() {
            self.bindings
                .entry(definition.id)
                .or_insert_with(|| definition.default_shortcut.clone());
        }

        self
    }

    pub fn binding_for(&self, command_id: CommandId) -> Option<&ShortcutBinding> {
        self.bindings.get(&command_id)
    }

    pub fn command_for_binding(&self, binding: &ShortcutBinding) -> Option<CommandId> {
        command_definitions().iter().find_map(|definition| {
            self.bindings
                .get(&definition.id)
                .filter(|candidate| *candidate == binding)
                .map(|_| definition.id)
        })
    }

    pub fn set_binding(&mut self, command_id: CommandId, binding: ShortcutBinding) {
        self.bindings.retain(|existing_id, existing_binding| {
            *existing_id == command_id || existing_binding != &binding
        });
        self.bindings.insert(command_id, binding);
    }

    pub fn clear_binding(&mut self, command_id: CommandId) {
        self.bindings.remove(&command_id);
    }

    pub fn reset_binding(&mut self, command_id: CommandId) {
        if let Some(definition) = command_definitions()
            .iter()
            .find(|definition| definition.id == command_id)
        {
            self.set_binding(command_id, definition.default_shortcut.clone());
        }
    }

    pub fn restore_defaults(&mut self) {
        *self = Self::with_defaults();
    }
}

pub fn binding_from_keyboard_event(event: &KeyboardEvent) -> Option<ShortcutBinding> {
    binding_from_web_key(event.key(), event.modifiers())
}

pub fn binding_from_shortcut_event_payload(
    payload: &ShortcutKeyEventPayload,
) -> Option<ShortcutBinding> {
    let key = WebKey::from_str(&payload.key).ok()?;
    let mut modifiers = Modifiers::empty();

    if payload.ctrl {
        modifiers.insert(Modifiers::CONTROL);
    }
    if payload.alt {
        modifiers.insert(Modifiers::ALT);
    }
    if payload.shift {
        modifiers.insert(Modifiers::SHIFT);
    }
    if payload.meta {
        modifiers.insert(Modifiers::META);
    }

    binding_from_web_key(key, modifiers)
}

fn binding_from_web_key(key: WebKey, modifiers: Modifiers) -> Option<ShortcutBinding> {
    let key = shortcut_key_from_web_key(key)?;

    Some(ShortcutBinding {
        ctrl: modifiers.ctrl(),
        alt: modifiers.alt(),
        shift: modifiers.shift(),
        super_key: modifiers.meta() || modifiers.contains(Modifiers::SUPER),
        key,
    })
}

fn shortcut_key_from_web_key(key: WebKey) -> Option<ShortcutKey> {
    match key {
        WebKey::Character(value) => normalize_character_key(&value),
        WebKey::Escape => Some(ShortcutKey::Escape),
        WebKey::Enter => Some(ShortcutKey::Enter),
        WebKey::Tab => Some(ShortcutKey::Tab),
        WebKey::Alt
        | WebKey::AltGraph
        | WebKey::CapsLock
        | WebKey::Control
        | WebKey::Fn
        | WebKey::FnLock
        | WebKey::Hyper
        | WebKey::Meta
        | WebKey::NumLock
        | WebKey::ScrollLock
        | WebKey::Shift
        | WebKey::Super
        | WebKey::Symbol
        | WebKey::SymbolLock => None,
        _ => None,
    }
}

fn normalize_character_key(value: &str) -> Option<ShortcutKey> {
    if value == " " {
        return Some(ShortcutKey::Space);
    }

    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    if trimmed.chars().count() == 1 {
        return Some(ShortcutKey::Character(trimmed.to_ascii_lowercase()));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::commands::CommandId;

    #[test]
    fn merged_with_defaults_keeps_custom_binding_and_fills_missing_values() {
        let mut settings = ShortcutSettings::default();
        settings.set_binding(
            CommandId::OpenCommandPalette,
            ShortcutBinding::new(ShortcutKey::Character("p".to_string())).with_ctrl(),
        );

        let merged = settings.merged_with_defaults();

        assert_eq!(
            merged.binding_for(CommandId::OpenCommandPalette),
            Some(&ShortcutBinding::new(ShortcutKey::Character("p".to_string())).with_ctrl())
        );
        assert!(merged.binding_for(CommandId::OpenSettings).is_some());
    }

    #[test]
    fn set_binding_unassigns_duplicate_shortcut_from_previous_command() {
        let binding = ShortcutBinding::new(ShortcutKey::Character("k".to_string())).with_ctrl();
        let mut settings = ShortcutSettings::with_defaults();

        settings.set_binding(CommandId::OpenSettings, binding.clone());

        assert_eq!(
            settings.binding_for(CommandId::OpenSettings),
            Some(&binding)
        );
        assert_eq!(settings.binding_for(CommandId::OpenCommandPalette), None);
    }

    #[test]
    fn display_label_formats_modifiers_and_special_keys() {
        let binding = ShortcutBinding::new(ShortcutKey::Escape)
            .with_ctrl()
            .with_shift();
        assert_eq!(binding.display_label(), "Ctrl+Shift+Esc");
    }

    #[test]
    fn binding_from_web_key_maps_character_and_modifiers() {
        let binding = binding_from_web_key(WebKey::Character("k".to_string()), Modifiers::CONTROL)
            .expect("binding should be created");

        assert_eq!(
            binding,
            ShortcutBinding::new(ShortcutKey::Character("k".to_string())).with_ctrl()
        );
    }

    #[test]
    fn binding_from_web_key_supports_space_shortcuts() {
        let binding = binding_from_web_key(WebKey::Character(" ".to_string()), Modifiers::SHIFT)
            .expect("binding should be created");

        assert_eq!(
            binding,
            ShortcutBinding {
                ctrl: false,
                alt: false,
                shift: true,
                super_key: false,
                key: ShortcutKey::Space,
            }
        );
    }

    #[test]
    fn binding_from_web_key_ignores_modifier_only_keys() {
        assert_eq!(binding_from_web_key(WebKey::Shift, Modifiers::SHIFT), None);
    }

    #[test]
    fn binding_from_shortcut_event_payload_maps_meta_and_character_keys() {
        let payload = ShortcutKeyEventPayload {
            key: "k".to_string(),
            ctrl: false,
            alt: false,
            shift: false,
            meta: true,
            repeat: false,
            target_tag: Some("DIV".to_string()),
            is_content_editable: false,
        };

        assert_eq!(
            binding_from_shortcut_event_payload(&payload),
            Some(ShortcutBinding {
                ctrl: false,
                alt: false,
                shift: false,
                super_key: true,
                key: ShortcutKey::Character("k".to_string()),
            })
        );
    }

    #[test]
    fn binding_from_shortcut_event_payload_maps_escape() {
        let payload = ShortcutKeyEventPayload {
            key: "Escape".to_string(),
            ctrl: false,
            alt: false,
            shift: false,
            meta: false,
            repeat: false,
            target_tag: Some("INPUT".to_string()),
            is_content_editable: false,
        };

        assert_eq!(
            binding_from_shortcut_event_payload(&payload),
            Some(ShortcutBinding::new(ShortcutKey::Escape))
        );
    }

    #[test]
    fn shortcut_event_payload_detects_editable_targets() {
        let payload = ShortcutKeyEventPayload {
            key: "a".to_string(),
            ctrl: false,
            alt: false,
            shift: false,
            meta: false,
            repeat: false,
            target_tag: Some("INPUT".to_string()),
            is_content_editable: false,
        };

        assert!(payload.is_editable_target());
    }
}
