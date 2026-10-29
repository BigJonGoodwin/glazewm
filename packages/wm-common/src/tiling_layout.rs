use serde::{Deserialize, Serialize};

/// Layout of a split container's children.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TilingLayout {
  /// Children are laid out side-by-side in the container's tiling
  /// direction.
  #[default]
  Tiled,
  /// Children share the container's full rect and only the active child
  /// (i.e. the most recently focused one) is shown.
  Tabbed,
}

impl TilingLayout {
  /// Gets the opposite layout.
  ///
  /// Example:
  /// ```
  /// # use wm_common::TilingLayout;
  /// assert_eq!(TilingLayout::Tiled.toggled(), TilingLayout::Tabbed);
  /// ```
  #[must_use]
  pub fn toggled(&self) -> Self {
    match self {
      Self::Tiled => Self::Tabbed,
      Self::Tabbed => Self::Tiled,
    }
  }
}
