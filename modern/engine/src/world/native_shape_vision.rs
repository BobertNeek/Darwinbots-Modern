//! Quads.CompareShapes: nearest visible rectangle surface and shape references.
use super::*;

#[derive(Clone, Copy)]
pub(super) struct ShapeReference {
    pub position: [f32; 2],
    pub velocity: [f32; 2],
}

fn angle_contains(center: f32, half: f32, delta: [f32; 2]) -> bool {
    let angle = (-delta[1]).atan2(delta[0]);
    let offset = (angle - center + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
        - std::f32::consts::PI;
    offset.abs() <= half
}

fn surface_point(
    origin: [f32; 2],
    shape: &Obstacle,
    eye: &EyeSnapshot,
    raw_width: i32,
) -> Option<[f32; 2]> {
    if shape.minimum[0] > origin[0] + eye.range
        || shape.maximum[0] < origin[0] - eye.range
        || shape.minimum[1] > origin[1] + eye.range
        || shape.maximum[1] < origin[1] - eye.range
    {
        return None;
    }
    let closest = std::array::from_fn(|i| origin[i].clamp(shape.minimum[i], shape.maximum[i]));
    let mut half = (raw_width as f32 + 35.0) / 400.0;
    while half > std::f32::consts::PI {
        half -= std::f32::consts::PI;
    }
    while half < 0.0 {
        half += std::f32::consts::PI;
    }
    if angle_contains(
        eye.center_radians,
        half,
        [closest[0] - origin[0], closest[1] - origin[1]],
    ) {
        return Some(closest);
    }
    [eye.center_radians + half, eye.center_radians - half]
        .into_iter()
        .filter_map(|angle| {
            let delta = [angle.cos() * eye.range, -angle.sin() * eye.range];
            let end = [origin[0] + delta[0], origin[1] + delta[1]];
            let fraction =
                crate::physics::segment_aabb_fraction(origin, end, shape.minimum, shape.maximum)?;
            (fraction > 0.0).then_some((
                [
                    origin[0] + delta[0] * fraction,
                    origin[1] + delta[1] * fraction,
                ],
                fraction,
            ))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(point, _)| point)
}

pub(super) fn compare_shapes(
    shapes: &[Obstacle],
    observer: &RobotState,
    vision: &VisionSnapshot,
    mut eyes: [i32; 9],
) -> ([i32; 9], Option<ShapeReference>) {
    let origin = observer.kinematics.position;
    let mut reference = None;
    // The native temporary position is updated only while processing eye five.
    let mut last_point = [0.0; 2];
    for shape in shapes {
        if origin[0] > shape.minimum[0]
            && origin[0] < shape.maximum[0]
            && origin[1] > shape.minimum[1]
            && origin[1] < shape.maximum[1]
        {
            return (
                [32000; 9],
                Some(ShapeReference {
                    position: last_point,
                    velocity: shape.drift,
                }),
            );
        }
        for (index, eye) in eyes.iter_mut().enumerate() {
            let Some(point) = surface_point(
                origin,
                shape,
                &vision.eyes[index],
                observer.program.memory.read(531 + index as i32),
            ) else {
                continue;
            };
            let distance = (point[0] - origin[0]).hypot(point[1] - origin[1]);
            if distance >= 32000.0 {
                continue;
            }
            if index == 4 {
                last_point = point;
            }
            let percent = (distance - observer.materials.radius + 10.0) / vision.eyes[index].range;
            let strength = if percent <= 0.0 {
                32000
            } else {
                resource_readback((1.0 / (percent * percent)).min(32000.0))
            };
            if strength > *eye {
                *eye = strength;
                if index == vision.focus_eye as usize {
                    reference = Some(ShapeReference {
                        position: last_point,
                        velocity: shape.drift,
                    });
                }
            }
        }
    }
    (eyes, reference)
}

pub(super) fn relative_velocity(
    target: [f32; 2],
    heading: f32,
    own_up: i32,
    own_dx: i32,
    previous_sx: i32,
) -> [i32; 5] {
    let angle = f64::from(heading);
    // Shape references assign Double projections directly to Integer cells.
    let up = (f64::from(target[0]) * angle.cos()
        - f64::from(target[1]) * angle.sin()
        - f64::from(own_up))
    .round_ties_even() as i32;
    let dx = (f64::from(target[1]) * angle.cos() + f64::from(target[0]) * angle.sin()
        - f64::from(own_dx))
    .round_ties_even() as i32;
    let scalar = ((f64::from(up).powi(2) + f64::from(dx).powi(2)).sqrt() as f32).min(32000.0);
    [resource_readback(scalar), -previous_sx, dx, -up, up]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sense(world: &mut Engine) {
        world.clear_after_dna_phase();
        world.native_buckets.synchronize(
            [world.config.world_width, world.config.world_height],
            &world.slots,
        );
        world.spatial_index_phase();
        world.sensing_phase().unwrap();
    }

    #[test]
    fn visible_shape_publishes_surface_type_motion_and_zero_robot_materials() {
        let mut world = Engine::new(EngineConfig {
            shapes_visible: true,
            ..EngineConfig::testing()
        })
        .unwrap();
        let id = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [300.0, 500.0])
            .unwrap();
        let robot = world.slots.get_mut(id).unwrap();
        robot.set_heading(0.0);
        robot.program.memory.write(200, 4);
        robot.program.memory.write(198, 3);
        robot.program.memory.write(696, 17);
        let mut shape = Obstacle::new(1, [600.0, 400.0], [650.0, 600.0]);
        shape.drift = [5.51, 3.51];
        world.add_obstacle(shape).unwrap();
        sense(&mut world);
        assert!(world.memory_at(id, 505).unwrap() > 0);
        for (address, value) in [
            (685, 1),
            (689, 600),
            (690, 500),
            (695, 2),
            // The preceding EraseLookOccurr clears the seeded .refvelsx.
            (696, 0),
            (697, 1),
            (698, -2),
            (699, 2),
            (701, 0),
            (688, 0),
            (477, 0),
        ] {
            assert_eq!(world.memory_at(id, address).unwrap(), value);
        }
        let resumed = crate::SaveFile::decode(&crate::SaveFile::encode(&world).unwrap()).unwrap();
        assert!(resumed.config.shapes_visible);
        let native = Engine::from_native_sim(&world.export_native_sim().unwrap()).unwrap();
        assert!(native.config.shapes_visible);
    }

    #[test]
    fn invisible_opaque_shape_occludes_bots_without_becoming_a_reference() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let id = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [300.0, 500.0])
            .unwrap();
        world.slots.get_mut(id).unwrap().set_heading(0.0);
        world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [800.0, 500.0])
            .unwrap();
        sense(&mut world);
        assert!(world.memory_at(id, 505).unwrap() > 0);
        assert_eq!(world.memory_at(id, MEM_REF_X).unwrap(), 800);
        world
            .add_obstacle(Obstacle::new(1, [600.0, 400.0], [650.0, 600.0]))
            .unwrap();
        sense(&mut world);
        assert_eq!(world.memory_at(id, 505).unwrap(), 0);
        assert_eq!(world.memory_at(id, 685).unwrap(), 0);
    }

    #[test]
    fn rectangle_inside_and_eye_edge_intersection_are_visible() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let id = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [500.0; 2])
            .unwrap();
        let robot = world.slots.get(id).unwrap();
        let inside = Obstacle::new(1, [400.0; 2], [600.0; 2]);
        assert_eq!(
            compare_shapes(&[inside], robot, &VisionSnapshot::default(), [0; 9]).0,
            [32000; 9]
        );
        let corner = Obstacle::new(2, [700.0, 440.0], [800.0, 490.0]);
        assert!(
            surface_point([500.0; 2], &corner, &VisionSnapshot::default().eyes[4], 0).is_some()
        );
    }
}
