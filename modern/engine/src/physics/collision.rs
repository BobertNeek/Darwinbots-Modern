pub(crate) fn segment_circle_fraction(
    start: [f32; 2],
    end: [f32; 2],
    center: [f32; 2],
    radius: f32,
) -> Option<f32> {
    let delta = [end[0] - start[0], end[1] - start[1]];
    let offset = [start[0] - center[0], start[1] - center[1]];
    let a = delta[0] * delta[0] + delta[1] * delta[1];
    let c = offset[0] * offset[0] + offset[1] * offset[1] - radius * radius;
    if c <= 0.0 {
        return Some(0.0);
    }
    if a <= f32::EPSILON {
        return None;
    }
    let b = 2.0 * (offset[0] * delta[0] + offset[1] * delta[1]);
    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        return None;
    }
    let denominator = 2.0 * a;
    let near = (-b - discriminant.sqrt()) / denominator;
    let far = (-b + discriminant.sqrt()) / denominator;
    [near, far].into_iter().find(|root| (0.0..=1.0).contains(root))
}

pub(crate) fn segment_aabb_fraction(
    start: [f32; 2],
    end: [f32; 2],
    minimum: [f32; 2],
    maximum: [f32; 2],
) -> Option<f32> {
    let delta = [end[0] - start[0], end[1] - start[1]];
    let mut entry = 0.0_f32;
    let mut exit = 1.0_f32;
    for axis in 0..2 {
        if delta[axis].abs() <= f32::EPSILON {
            if start[axis] < minimum[axis] || start[axis] > maximum[axis] {
                return None;
            }
            continue;
        }
        let inverse = delta[axis].recip();
        let mut first = (minimum[axis] - start[axis]) * inverse;
        let mut second = (maximum[axis] - start[axis]) * inverse;
        if first > second {
            std::mem::swap(&mut first, &mut second);
        }
        entry = entry.max(first);
        exit = exit.min(second);
        if entry > exit {
            return None;
        }
    }
    (0.0..=1.0).contains(&entry).then_some(entry)
}

pub(crate) fn resolve_collisions(
    positions: &mut [[f32; 2]],
    velocities: &mut [[f32; 2]],
    radii: &[f32],
    masses: &[f32],
    pairs: &[(usize, usize)],
    elasticity: f32,
) {
    let elasticity = elasticity.clamp(0.0, 1.0);
    for &(first, second) in pairs {
        if first == second || first >= positions.len() || second >= positions.len() {
            continue;
        }
        let minimum_distance = radii.get(first).copied().unwrap_or(1.0)
            + radii.get(second).copied().unwrap_or(1.0);
        let delta = [
            positions[second][0] - positions[first][0],
            positions[second][1] - positions[first][1],
        ];
        let distance_squared = delta[0] * delta[0] + delta[1] * delta[1];
        if distance_squared >= minimum_distance * minimum_distance {
            continue;
        }
        let (normal, distance) = if distance_squared <= f32::EPSILON {
            ([if (first ^ second) & 1 == 0 { 1.0 } else { -1.0 }, 0.0], 0.0)
        } else {
            let distance = distance_squared.sqrt();
            ([delta[0] / distance, delta[1] / distance], distance)
        };
        let first_mass = masses.get(first).copied().unwrap_or(1.0).clamp(1.0, 32_000.0);
        let second_mass = masses.get(second).copied().unwrap_or(1.0).clamp(1.0, 32_000.0);
        let total_mass = first_mass + second_mass;
        let overlap = minimum_distance - distance;
        positions[first][0] -= normal[0] * overlap * second_mass / total_mass;
        positions[first][1] -= normal[1] * overlap * second_mass / total_mass;
        positions[second][0] += normal[0] * overlap * first_mass / total_mass;
        positions[second][1] += normal[1] * overlap * first_mass / total_mass;

        let relative_speed = (velocities[second][0] - velocities[first][0]) * normal[0]
            + (velocities[second][1] - velocities[first][1]) * normal[1];
        if relative_speed >= 0.0 {
            continue;
        }
        let impulse = -(1.0 + elasticity) * relative_speed
            / (first_mass.recip() + second_mass.recip());
        velocities[first][0] -= normal[0] * impulse / first_mass;
        velocities[first][1] -= normal[1] * impulse / first_mass;
        velocities[second][0] += normal[0] * impulse / second_mass;
        velocities[second][1] += normal[1] * impulse / second_mass;
    }
}
