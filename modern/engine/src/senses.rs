// These DB2.48.32 Single literals are compatibility data, not approximations to replace.
#[allow(clippy::approx_constant)]
const LEGACY_FULL_TURN: f32 = 6.28;
#[allow(clippy::approx_constant)]
const LEGACY_HALF_TURN: f32 = 3.14;
const LEGACY_QUARTER_TURN: f32 = 1.57;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LegacyDirection {
    Up,
    Down,
    Right,
    Left,
}

pub(crate) fn legacy_relative_angle(
    robot_position: [f32; 2],
    robot_aim: f32,
    event_point: [f32; 2],
) -> f32 {
    let aim = LEGACY_FULL_TURN - robot_aim;
    let dx = event_point[0] - robot_position[0];
    let dy = event_point[1] - robot_position[1];
    let ang = if dx != 0.0 {
        let mut ang = (dy / dx).atan();
        if dx < 0.0 {
            ang -= LEGACY_HALF_TURN;
        }
        ang
    } else if dy > 0.0 {
        LEGACY_QUARTER_TURN
    } else if dy < 0.0 {
        -LEGACY_QUARTER_TURN
    } else {
        0.0
    };
    let mut relative = ang - aim;
    while relative < 0.0 {
        relative += LEGACY_FULL_TURN;
    }
    while relative > LEGACY_FULL_TURN {
        relative -= LEGACY_FULL_TURN;
    }
    relative
}

#[allow(clippy::manual_range_contains)]
pub(crate) fn classify_legacy_direction(relative_angle: f32) -> Option<LegacyDirection> {
    if relative_angle > 5.49 || relative_angle < 0.78 {
        Some(LegacyDirection::Up)
    } else if relative_angle > 2.36 && relative_angle < 3.92 {
        Some(LegacyDirection::Down)
    } else if relative_angle > 0.78 && relative_angle < 2.36 {
        Some(LegacyDirection::Right)
    } else if relative_angle > 3.92 && relative_angle < 5.49 {
        Some(LegacyDirection::Left)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{
        LEGACY_FULL_TURN, LegacyDirection, classify_legacy_direction, legacy_relative_angle,
    };

    #[test]
    fn relative_angle_selects_all_four_aim_relative_quadrants() {
        let cases = [
            ([1.0, 0.0], LegacyDirection::Up),
            ([0.0, 1.0], LegacyDirection::Right),
            ([-1.0, 0.0], LegacyDirection::Down),
            ([0.0, -1.0], LegacyDirection::Left),
        ];

        for (event, expected) in cases {
            let angle = legacy_relative_angle([0.0, 0.0], 0.0, event);
            assert_eq!(
                classify_legacy_direction(angle),
                Some(expected),
                "{event:?}"
            );
        }
    }

    #[test]
    fn direction_thresholds_are_strict() {
        for boundary in [0.78, 2.36, 3.92, 5.49] {
            assert_eq!(
                classify_legacy_direction(boundary),
                None,
                "boundary {boundary} selected a direction"
            );
        }
        assert_eq!(classify_legacy_direction(0.0), Some(LegacyDirection::Up));
        assert_eq!(
            classify_legacy_direction(LEGACY_FULL_TURN),
            Some(LegacyDirection::Up)
        );
    }
}
