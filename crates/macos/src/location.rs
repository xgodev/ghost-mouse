//! The coordinate an event must carry for a window point to land where it should.

use ghost_mouse_core::Point;

/// Measured on macOS 26: AppKit reads the location as window-relative once the event names a
/// window, with `y` mirrored by the main display's height. The window's own position never enters.
pub(crate) fn event_location(at: Point, main_display_height: f64) -> Point {
    Point { x: at.x, y: main_display_height - at.y }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn y_mirrors_by_the_main_display_height() {
        assert_eq!(event_location(Point { x: 180.0, y: 112.0 }, 1440.0), Point { x: 180.0, y: 1328.0 });
    }
}
