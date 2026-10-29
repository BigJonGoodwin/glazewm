use anyhow::Context;
use wm_common::{GapsConfig, TilingDirection, TilingLayout, WmEvent};
use wm_platform::Direction;

use super::{
  move_container_within_tree, set_focused_descendant, tiling_focus_target,
  wrap_in_split_container,
};
use crate::{
  models::{Container, SplitContainer},
  traits::{CommonGetters, TilingDirectionGetters},
  wm_state::WmState,
};

/// Moves a tiling window into the tiling window in the given direction,
/// forming a tabbed split container with it (or joining its tabbed split
/// container if it already has one).
///
/// The moved window becomes the active tab. No-op for non-tiling windows
/// and if there is no tiling window in the given direction within the
/// workspace.
pub fn stack_in_direction(
  container: &Container,
  direction: &Direction,
  state: &mut WmState,
  gaps_config: &GapsConfig,
) -> anyhow::Result<()> {
  if !container.is_tiling_window() {
    return Ok(());
  }

  let Some(target) = tiling_focus_target(container, direction)? else {
    return Ok(());
  };

  let workspace = container.workspace().context("No workspace.")?;
  let target_parent = target.parent().context("No parent.")?;

  let existing_stack = target_parent
    .as_split()
    .filter(|split| split.is_tabbed())
    .cloned();

  let is_new_stack = existing_stack.is_none();

  let stack = if let Some(stack) = existing_stack {
    stack
  } else {
    // Tab order follows the direction the windows were laid out in, so
    // that unstacking restores a similar layout.
    let stack = SplitContainer::new(
      TilingDirection::from_direction(direction),
      gaps_config.clone(),
    );

    stack.set_tiling_layout(TilingLayout::Tabbed);

    wrap_in_split_container(
      &stack,
      &target_parent,
      &[target.as_tiling_container()?],
    )?;

    stack
  };

  let target_index = match direction {
    Direction::Left | Direction::Up => target.index(),
    Direction::Right | Direction::Down => target.index() + 1,
  };

  move_container_within_tree(
    container,
    &stack.clone().into(),
    target_index,
    state,
  )?;

  set_focused_descendant(container, None);

  // The stack might have been merged into its parent split container
  // while flattening (e.g. H[T[1 2]] -> T[1 2]).
  let stack = container
    .parent()
    .and_then(|parent| parent.as_split().cloned())
    .context("Stacked window has no split container parent.")?;

  if is_new_stack {
    state.emit_event(WmEvent::TilingLayoutChanged {
      split_container: stack.to_dto()?,
      new_layout: stack.tiling_layout(),
    });
  }

  state
    .pending_sync
    .queue_container_to_redraw(workspace)
    .queue_focus_change();

  Ok(())
}
