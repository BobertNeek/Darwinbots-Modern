use super::organism_radius;

pub(crate) fn resolve_collisions(
    positions: &mut [[f32; 2]],
    velocities: &mut [[f32; 2]],
    energies: &[i32],
    masses: &[f32],
    pairs: &[(usize, usize)],
    elasticity: f32,
) {
    let elasticity = elasticity.clamp(0.0, 1.0);
    for &(first, second) in pairs {
        if first == second || first >= positions.len() || second >= positions.len() {
            continue;
        }
        let minimum_distance = organism_radius(energies.get(first).copied().unwrap_or(1))
            + organism_radius(energies.get(second).copied().unwrap_or(1));
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
