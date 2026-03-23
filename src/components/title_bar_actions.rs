use crate::shortcuts::ShortcutSettings;

use super::commands::CommandId;

#[derive(Clone, Copy)]
pub(crate) struct MenuItemDef {
    pub(crate) label: &'static str,
    pub(crate) command_id: CommandId,
}

#[derive(Clone, Copy)]
pub(crate) struct MenuSectionDef {
    pub(crate) title: &'static str,
    pub(crate) items: &'static [MenuItemDef],
}

const FILE_MENU: [MenuItemDef; 4] = [
    MenuItemDef {
        label: "Open Assembly",
        command_id: CommandId::OpenAssembly,
    },
    MenuItemDef {
        label: "Export Project",
        command_id: CommandId::ExportProject,
    },
    MenuItemDef {
        label: "Open Export Folder",
        command_id: CommandId::OpenExportFolder,
    },
    MenuItemDef {
        label: "Clear Workspace",
        command_id: CommandId::ClearWorkspace,
    },
];

const EDIT_MENU: [MenuItemDef; 1] = [MenuItemDef {
    label: "Run Analysis",
    command_id: CommandId::RunAnalysis,
}];

const VIEW_MENU: [MenuItemDef; 1] = [MenuItemDef {
    label: "Toggle Findings Panel",
    command_id: CommandId::ToggleFindings,
}];

const TOOLS_MENU: [MenuItemDef; 2] = [
    MenuItemDef {
        label: "Search Command Palette",
        command_id: CommandId::OpenCommandPalette,
    },
    MenuItemDef {
        label: "Settings",
        command_id: CommandId::OpenSettings,
    },
];

pub(crate) const MENUS: [MenuSectionDef; 4] = [
    MenuSectionDef {
        title: "File",
        items: &FILE_MENU,
    },
    MenuSectionDef {
        title: "Edit",
        items: &EDIT_MENU,
    },
    MenuSectionDef {
        title: "View",
        items: &VIEW_MENU,
    },
    MenuSectionDef {
        title: "Tools",
        items: &TOOLS_MENU,
    },
];

pub(crate) fn command_shortcut_label(
    shortcuts: &ShortcutSettings,
    command_id: CommandId,
) -> Option<String> {
    shortcuts
        .binding_for(command_id)
        .map(|binding| binding.display_label())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shortcuts::{ShortcutBinding, ShortcutKey, ShortcutSettings};

    #[test]
    fn shortcut_lookup_returns_none_when_command_is_unbound() {
        let shortcuts = ShortcutSettings::default();

        assert_eq!(
            command_shortcut_label(&shortcuts, CommandId::RunAnalysis),
            None
        );
    }

    #[test]
    fn shortcut_lookup_uses_current_binding() {
        let mut shortcuts = ShortcutSettings::with_defaults();
        shortcuts.set_binding(
            CommandId::OpenAssembly,
            ShortcutBinding::new(ShortcutKey::Character("p".to_string())).with_ctrl(),
        );

        assert_eq!(
            command_shortcut_label(&shortcuts, CommandId::OpenAssembly),
            Some("Ctrl+P".to_string())
        );
    }
}
