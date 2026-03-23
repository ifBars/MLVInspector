/// Left explorer panel: assembly switcher plus namespace/type/method browser.
use std::collections::BTreeSet;

use dioxus::prelude::*;

use crate::state::AppState;

use super::explorer_assemblies::ExplorerAssemblySwitcher;
use super::explorer_metadata::has_metadata;
use super::explorer_tree::ExplorerTree;
use super::helpers::{extract_methods, group_types_by_namespace};
use super::theme::C_BG_BASE;
use super::view_models::IlTab;

#[component]
pub fn ExplorerPanel(
    sidebar_width: f64,
    open_tabs: Signal<Vec<IlTab>>,
    active_tab_id: Signal<Option<String>>,
    selected_finding: Signal<Option<usize>>,
) -> Element {
    let state = use_context::<AppState>();

    let expanded_assemblies = use_signal(BTreeSet::<String>::new);
    let expanded_namespaces = use_signal(BTreeSet::<String>::new);
    let expanded_types = use_signal(BTreeSet::<String>::new);

    let selected_id = state.selected_id.read().clone();
    let assemblies = state.assemblies.read().clone();

    let (methods, grouped_types, assembly_metadata) = selected_id
        .as_ref()
        .and_then(|id| {
            let explore_key = format!("{id}::explore");
            state.with_analysis_result(&explore_key, |result| {
                let metadata = result
                    .explore
                    .as_ref()
                    .map(|payload| payload.assembly_metadata.clone())
                    .unwrap_or_default();
                let methods = extract_methods(result);
                let grouped_types = result
                    .explore
                    .as_ref()
                    .map(|payload| group_types_by_namespace(&payload.types, &methods))
                    .unwrap_or_default();
                (methods, grouped_types, metadata)
            })
        })
        .unwrap_or_default();

    let type_count = grouped_types.iter().map(|ns| ns.types.len()).sum::<usize>();
    let namespace_count = grouped_types.len();
    let methods_count = methods.len();

    let active_tab = {
        let id = active_tab_id.read().clone();
        id.and_then(|tab_id| {
            open_tabs
                .read()
                .iter()
                .find(|tab| tab.id == tab_id)
                .cloned()
        })
    };
    let selected_method_name = active_tab.as_ref().and_then(|tab| {
        tab.method_name
            .as_ref()
            .map(|method_name| format!("{}::{}", tab.type_name, method_name))
    });
    let selected_type_name = active_tab.as_ref().map(|tab| tab.type_name.clone());

    let selected_assembly = selected_id
        .as_ref()
        .and_then(|id| assemblies.iter().find(|asm| asm.id == *id))
        .cloned();

    let assemblies_count = assemblies.len();
    let metadata_available = has_metadata(&assembly_metadata);

    rsx! {
        div {
            style: format!(
                "width: {sidebar_width:.0}px; flex-shrink: 0; display: flex; flex-direction: column; background: {C_BG_BASE};"
            ),

            div {
                class: "panel-header",
                span { "Explorer" }
                span {
                    class: "badge",
                    "{assemblies_count} asm / {type_count} types / {namespace_count} ns"
                }
            }

            ExplorerAssemblySwitcher {
                assemblies: assemblies.clone(),
                selected_id: selected_id.clone(),
                open_tabs,
                active_tab_id,
                selected_finding,
            }

            div {
                style: "flex: 1; overflow-y: auto; padding: 8px 0 10px;",

                if let Some(selected_assembly) = selected_assembly {
                    ExplorerTree {
                        selected_assembly,
                        has_metadata: metadata_available,
                        grouped_types,
                        namespace_count,
                        type_count,
                        methods_count,
                        selected_type_name,
                        selected_method_name,
                        expanded_assemblies,
                        expanded_namespaces,
                        expanded_types,
                        open_tabs,
                        active_tab_id,
                        selected_finding,
                    }
                } else {
                    div {
                        class: "empty-state",
                        p { "Select an assembly to browse namespaces, types, and methods" }
                    }
                }
            }
        }
    }
}
