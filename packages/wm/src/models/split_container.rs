use std::{
  cell::{Ref, RefCell, RefMut},
  collections::VecDeque,
  rc::Rc,
};

use anyhow::Context;
use uuid::Uuid;
use wm_common::{
  ContainerDto, GapsConfig, SplitContainerDto, TilingDirection,
  TilingLayout,
};
use wm_platform::Rect;

use crate::{
  impl_common_getters, impl_container_debug,
  impl_position_getters_as_resizable, impl_tiling_size_getters,
  models::{
    Container, DirectionContainer, TilingContainer, WindowContainer,
  },
  traits::{
    CommonGetters, PositionGetters, TilingDirectionGetters,
    TilingSizeGetters,
  },
};

#[derive(Clone)]
pub struct SplitContainer(Rc<RefCell<SplitContainerInner>>);

struct SplitContainerInner {
  id: Uuid,
  parent: Option<Container>,
  children: VecDeque<Container>,
  child_focus_order: VecDeque<Uuid>,
  tiling_size: f32,
  tiling_direction: TilingDirection,
  tiling_layout: TilingLayout,
  gaps_config: GapsConfig,
}

impl SplitContainer {
  pub fn new(
    tiling_direction: TilingDirection,
    gaps_config: GapsConfig,
  ) -> Self {
    let split = SplitContainerInner {
      id: Uuid::new_v4(),
      parent: None,
      children: VecDeque::new(),
      child_focus_order: VecDeque::new(),
      tiling_size: 1.0,
      tiling_direction,
      tiling_layout: TilingLayout::Tiled,
      gaps_config,
    };

    Self(Rc::new(RefCell::new(split)))
  }

  /// Whether the container's children are shown as tabs.
  pub fn is_tabbed(&self) -> bool {
    self.tiling_layout() == TilingLayout::Tabbed
  }

  /// Sets whether children are tiled side-by-side or shown as tabs.
  pub fn set_tiling_layout(&self, tiling_layout: TilingLayout) {
    self.0.borrow_mut().tiling_layout = tiling_layout;
  }

  /// Child that is shown when the container is tabbed.
  ///
  /// The active tab is the most recently focused tiling child, so focus
  /// changes and tab switches are always in sync.
  ///
  /// Returns `None` if the container is tiled.
  pub fn active_tab(&self) -> Option<TilingContainer> {
    if !self.is_tabbed() {
      return None;
    }

    self
      .child_focus_order()
      .find_map(|child| child.as_tiling_container().ok())
      .or_else(|| self.tiling_children().next())
  }

  pub fn to_dto(&self) -> anyhow::Result<ContainerDto> {
    let rect = self.to_rect()?;
    let children = self
      .children()
      .iter()
      .map(CommonGetters::to_dto)
      .try_collect()?;

    Ok(ContainerDto::Split(SplitContainerDto {
      id: self.id(),
      parent_id: self.parent().map(|parent| parent.id()),
      children,
      child_focus_order: self.0.borrow().child_focus_order.clone().into(),
      has_focus: self.has_focus(None),
      tiling_size: self.tiling_size(),
      tiling_direction: self.tiling_direction(),
      layout: self.tiling_layout(),
      active_child_id: self.active_tab().map(|child| child.id()),
      width: rect.width(),
      height: rect.height(),
      x: rect.x(),
      y: rect.y(),
    }))
  }
}

impl_container_debug!(SplitContainer);
impl_common_getters!(SplitContainer);
impl_tiling_size_getters!(SplitContainer);
impl_position_getters_as_resizable!(SplitContainer);

impl TilingDirectionGetters for SplitContainer {
  fn tiling_direction(&self) -> TilingDirection {
    self.0.borrow().tiling_direction.clone()
  }

  fn set_tiling_direction(&self, tiling_direction: TilingDirection) {
    self.0.borrow_mut().tiling_direction = tiling_direction;
  }

  fn tiling_layout(&self) -> TilingLayout {
    self.0.borrow().tiling_layout.clone()
  }
}
