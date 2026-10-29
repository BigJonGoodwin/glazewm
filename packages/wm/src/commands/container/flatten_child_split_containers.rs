use super::flatten_split_container;
use crate::{
  models::{Container, DirectionContainer, SplitContainer},
  traits::{CommonGetters, TilingDirectionGetters},
};

/// Flattens any redundant split containers at the top-level of the given
/// parent container.
///
/// For example:
/// ```ignore,compile_fail
/// H[1 H[V[2, 3]]] -> H[1, 2, 3]
/// H[1 H[2, 3]] -> H[1, 2, 3]
/// H[V[1]] -> V[1]
/// ```
///
/// Tabbed split containers (`T`) are only flattened once they have a
/// single child, or when they are the only child of a split container
/// (which then becomes tabbed instead):
/// ```ignore,compile_fail
/// H[1 T[2]] -> H[1, 2]
/// H[1 H[T[2, 3]]] -> H[1, T[2, 3]]
/// H[T[1, 2]] -> T[1, 2]
/// ```
pub fn flatten_child_split_containers(
  parent: &Container,
) -> anyhow::Result<()> {
  if let Ok(parent) = parent.as_direction_container() {
    // Get children that are either tiling windows or split containers.
    let tiling_children = parent
      .children()
      .into_iter()
      .filter(|child| child.is_tiling_window() || child.is_split())
      .collect::<Vec<_>>();

    if tiling_children.len() == 1 {
      // Handle case where the parent is a split container and has a
      // single split container child.
      if let Some(split_child) = tiling_children[0].as_split() {
        if split_child.is_tabbed() || is_tabbed(&parent) {
          flatten_only_child_with_layout(&parent, split_child)?;
        } else {
          // Take over the child's direction. This is usually the inverse
          // of the parent's, but not after untabbing (e.g. H[T(h)[1 2]]
          // -> H[H[1 2]] -> H[1 2]).
          let tiling_direction = split_child.tiling_direction();
          flatten_split_container(split_child.clone())?;
          parent.set_tiling_direction(tiling_direction);
        }
      }
    } else {
      let split_children = tiling_children
        .into_iter()
        .filter_map(|child| child.as_split().cloned())
        .collect::<Vec<_>>();

      for split_child in split_children {
        // A tabbed container with a single tab is no longer a stack.
        if split_child.is_tabbed() && split_child.child_count() == 1 {
          flatten_split_container(split_child)?;
          continue;
        }

        // Tiled split containers with the same tiling direction as their
        // (tiled) parent are redundant.
        let is_redundant = !split_child.is_tabbed()
          && !is_tabbed(&parent)
          && split_child.tiling_direction() == parent.tiling_direction();

        if !is_redundant {
          continue;
        }

        // Additionally flatten redundant top-level split containers in
        // the child.
        if split_child.child_count() == 1 {
          if let Some(split_grandchild) =
            split_child.children()[0].as_split()
          {
            if !split_grandchild.is_tabbed() {
              flatten_split_container(split_grandchild.clone())?;
            }
          }
        }

        flatten_split_container(split_child)?;
      }
    }
  }

  Ok(())
}

/// Whether the direction container is a tabbed split container.
fn is_tabbed(container: &DirectionContainer) -> bool {
  matches!(container, DirectionContainer::Split(split) if split.is_tabbed())
}

/// Flattens the only tiling child of `parent` when either of them is
/// tabbed.
///
/// A split container parent takes over the child's layout and tiling
/// direction. A workspace parent keeps a tabbed child, since workspaces
/// cannot be tabbed themselves.
fn flatten_only_child_with_layout(
  parent: &DirectionContainer,
  split_child: &SplitContainer,
) -> anyhow::Result<()> {
  // A container with a single child has no layout to keep.
  if split_child.child_count() == 1 {
    return flatten_split_container(split_child.clone());
  }

  match parent {
    DirectionContainer::Split(split_parent) => {
      split_parent.set_tiling_layout(split_child.tiling_layout());
      split_parent.set_tiling_direction(split_child.tiling_direction());
      flatten_split_container(split_child.clone())
    }
    DirectionContainer::Workspace(_) => Ok(()),
  }
}
