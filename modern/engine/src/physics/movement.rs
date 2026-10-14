use crate::PhysicsSettings;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct MovementInput {
    pub up: i32,
    pub down: i32,
    pub left: i32,
    pub right: i32,
    pub aim_radians: f32,
    pub new_move: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct MovementState {
    pub velocity: [f32; 2],
}

pub(crate) fn derived_mass(body: f32, shell: f32, chloroplasts: f32) -> f32 {
    (body.max(0.0) / 1_000.0)
        + (shell.max(0.0) / 200.0)
        + (chloroplasts.clamp(0.0, 32_000.0) / 32_000.0) * 31_680.0
}

pub(crate) fn voluntary_impulse(
    input: MovementInput,
    mass: f32,
    settings: &PhysicsSettings,
) -> [f32; 2] {
    let multiplier = if input.new_move { 1.0 } else { mass.max(1.0) };
    let forward = (input.up as i64 - input.down as i64) as f32 * multiplier;
    let lateral = (input.left as i64 - input.right as i64) as f32 * multiplier;
    let mut world = [
        forward * input.aim_radians.cos() + lateral * input.aim_radians.sin(),
        lateral * input.aim_radians.cos() - forward * input.aim_radians.sin(),
    ];
    let magnitude = world[0].hypot(world[1]);
    let max_velocity = settings.max_velocity.max(0.0);
    if magnitude > max_velocity {
        world = [
            world[0] / magnitude * max_velocity,
            world[1] / magnitude * max_velocity,
        ];
    }
    [
        world[0] * settings.movement_efficiency,
        world[1] * settings.movement_efficiency,
    ]
}

pub(crate) fn voluntary_cost_magnitude(
    input: MovementInput,
    mass: f32,
    settings: &PhysicsSettings,
) -> f32 {
    let multiplier = if input.new_move { 1.0 } else { mass.max(1.0) };
    let forward = (input.up as i64 - input.down as i64) as f32 * multiplier;
    let lateral = (input.left as i64 - input.right as i64) as f32 * multiplier;
    forward.hypot(lateral).min(settings.max_velocity.max(0.0))
}

pub(crate) fn apply_voluntary_impulse(
    state: &mut MovementState,
    input: MovementInput,
    mass: f32,
    settings: &PhysicsSettings,
) {
    let impulse = voluntary_impulse(input, mass, settings);
    state.velocity[0] += impulse[0];
    state.velocity[1] += impulse[1];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_dot_cross_motion_uses_east_north_west_south_bearings() {
        let settings = PhysicsSettings {
            movement_efficiency: 1.0,
            max_velocity: 100.0,
            ..Default::default()
        };
        for (angle, expected) in [
            (0.0, [40.0, 0.0]),
            (std::f32::consts::FRAC_PI_2, [0.0, -40.0]),
            (std::f32::consts::PI, [-40.0, 0.0]),
            (3.0 * std::f32::consts::FRAC_PI_2, [0.0, 40.0]),
        ] {
            let actual = voluntary_impulse(
                MovementInput {
                    up: 40,
                    aim_radians: angle,
                    ..Default::default()
                },
                1.0,
                &settings,
            );
            for axis in 0..2 {
                assert!((actual[axis] - expected[axis]).abs() < 0.00001);
            }
        }
        assert_eq!(
            voluntary_impulse(
                MovementInput {
                    left: 40,
                    ..Default::default()
                },
                1.0,
                &settings
            ),
            [0.0, 40.0]
        );
        assert_eq!(
            voluntary_impulse(
                MovementInput {
                    right: 40,
                    ..Default::default()
                },
                1.0,
                &settings
            ),
            [0.0, -40.0]
        );
    }
}
