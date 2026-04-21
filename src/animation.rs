#[cfg(feature = "animations")]
use tachyonfx::{Effect, Interpolation, Shader, fx};

pub struct State {
  #[cfg(feature = "animations")]
  active_effect:     Option<Effect>,
  #[cfg(feature = "animations")]
  last_render_time:  std::time::Instant,
  #[cfg(feature = "animations")]
  startup_triggered: bool,
}

impl State {
  pub fn new() -> Self {
    Self {
      #[cfg(feature = "animations")]
      active_effect:     None,
      #[cfg(feature = "animations")]
      last_render_time:  std::time::Instant::now(),
      #[cfg(feature = "animations")]
      startup_triggered: false,
    }
  }

  /// Advance animation state for one frame and return the elapsed duration.
  #[cfg(feature = "animations")]
  pub fn tick(&mut self, enabled: bool, startup_duration_ms: u32) -> std::time::Duration {
    let now = std::time::Instant::now();
    let elapsed = now.duration_since(self.last_render_time);
    self.last_render_time = now;

    #[cfg(not(test))]
    if !self.startup_triggered {
      if enabled {
        self.active_effect = Some(startup(startup_duration_ms));
      }
      self.startup_triggered = true;
    }

    // Drop finished effects.
    if self.active_effect.as_ref().map_or(false, |e| !e.running()) {
      self.active_effect = None;
    }

    elapsed
  }

  #[cfg(feature = "animations")]
  pub fn active_effect(&mut self) -> Option<&mut Effect> {
    self.active_effect.as_mut()
  }
}

#[cfg(feature = "animations")]
fn startup(duration_ms: u32) -> Effect {
  fx::coalesce((duration_ms, Interpolation::SineOut))
}
