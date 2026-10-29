use anyhow::Context;

use super::{
  flatten_child_split_containers, move_container_within_tree,
  set_focused_descendant,
};
use crate::{models::Container, traits::CommonGetters, wm_state::WmState};

/// Moves a tiling window out of its tabbed split container, placing it
/// right after the container in the container's parent.
///
/// The tabbed container is dissolved if a single tab is left. No-op if
/// the window is not a tab.
pub fn unstack(
  container: &Container,
  state: &mut WmState,
) -> anyhow::Result<()> {
  let Some(stack) = container
    .parent()
    .and_then(|parent| parent.as_split().cloned())
    .filter(|split| split.is_tabbed())
  else {
    return Ok(());
  };

  let workspace = container.workspace().context("No workspace.")?;
  let target_parent = stack.parent().context("No parent.")?;

  move_container_within_tree(
    container,
    &target_parent,
    stack.index() + 1,
    state,
  )?;

  // Dissolve the stack if a single tab is left (e.g. H[T[1] 2] ->
  // H[1 2]).
  flatten_child_split_containers(&target_parent)?;

  set_focused_descendant(container, None);

  state
    .pending_sync
    .queue_container_to_redraw(workspace)
    .queue_focus_change();

  Ok(())
}
