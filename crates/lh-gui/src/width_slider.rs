//! The slider state for the chat width, built in one place so the order of its settings is right and can be tested
//! without a window. gpui-component clamps the value every time the minimum or the maximum is set, so the maximum
//! must go in first: with its default maximum of 100 still in place, a minimum of 480 makes the clamp panic.

use gpui_component::slider::SliderState;

use crate::layout::{mirror, slider_max, ColumnWidth, COLUMN_MIN, COLUMN_STEP};

/// A slider for `column` in a middle of `room` pixels. In right-to-left it holds the mirrored value.
pub fn state(column: ColumnWidth, room: f32, rtl: bool) -> SliderState {
    let pos = column.position(room);
    let value = if rtl { mirror(pos, room) } else { pos };
    SliderState::new().max(slider_max(room)).min(COLUMN_MIN).step(COLUMN_STEP).default_value(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_builds_for_the_room_the_app_starts_with() {
        let s = state(ColumnWidth::default(), 1200., false);
        assert_eq!((s.min_value(), s.max_value(), s.step_value()), (480., 1200., 10.));
        assert_eq!(s.value().end(), 860.);
    }

    #[test]
    fn it_builds_for_every_width_room_and_direction_including_tiny_windows() {
        let widths = [ColumnWidth::default(), ColumnWidth::Full, ColumnWidth::Px(480.), ColumnWidth::Px(5000.), ColumnWidth::px(f32::NAN)];
        for room in [0., 1., 100., 479., 480., 489., 490., 600., 1200., 3840., 1e9, f32::NAN, f32::INFINITY, -50.] {
            for rtl in [false, true] {
                for w in widths {
                    let s = state(w, room, rtl);
                    assert!(s.min_value() < s.max_value(), "room {room}");
                    let v = s.value().end();
                    assert!(v >= s.min_value() && v <= s.max_value(), "room {room} rtl {rtl} {w:?}: {v}");
                }
            }
        }
    }

    #[test]
    fn right_to_left_holds_the_mirrored_value() {
        let s = state(ColumnWidth::Px(900.), 1200., true);
        assert_eq!(s.value().end(), 780.);
        assert_eq!(state(ColumnWidth::Full, 1200., true).value().end(), 480., "Full is the left end when mirrored");
    }
}
