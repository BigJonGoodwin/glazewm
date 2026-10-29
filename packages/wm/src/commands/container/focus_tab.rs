use super::set_focused_descendant;
use crate::{
  models::{Container, SplitContainer},
  traits::CommonGetters,
  wm_state::WmState,
};

/// Tab to switch to relative to the active tab.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TabTarget {
  Next,
  Previous,
}

/// Focuses the next or previous tab of the closest tabbed split container
/// that contains the given container. Wraps around at either end.
///
/// If the tab is itself a split container, its most recently focused
/// descendant is focused. No-op if the container is not within a tabbed
/// split container.
pub fn focus_tab(
  container: &Container,
  target: TabTarget,
  state: &mut WmState,
) {
  // Find the closest tabbed ancestor, and its child that contains the
  // given container (i.e. the current tab).
  let Some((stack, current_tab)) =
    container.self_and_ancestors().find_map(|ancestor| {
      ancestor
        .parent()
        .and_then(|parent| parent.as_split().cloned())
        .filter(SplitContainer::is_tabbed)
        .map(|split| (split, ancestor))
    })
  else {
    return;
  };

  let tabs = stack.tiling_children().collect::<Vec<_>>();

  let Some(current_index) =
    tabs.iter().position(|tab| tab.id() == current_tab.id())
  else {
    return;
  };

  let target_index = match target {
    TabTarget::Next => (current_index + 1) % tabs.len(),
    TabTarget::Previous => (current_index + tabs.len() - 1) % tabs.len(),
  };

  let target_tab: Container = tabs[target_index].clone().into();

  let focus_target = target_tab
    .descendant_focus_order()
    .find(Container::is_tiling_window)
    .unwrap_or_else(|| target_tab.clone());

  set_focused_descendant(&focus_target, None);
  state.pending_sync.queue_focus_change();
}
