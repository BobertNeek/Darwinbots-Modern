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
    [near, far]
        .into_iter()
        .find(|root| (0.0..=1.0).contains(root))
}

pub(crate) fn moving_circle_fraction(
    start: [f32; 2],
    end: [f32; 2],
    center: [f32; 2],
    center_velocity: [f32; 2],
    radius: f32,
) -> Option<f32> {
    let relative_end = [end[0] - center_velocity[0], end[1] - center_velocity[1]];
    let delta = [relative_end[0] - start[0], relative_end[1] - start[1]];
    let offset = [start[0] - center[0], start[1] - center[1]];
    let c = offset[0] * offset[0] + offset[1] * offset[1] - radius * radius;
    if c < 0.0 {
        return Some(0.0);
    }
    let a = delta[0] * delta[0] + delta[1] * delta[1];
    if a == 0.0 {
        return None;
    }
    let dot = delta[0] * offset[0] + delta[1] * offset[1];
    let discriminant = dot * dot - a * c;
    if discriminant < 0.0 {
        return None;
    }
    [
        (-dot - discriminant.sqrt()) / a,
        (-dot + discriminant.sqrt()) / a,
    ]
    .into_iter()
    .find(|root| *root > 0.0 && *root < 1.0)
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

fn native_magnitude(vector: [f32; 2]) -> f32 {
    let maximum = vector[0].abs().max(vector[1].abs());
    let minimum = vector[0].abs().min(vector[1].abs());
    if maximum < 0.00001 {
        0.0
    } else {
        let ratio = minimum / maximum;
        (f64::from(maximum) * (1.0 + f64::from(ratio).powi(2)).sqrt()) as f32
    }
}

fn native_scalar(vector: [f32; 2], scale: f32) -> [f32; 2] {
    let scale = scale.clamp(-32_000.0, 32_000.0);
    vector.map(|component| component.clamp(-32_000.0, 32_000.0) * scale)
}

fn add(first: [f32; 2], second: [f32; 2]) -> [f32; 2] {
    [first[0] + second[0], first[1] + second[1]]
}

pub(crate) fn resolve_collisions(
    positions: &mut [[f32; 2]],
    velocities: &mut [[f32; 2]],
    radii: &[f32],
    masses: &[f32],
    fixed: &[bool],
    pairs: &[(usize, usize)],
    elasticity: f32,
) {
    // Physics.Repel3, including its gradual separation, projection damping and
    // finite fixed-bot mass. VectorScalar's native Single clamp is intentional.
    let elasticity = elasticity.clamp(0.0, 1.0);
    for &(first, second) in pairs {
        if first == second || first >= positions.len() || second >= positions.len() {
            continue;
        }
        let minimum_distance =
            radii.get(first).copied().unwrap_or(1.0) + radii.get(second).copied().unwrap_or(1.0);
        let delta = [
            positions[second][0] - positions[first][0],
            positions[second][1] - positions[first][1],
        ];
        let distance = native_magnitude(delta);
        if distance >= minimum_distance {
            continue;
        }
        let inverse = if distance == 0.0 {
            -1.0
        } else {
            (1.0 / f64::from(distance)) as f32
        };
        let normal = delta.map(|component| component * inverse);
        let first_mass = masses
            .get(first)
            .copied()
            .unwrap_or(1.0)
            .clamp(1.0, 32_000.0);
        let second_mass = masses
            .get(second)
            .copied()
            .unwrap_or(1.0)
            .clamp(1.0, 32_000.0);
        let first_fixed = fixed.get(first).copied().unwrap_or(false);
        let second_fixed = fixed.get(second).copied().unwrap_or(false);
        let overlap = minimum_distance - distance;
        let stationary = native_magnitude(velocities[first]) < 0.0001
            && native_magnitude(velocities[second]) < 0.0001;
        let (first_separation, second_separation) = if first_fixed && second_fixed || stationary {
            let half = native_scalar(normal, (f64::from(overlap) / 2.0) as f32);
            (half, half)
        } else {
            let scale =
                (f64::from(overlap) / (1.0 + 55_f64.powf(0.3 - f64::from(elasticity)))) as f32;
            let separation = native_scalar(normal, scale);
            let total_mass = first_mass + second_mass;
            (
                native_scalar(separation, second_mass / total_mass),
                native_scalar(separation, first_mass / total_mass),
            )
        };
        for axis in 0..2 {
            positions[first][axis] -= first_separation[axis];
            positions[second][axis] += second_separation[axis];
        }
        if inverse == -1.0 {
            continue;
        }
        let projection = |velocity: [f32; 2]| {
            (f64::from(velocity[0] * normal[0] + velocity[1] * normal[1]) * 0.99) as f32
        };
        let first_projection = projection(velocities[first]);
        let second_projection = projection(velocities[second]);
        let first_component = native_scalar(
            normal,
            if first_projection <= 0.0 {
                0.000001
            } else {
                first_projection
            },
        );
        let second_component = native_scalar(
            normal,
            if second_projection >= 0.0 {
                -0.000001
            } else {
                second_projection
            },
        );
        let mass1 = if first_fixed { 32_000.0 } else { first_mass };
        let mass2 = if second_fixed { 32_000.0 } else { second_mass };
        let inverse_mass = 1.0 / (mass1 + mass2);
        let first_result = native_scalar(
            add(
                native_scalar(
                    second_component,
                    ((f64::from(elasticity) + 1.0) * f64::from(mass2)) as f32,
                ),
                native_scalar(first_component, mass1 - elasticity * mass2),
            ),
            inverse_mass,
        );
        let second_result = native_scalar(
            add(
                native_scalar(
                    first_component,
                    ((f64::from(elasticity) + 1.0) * f64::from(mass1)) as f32,
                ),
                native_scalar(second_component, mass2 - elasticity * mass1),
            ),
            inverse_mass,
        );
        for axis in 0..2 {
            if !first_fixed {
                velocities[first][axis] =
                    velocities[first][axis] - first_component[axis] + first_result[axis];
            }
            if !second_fixed {
                velocities[second][axis] =
                    velocities[second][axis] - second_component[axis] + second_result[axis];
            }
        }
    }
}

#[cfg(test)]
mod native_contact_tests {
    use super::resolve_collisions;

    #[test]
    fn moving_contacts_keep_native_compression_and_projection_damping() {
        let mut positions = [[0.0, 0.0], [100.0, 0.0]];
        let mut velocities = [[10.0, 0.0], [-10.0, 0.0]];
        resolve_collisions(
            &mut positions,
            &mut velocities,
            &[60.0; 2],
            &[1.0; 2],
            &[false; 2],
            &[(0, 1)],
            0.5,
        );
        assert!((velocities[0][0] + 4.85).abs() < 0.00001);
        assert!((velocities[1][0] - 4.85).abs() < 0.00001);
        let distance = positions[1][0] - positions[0][0];
        assert!(distance > 100.0 && distance < 120.0);
    }

    #[test]
    fn fixed_contact_uses_native_mass_and_vector_scalar_clamp() {
        let mut positions = [[0.0, 0.0], [100.0, 0.0]];
        let mut velocities = [[0.0, 0.0], [-10.0, 0.0]];
        resolve_collisions(
            &mut positions,
            &mut velocities,
            &[60.0; 2],
            &[1.0; 2],
            &[true, false],
            &[(0, 1)],
            1.0,
        );
        assert_eq!(velocities[0], [0.0; 2]);
        assert!((velocities[1][0] - 0.8999683).abs() < 0.00001);
        assert!(positions[0][0] < 0.0, "Repel3 also shifts fixed positions");
    }

    #[test]
    fn stationary_and_fixed_contacts_separate_equally_and_coincident_centers_stay_put() {
        for fixed in [[false; 2], [true; 2]] {
            let mut positions = [[0.0, 0.0], [100.0, 0.0]];
            let mut velocities = [[0.0; 2]; 2];
            resolve_collisions(
                &mut positions,
                &mut velocities,
                &[60.0; 2],
                &[1.0, 20.0],
                &fixed,
                &[(0, 1)],
                0.5,
            );
            assert_eq!(positions, [[-10.0, 0.0], [110.0, 0.0]]);
        }
        let mut positions = [[0.0; 2]; 2];
        let mut velocities = [[0.0; 2]; 2];
        resolve_collisions(
            &mut positions,
            &mut velocities,
            &[60.0; 2],
            &[1.0; 2],
            &[false; 2],
            &[(0, 1)],
            0.5,
        );
        assert_eq!(positions, [[0.0; 2]; 2]);
        assert_eq!(velocities, [[0.0; 2]; 2]);
    }
}

#[cfg(test)]
mod projectile_motion_tests {
    use super::moving_circle_fraction;

    #[test]
    fn original_shot_collision_accounts_for_motion_and_excludes_cycle_endpoints() {
        assert_eq!(
            moving_circle_fraction([0.0, 0.0], [10.0, 0.0], [15.0, 0.0], [0.0, 0.0], 1.0),
            None
        );
        assert_eq!(
            moving_circle_fraction([0.0, 0.0], [10.0, 0.0], [15.0, 0.0], [-10.0, 0.0], 1.0),
            Some(0.7)
        );
        assert_eq!(
            moving_circle_fraction([0.0, 0.0], [10.0, 0.0], [11.0, 0.0], [0.0, 0.0], 1.0),
            None
        );
        assert_eq!(
            moving_circle_fraction([0.0, 0.0], [10.0, 0.0], [0.0, 0.0], [0.0, 0.0], 1.0),
            Some(0.0)
        );
    }
}
