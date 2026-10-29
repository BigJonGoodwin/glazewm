use tracing::info;
use wm_common::{try_warn, WindowRuleEvent};
use wm_platform::NativeWindow;

use crate::{
  commands::{
    general::emit_tabbed_containers_changed, window::run_window_rules,
  },
  models::SplitContainer,
  traits::{CommonGetters, WindowGetters},
  user_config::UserConfig,
  wm_state::WmState,
};

pub fn handle_window_title_changed(
  native_window: &NativeWindow,
  state: &mut WmState,
  config: &mut UserConfig,
) -> anyhow::Result<()> {
  let found_window = state.window_from_native(native_window);

  if let Some(window) = found_window {
    info!("Window title changed: {window}");

    let title = try_warn!(window.native().title());

    window.update_native_properties(|properties| {
      properties.title = title;
    });

    // Tab titles are part of the tabbed containers event.
    if window.ancestors().any(|ancestor| {
      ancestor.as_split().is_some_and(SplitContainer::is_tabbed)
    }) {
      emit_tabbed_containers_changed(state)?;
    }

    // Run window rules for title change events.
    run_window_rules(
      window,
      &WindowRuleEvent::TitleChange,
      state,
      config,
    )?;
  }

  Ok(())
}
