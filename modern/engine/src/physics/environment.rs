use crate::PhysicsSettings;

const VELOCITY_EPSILON: f32 = 0.000_000_1;

pub(crate) fn environment_impulse(
    gravity: [f32; 2],
    brownian_motion: f32,
    seed: u64,
    tick: u64,
    stable_slot: usize,
) -> [f32; 2] {
    if brownian_motion <= 0.0 {
        return gravity;
    }
    let random = splitmix64(seed ^ tick.rotate_left(17) ^ (stable_slot as u64).rotate_left(33));
    let magnitude = brownian_motion * 0.5 * unit_float(random);
    let angle = unit_float(splitmix64(random)) * std::f32::consts::TAU;
    [
        gravity[0] + angle.cos() * magnitude,
        gravity[1] + angle.sin() * magnitude,
    ]
}

pub(crate) fn apply_linear_drag(velocity: &mut [f32; 2], drag: f32) {
    let retention = 1.0 - drag.clamp(0.0, 0.99);
    velocity[0] *= retention;
    velocity[1] *= retention;
    suppress_underflow(velocity);
}

pub(crate) fn apply_surface_friction(
    velocity: &mut [f32; 2],
    mass: f32,
    settings: &PhysicsSettings,
) {
    if settings.surface_gravity <= 0.0 {
        return;
    }
    let speed = velocity[0].hypot(velocity[1]);
    if speed <= f32::EPSILON {
        return;
    }
    let impulse = (mass * settings.surface_gravity * settings.kinetic_friction).min(speed);
    velocity[0] -= velocity[0] / speed * impulse;
    velocity[1] -= velocity[1] / speed * impulse;
    suppress_underflow(velocity);
}

pub(crate) fn apply_fluid_drag(
    velocity: &mut [f32; 2],
    radius: f32,
    settings: &PhysicsSettings,
) {
    let speed = velocity[0].hypot(velocity[1]);
    if speed < VELOCITY_EPSILON || settings.density <= 0.0 || settings.viscosity <= 0.0 {
        return;
    }
    let coefficient = sphere_drag_coefficient(
        speed as f64,
        radius.max(0.0) as f64,
        settings.density,
        settings.viscosity,
    );
    let area = std::f64::consts::PI * (radius.max(0.0) as f64).powi(2);
    let impulse = (0.5 * coefficient * settings.density * (speed as f64).powi(2) * area)
        .min(speed as f64 * 0.99) as f32;
    velocity[0] -= velocity[0] / speed * impulse;
    velocity[1] -= velocity[1] / speed * impulse;
    suppress_underflow(velocity);
}

pub(crate) fn apply_resistance(
    velocity: &mut [f32; 2],
    mass: f32,
    radius: f32,
    linear_drag: f32,
    settings: &PhysicsSettings,
) {
    apply_surface_friction(velocity, mass, settings);
    apply_fluid_drag(velocity, radius, settings);
    apply_linear_drag(velocity, linear_drag);
}

pub(crate) fn integrate_body(
    position: &mut [f32; 2],
    velocity: [f32; 2],
    world_size: [f32; 2],
) {
    position[0] = (position[0] + velocity[0]).clamp(0.0, world_size[0]);
    position[1] = (position[1] + velocity[1]).clamp(0.0, world_size[1]);
}

fn sphere_drag_coefficient(speed: f64, radius: f64, density: f64, viscosity: f64) -> f64 {
    if viscosity <= 0.0 || density <= 0.0 || radius <= 0.0 {
        return 0.0;
    }
    let reynolds = radius * 2.0 * speed.max(0.000_01) * density / viscosity;
    if reynolds == 0.0 {
        return 0.0;
    }
    let y1 = 24.0 / 300_000.0 + 6.0 / (1.0 + 300_000.0_f64.sqrt()) + 0.4;
    let y2 = 0.09;
    let alpha = (y2 - y1) * 50_000.0_f64.powi(-2);
    if reynolds < 300_000.0 {
        24.0 / reynolds + 6.0 / (1.0 + reynolds.sqrt()) + 0.4
    } else if reynolds < 350_000.0 {
        alpha * (reynolds - 300_000.0).powi(2) + y1
    } else if reynolds < 600_000.0 {
        0.09
    } else if reynolds < 4_000_000.0 {
        (reynolds / 600_000.0).powf(0.55) * y2
    } else {
        0.255
    }
}

fn suppress_underflow(velocity: &mut [f32; 2]) {
    if velocity[0].abs() < VELOCITY_EPSILON {
        velocity[0] = 0.0;
    }
    if velocity[1].abs() < VELOCITY_EPSILON {
        velocity[1] = 0.0;
    }
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn unit_float(value: u64) -> f32 {
    (value >> 40) as f32 / ((1_u32 << 24) - 1) as f32
}
