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

pub(crate) fn derived_mass(body: i32, shell: i32, chloroplasts: i32) -> f32 {
    (body.max(0) as f32 / 1_000.0)
        + (shell.max(0) as f32 / 200.0)
        + (chloroplasts.clamp(0, 32_000) as f32 / 32_000.0) * 31_680.0
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
        forward * input.aim_radians.sin() + lateral * input.aim_radians.cos(),
        forward * input.aim_radians.cos() - lateral * input.aim_radians.sin(),
    ];
    let magnitude = world[0].hypot(world[1]);
    if magnitude > settings.max_velocity {
        world = [
            world[0] / magnitude * settings.max_velocity,
            world[1] / magnitude * settings.max_velocity,
        ];
    }
    [
        world[0] * settings.movement_efficiency,
        world[1] * settings.movement_efficiency,
    ]
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
