use std::collections::HashMap;

use wm_common::{ContainerDto, WmEvent};

use crate::{traits::CommonGetters, wm_state::WmState};

/// Queues a redraw of every tabbed split container whose active tab has
/// changed since the last call.
///
/// The active tab follows focus, so this catches every way it can change
/// (focus commands, closing or moving out the active tab, etc.) without
/// each command having to track it.
pub fn queue_active_tab_redraws(state: &mut WmState) {
  let active_tabs = state
    .tabbed_containers()
    .into_iter()
    .filter_map(|split| {
      let active_tab = split.active_tab()?;
      Some((split.id(), (split, active_tab.id())))
    })
    .collect::<HashMap<_, _>>();

  for (split_id, (split, active_tab_id)) in &active_tabs {
    if state.active_tabs.get(split_id) != Some(active_tab_id) {
      state.pending_sync.queue_container_to_redraw(split.clone());
    }
  }

  state.active_tabs = active_tabs
    .into_iter()
    .map(|(split_id, (_, active_tab_id))| (split_id, active_tab_id))
    .collect();
}

/// Gets all tabbed split containers on displayed workspaces.
pub fn displayed_tabbed_containers(
  state: &WmState,
) -> anyhow::Result<Vec<ContainerDto>> {
  state
    .tabbed_containers()
    .into_iter()
    .filter(|split| {
      split
        .workspace()
        .is_some_and(|workspace| workspace.is_displayed())
    })
    .map(|split| split.to_dto())
    .try_collect()
}

/// Emits `WmEvent::TabbedContainersChanged` if the tabbed containers on
/// displayed workspaces differ from the last emitted ones.
///
/// Containers are compared by their serialized DTOs, so any change that a
/// client could draw (active tab, children, titles, position) is caught.
pub fn emit_tabbed_containers_changed(
  state: &mut WmState,
) -> anyhow::Result<()> {
  let tabbed_containers = displayed_tabbed_containers(state)?;
  let snapshot = serde_json::to_string(&tabbed_containers)?;

  if state.tabbed_containers_snapshot != snapshot {
    state.tabbed_containers_snapshot = snapshot;
    state.emit_event(WmEvent::TabbedContainersChanged { tabbed_containers });
  }

  Ok(())
}
