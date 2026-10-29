//! Tests for the tabbed layout of split containers and its commands.

use clap::Parser;
use tokio::sync::mpsc;
use wm_common::{
  ContainerDto, GapsConfig, InvokeCommand, TilingDirection, TilingLayout,
  WmEvent,
};
use wm_platform::{Direction, Dispatcher, LengthValue};

use super::{
  attach_container, detach_container, flatten_child_split_containers,
  focus_tab, set_focused_descendant, stack_in_direction,
  tiling_focus_target, toggle_tabbed, unstack, TabTarget,
};
use crate::{
  commands::general::{
    displayed_tabbed_containers, queue_active_tab_redraws,
  },
  models::{
    Container, Monitor, SplitContainer, TilingContainer, TilingWindow,
    Workspace,
  },
  traits::{
    CommonGetters, PositionGetters, TilingDirectionGetters,
    TilingSizeGetters, WindowGetters,
  },
  wm_state::WmState,
};

/// Creates a `WmState` with a single monitor displaying the workspace.
fn mock_state(workspace: &Workspace) -> WmState {
  let (event_tx, _) = mpsc::unbounded_channel();
  let (exit_tx, _) = mpsc::unbounded_channel();
  let state = WmState::new(Dispatcher::mock(), event_tx, exit_tx);

  let monitor = Monitor::mock().workspaces(vec![workspace.clone()]).call();
  attach_container(
    &monitor.into(),
    &state.root_container.clone().into(),
    None,
  )
  .unwrap();

  state
}

/// Creates a tiling window with the given title.
fn window(title: &str) -> TilingWindow {
  TilingWindow::mock().title(title.to_string()).call()
}

/// Creates a horizontal workspace with the given tiling windows.
fn workspace_with(windows: &[&TilingWindow]) -> Workspace {
  Workspace::mock()
    .tiling_containers(
      windows.iter().map(|w| (*w).clone().into()).collect(),
    )
    .call()
}

/// Focuses the given window.
fn focus(window: &TilingWindow) {
  set_focused_descendant(&window.clone().into(), None);
}

/// Gets the titles of a container's tiling children, with nested splits
/// written as `[...]`.
fn layout(container: &Container) -> String {
  container
    .tiling_children()
    .map(|child| match child {
      TilingContainer::TilingWindow(window) => {
        window.native_properties().title
      }
      TilingContainer::Split(split) => {
        let prefix = if split.is_tabbed() { "T" } else { "S" };
        format!("{prefix}[{}]", layout(&split.into()))
      }
    })
    .collect::<Vec<_>>()
    .join(" ")
}

/// Gets the window's parent split container.
fn parent_split(window: &TilingWindow) -> SplitContainer {
  window.parent().unwrap().as_split().unwrap().clone()
}

#[test]
fn toggle_tabbed_wraps_workspace_children_and_back() {
  let (a, b, c) = (window("a"), window("b"), window("c"));
  let workspace = workspace_with(&[&a, &b, &c]);
  let mut state = mock_state(&workspace);
  focus(&b);

  toggle_tabbed(&b.clone().into(), &mut state, &GapsConfig::default())
    .unwrap();

  assert_eq!(layout(&workspace.clone().into()), "T[a b c]");
  let stack = parent_split(&b);
  assert_eq!(stack.tiling_layout(), TilingLayout::Tabbed);
  assert_eq!(stack.tiling_direction(), TilingDirection::Horizontal);
  assert_eq!(stack.active_tab().unwrap().id(), b.id());

  // All tabs share the stack's rect; only the active one is shown.
  let stack_rect = stack.to_rect().unwrap();
  for tab in [&a, &b, &c] {
    assert_eq!(tab.to_rect().unwrap(), stack_rect);
  }
  assert!(a.is_in_inactive_tab());
  assert!(!b.is_in_inactive_tab());
  assert!(c.is_in_inactive_tab());

  toggle_tabbed(&b.clone().into(), &mut state, &GapsConfig::default())
    .unwrap();

  assert_eq!(layout(&workspace.clone().into()), "a b c");
  assert_eq!(workspace.tiling_direction(), TilingDirection::Horizontal);
  assert!(!a.is_in_inactive_tab());
  assert_ne!(a.to_rect().unwrap(), b.to_rect().unwrap());
}

#[test]
fn toggle_tabbed_toggles_parent_split() {
  let (a, b, c) = (window("a"), window("b"), window("c"));
  let split = SplitContainer::mock()
    .tiling_direction(TilingDirection::Vertical)
    .tiling_containers(vec![b.clone().into(), c.clone().into()])
    .call();
  let workspace = Workspace::mock()
    .tiling_containers(vec![a.clone().into(), split.clone().into()])
    .call();
  let mut state = mock_state(&workspace);
  focus(&c);

  toggle_tabbed(&c.clone().into(), &mut state, &GapsConfig::default())
    .unwrap();

  assert_eq!(layout(&workspace.clone().into()), "a T[b c]");
  assert_eq!(split.tiling_direction(), TilingDirection::Vertical);
  assert_eq!(split.active_tab().unwrap().id(), c.id());

  toggle_tabbed(&c.clone().into(), &mut state, &GapsConfig::default())
    .unwrap();

  assert_eq!(layout(&workspace.into()), "a S[b c]");
  assert_eq!(split.tiling_direction(), TilingDirection::Vertical);
}

#[test]
fn toggle_tabbed_ignores_lone_window() {
  let a = window("a");
  let workspace = workspace_with(&[&a]);
  let mut state = mock_state(&workspace);
  focus(&a);

  toggle_tabbed(&a.clone().into(), &mut state, &GapsConfig::default())
    .unwrap();

  assert_eq!(layout(&workspace.into()), "a");
}

#[test]
fn stack_creates_and_joins_stack() {
  let (a, b, c) = (window("a"), window("b"), window("c"));
  let workspace = workspace_with(&[&a, &b, &c]);
  let mut state = mock_state(&workspace);
  focus(&a);

  stack_in_direction(
    &a.clone().into(),
    &Direction::Right,
    &mut state,
    &GapsConfig::default(),
  )
  .unwrap();

  assert_eq!(layout(&workspace.clone().into()), "T[b a] c");
  assert_eq!(parent_split(&a).active_tab().unwrap().id(), a.id());

  // Stacking the next window left joins the existing stack, before its
  // active tab.
  focus(&c);
  stack_in_direction(
    &c.clone().into(),
    &Direction::Left,
    &mut state,
    &GapsConfig::default(),
  )
  .unwrap();

  assert_eq!(layout(&workspace.clone().into()), "T[b c a]");
  let stack = parent_split(&c);
  assert_eq!(stack.active_tab().unwrap().id(), c.id());
  assert_eq!(state.focused_container().unwrap().id(), c.id());
}

#[test]
fn stack_without_neighbour_is_noop() {
  let (a, b) = (window("a"), window("b"));
  let workspace = workspace_with(&[&a, &b]);
  let mut state = mock_state(&workspace);
  focus(&a);

  stack_in_direction(
    &a.clone().into(),
    &Direction::Up,
    &mut state,
    &GapsConfig::default(),
  )
  .unwrap();

  assert_eq!(layout(&workspace.into()), "a b");
}

#[test]
fn unstack_moves_window_out_and_dissolves_stack() {
  let (a, b, c) = (window("a"), window("b"), window("c"));
  let workspace = workspace_with(&[&a, &b, &c]);
  let mut state = mock_state(&workspace);
  focus(&a);
  stack_in_direction(
    &a.clone().into(),
    &Direction::Right,
    &mut state,
    &GapsConfig::default(),
  )
  .unwrap();
  assert_eq!(layout(&workspace.clone().into()), "T[b a] c");

  unstack(&a.clone().into(), &mut state).unwrap();

  assert_eq!(layout(&workspace.clone().into()), "b a c");
  assert_eq!(state.focused_container().unwrap().id(), a.id());

  // Tiling sizes still add up to the whole workspace.
  let total = workspace
    .tiling_children()
    .map(|child| child.tiling_size())
    .sum::<f32>();
  assert!((total - 1.0).abs() < 0.001);

  // Unstacking a window that is not a tab does nothing.
  unstack(&c.clone().into(), &mut state).unwrap();
  assert_eq!(layout(&workspace.into()), "b a c");
}

#[test]
fn unstack_keeps_stack_with_remaining_tabs() {
  let (a, b, c) = (window("a"), window("b"), window("c"));
  let workspace = workspace_with(&[&a, &b, &c]);
  let mut state = mock_state(&workspace);
  focus(&a);
  toggle_tabbed(&a.clone().into(), &mut state, &GapsConfig::default())
    .unwrap();

  unstack(&b.clone().into(), &mut state).unwrap();

  assert_eq!(layout(&workspace.into()), "T[a c] b");
}

#[test]
fn focus_tab_cycles_and_wraps() {
  let (a, b, c) = (window("a"), window("b"), window("c"));
  let workspace = workspace_with(&[&a, &b, &c]);
  let mut state = mock_state(&workspace);
  focus(&a);
  toggle_tabbed(&a.clone().into(), &mut state, &GapsConfig::default())
    .unwrap();
  let stack = parent_split(&a);

  focus_tab(&a.clone().into(), TabTarget::Next, &mut state);
  assert_eq!(stack.active_tab().unwrap().id(), b.id());
  assert_eq!(state.focused_container().unwrap().id(), b.id());

  focus_tab(&b.clone().into(), TabTarget::Next, &mut state);
  focus_tab(&c.clone().into(), TabTarget::Next, &mut state);
  assert_eq!(stack.active_tab().unwrap().id(), a.id());

  focus_tab(&a.clone().into(), TabTarget::Previous, &mut state);
  assert_eq!(stack.active_tab().unwrap().id(), c.id());
  assert!(state.pending_sync.needs_focus_update());
}

#[test]
fn focus_tab_outside_stack_is_noop() {
  let (a, b) = (window("a"), window("b"));
  let workspace = workspace_with(&[&a, &b]);
  let mut state = mock_state(&workspace);
  focus(&a);

  focus_tab(&a.clone().into(), TabTarget::Next, &mut state);

  assert_eq!(state.focused_container().unwrap().id(), a.id());
  assert!(!state.pending_sync.needs_focus_update());
}

#[test]
fn focus_in_direction_enters_stack_at_active_tab() {
  let (a, b, c, d) = (window("a"), window("b"), window("c"), window("d"));
  let stack = SplitContainer::mock()
    .tiling_containers(vec![a.clone().into(), b.clone().into()])
    .call();
  stack.set_tiling_layout(TilingLayout::Tabbed);
  let workspace = Workspace::mock()
    .tiling_containers(vec![
      c.clone().into(),
      stack.clone().into(),
      d.clone().into(),
    ])
    .call();
  let _state = mock_state(&workspace);
  focus(&b);
  focus(&c);

  for (origin, direction) in
    [(&c, Direction::Right), (&d, Direction::Left)]
  {
    let target =
      tiling_focus_target(&origin.clone().into(), &direction).unwrap();
    assert_eq!(target.unwrap().id(), b.id());
  }
}

#[test]
fn closing_active_tab_activates_previous_tab() {
  let (a, b, c) = (window("a"), window("b"), window("c"));
  let workspace = workspace_with(&[&a, &b, &c]);
  let mut state = mock_state(&workspace);
  focus(&c);
  focus(&a);
  toggle_tabbed(&a.clone().into(), &mut state, &GapsConfig::default())
    .unwrap();
  let stack = parent_split(&a);

  // Same steps as `unmanage_window`.
  detach_container(a.clone().into()).unwrap();
  flatten_child_split_containers(&workspace.clone().into()).unwrap();

  assert_eq!(layout(&workspace.clone().into()), "T[b c]");
  assert_eq!(stack.active_tab().unwrap().id(), c.id());

  // A stack becoming one window dissolves.
  detach_container(c.clone().into()).unwrap();
  flatten_child_split_containers(&workspace.clone().into()).unwrap();

  assert_eq!(layout(&workspace.into()), "b");
  assert!(!b.is_in_inactive_tab());
}

#[test]
fn tabbed_split_is_not_merged_into_same_direction_parent() {
  let (a, b, c) = (window("a"), window("b"), window("c"));
  let stack = SplitContainer::mock()
    .tiling_containers(vec![b.clone().into(), c.clone().into()])
    .call();
  stack.set_tiling_layout(TilingLayout::Tabbed);
  let workspace = Workspace::mock()
    .tiling_containers(vec![a.clone().into(), stack.into()])
    .call();
  let _state = mock_state(&workspace);

  flatten_child_split_containers(&workspace.clone().into()).unwrap();

  assert_eq!(layout(&workspace.into()), "a T[b c]");
}

#[test]
fn lone_tabbed_child_is_merged_into_parent_split() {
  let (a, b, c) = (window("a"), window("b"), window("c"));
  let stack = SplitContainer::mock()
    .tiling_direction(TilingDirection::Vertical)
    .tiling_containers(vec![b.clone().into(), c.clone().into()])
    .call();
  stack.set_tiling_layout(TilingLayout::Tabbed);
  let outer = SplitContainer::mock()
    .tiling_containers(vec![stack.into()])
    .call();
  let workspace = Workspace::mock()
    .tiling_direction(TilingDirection::Vertical)
    .tiling_containers(vec![a.clone().into(), outer.clone().into()])
    .call();
  let _state = mock_state(&workspace);

  flatten_child_split_containers(&outer.clone().into()).unwrap();

  assert_eq!(layout(&workspace.into()), "a T[b c]");
  assert!(outer.is_tabbed());
  assert_eq!(outer.tiling_direction(), TilingDirection::Vertical);
}

#[test]
fn tab_bar_height_insets_tabs() {
  let (a, b) = (window("a"), window("b"));
  let gaps_config = GapsConfig {
    tab_bar_height: LengthValue::from_px(26),
    ..GapsConfig::default()
  };
  for tab in [&a, &b] {
    tab.set_gaps_config(gaps_config.clone());
  }
  let workspace = workspace_with(&[&a, &b]);
  let mut state = mock_state(&workspace);
  focus(&a);
  toggle_tabbed(&a.clone().into(), &mut state, &gaps_config).unwrap();

  let stack_rect = parent_split(&a).to_rect().unwrap();
  let tab_rect = a.to_rect().unwrap();

  assert_eq!(tab_rect.x(), stack_rect.x());
  assert_eq!(tab_rect.y(), stack_rect.y() + 26);
  assert_eq!(tab_rect.width(), stack_rect.width());
  assert_eq!(tab_rect.height(), stack_rect.height() - 26);
}

#[test]
fn active_tab_change_queues_redraw() {
  let (a, b) = (window("a"), window("b"));
  let workspace = workspace_with(&[&a, &b]);
  let mut state = mock_state(&workspace);
  focus(&a);
  toggle_tabbed(&a.clone().into(), &mut state, &GapsConfig::default())
    .unwrap();
  let stack = parent_split(&a);

  queue_active_tab_redraws(&mut state);
  state.pending_sync.clear();

  // Unchanged active tab: nothing to redraw.
  queue_active_tab_redraws(&mut state);
  assert!(state.pending_sync.containers_to_redraw().is_empty());

  focus(&b);
  queue_active_tab_redraws(&mut state);
  assert!(state
    .pending_sync
    .containers_to_redraw()
    .contains_key(&stack.id()));
}

#[test]
fn ipc_dto_and_event_include_layout() {
  let (a, b) = (
    TilingWindow::mock()
      .title("Terminal".to_string())
      .process_name("WindowsTerminal".to_string())
      .call(),
    TilingWindow::mock()
      .title("Notes".to_string())
      .process_name("notepad".to_string())
      .call(),
  );
  let workspace = workspace_with(&[&a, &b]);
  let mut state = mock_state(&workspace);
  focus(&b);
  toggle_tabbed(&b.clone().into(), &mut state, &GapsConfig::default())
    .unwrap();
  let stack = parent_split(&b);

  let tabbed_containers = displayed_tabbed_containers(&state).unwrap();
  assert_eq!(tabbed_containers.len(), 1);

  let ContainerDto::Split(dto) = &tabbed_containers[0] else {
    panic!("Expected a split container DTO.");
  };
  assert_eq!(dto.id, stack.id());
  assert_eq!(dto.layout, TilingLayout::Tabbed);
  assert_eq!(dto.active_child_id, Some(b.id()));

  let event = WmEvent::TabbedContainersChanged { tabbed_containers };
  let json = serde_json::to_value(&event).unwrap();
  assert_eq!(json["eventType"], "tabbed_containers_changed");
  let split_json = &json["tabbedContainers"][0];
  assert_eq!(split_json["type"], "split");
  assert_eq!(split_json["layout"], "tabbed");
  assert_eq!(split_json["activeChildId"], b.id().to_string());
  println!("{}", serde_json::to_string_pretty(&event).unwrap());

  let event = WmEvent::TilingLayoutChanged {
    split_container: stack.to_dto().unwrap(),
    new_layout: TilingLayout::Tabbed,
  };
  let json = serde_json::to_value(&event).unwrap();
  assert_eq!(json["eventType"], "tiling_layout_changed");
  assert_eq!(json["newLayout"], "tabbed");

  // Tiled split containers report no active child.
  toggle_tabbed(&b.clone().into(), &mut state, &GapsConfig::default())
    .unwrap();
  assert!(displayed_tabbed_containers(&state).unwrap().is_empty());
}

#[test]
fn parses_tab_commands() {
  let parse = |args: &str| {
    InvokeCommand::try_parse_from(
      std::iter::once("").chain(args.split_whitespace()),
    )
  };

  assert_eq!(parse("toggle-tabbed").unwrap(), InvokeCommand::ToggleTabbed);
  assert_eq!(parse("unstack").unwrap(), InvokeCommand::Unstack);
  assert_eq!(
    parse("stack --direction left").unwrap(),
    InvokeCommand::Stack {
      direction: Direction::Left
    }
  );
  assert!(matches!(
    parse("focus --next-tab").unwrap(),
    InvokeCommand::Focus(args) if args.next_tab && !args.prev_tab
  ));
  assert!(matches!(
    parse("focus --prev-tab").unwrap(),
    InvokeCommand::Focus(args) if args.prev_tab
  ));
  assert!(parse("focus --next-tab --prev-tab").is_err());
}
