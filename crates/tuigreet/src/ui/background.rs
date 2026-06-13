//! Menu item type for the background animation switcher.

use std::borrow::Cow;

use crate::ui::{
  bg_animation::{KINDS, Kind},
  common::menu::MenuItem,
};

#[derive(Clone, Default)]
pub struct Background {
  /// `None` disables the background, `Some(kind)` selects it.
  pub kind:  Option<Kind>,
  pub label: String,
}

impl MenuItem for Background {
  fn format(&self) -> Cow<'_, str> {
    Cow::Borrowed(&self.label)
  }
}

/// Build the menu list: a "None" entry followed by every registered kind.
pub fn options(none_label: &str) -> Vec<Background> {
  let mut out = Vec::with_capacity(KINDS.len() + 1);
  out.push(Background {
    kind:  None,
    label: none_label.to_string(),
  });
  for entry in KINDS {
    out.push(Background {
      kind:  Some(entry.kind),
      label: entry.label.to_string(),
    });
  }
  out
}
