//! What Milestone 2 draws over the map in Normal mode (W0-B), after the
//! spotlight card and the director's caption: one line per item (the plan
//! ghost's legend, the director's badge, the mood strip's click targets...).

use crate::app::App;
use ratatui::layout::Rect;
use ratatui::Frame;

/// `over` is the map's area.
pub(super) fn draw(f: &mut Frame, app: &App, over: Rect) {
    let _ = (f, app, over);
}
