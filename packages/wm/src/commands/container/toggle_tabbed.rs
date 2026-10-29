use anyhow::Context;
use wm_common::{GapsConfig, TilingLayout, WmEvent};

use super::{flatten_child_split_containers, wrap_in_split_container};
use crate::{
  models::{Container, DirectionContainer, SplitContainer},
  traits::{CommonGetters, TilingDirectionGetters},
  wm_state::WmState,
};

/// Toggles the layout of a tiling window's parent split container between
/// tiled and tabbed.
///
/// If the window is a direct child of its workspace, the workspace's
/// tiling children are wrapped in a new tabbed split container. Untabbing
/// keeps the split container's tiling direction.
///
/// No-op for non-tiling windows, workspaces, and lone tiling windows.
pub fn toggle_tabbed(
  container: &Container,
  state: &mut WmState,
  gaps_config: &GapsConfig,
) -> anyhow::Result<()> {
  let Container::TilingWindow(window) = container else {
    return Ok(());
  };

  let workspace = window.workspace().context("No workspace.")?;

  let split_container = match window
    .direction_container()
    .context("No direction container.")?
  {
    DirectionContainer::Split(split_container) => {
      split_container
        .set_tiling_layout(split_container.tiling_layout().toggled());

      split_container
    }
    DirectionContainer::Workspace(workspace) => {
      let tiling_children = workspace.tiling_children().collect::<Vec<_>>();

      // A single window cannot be stacked.
      if tiling_children.len() < 2 {
        return Ok(());
      }

      let split_container = SplitContainer::new(
        workspace.tiling_direction(),
        gaps_config.clone(),
      );

      split_container.set_tiling_layout(TilingLayout::Tabbed);

      wrap_in_split_container(
        &split_container,
        &workspace.into(),
        &tiling_children,
      )?;

      split_container
    }
  };

  state.emit_event(WmEvent::TilingLayoutChanged {
    split_container: split_container.to_dto()?,
    new_layout: split_container.tiling_layout(),
  });

  // An untabbed container can be redundant with its parent. For example,
  // H[1 T(horizontal)[2 3]] becomes H[1 H[2 3]], which is flattened to
  // H[1 2 3]. Likewise, a tabbed container that is the only child of a
  // split container is merged into it.
  if let Some(parent) = split_container.parent() {
    flatten_child_split_containers(&parent)?;
  }

  state.pending_sync.queue_container_to_redraw(workspace);

  Ok(())
}
