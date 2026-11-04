//! Obstacles.bas DoObstacleCollisions: ordered pushes precede border and force handling.

use super::*;

impl Engine {
    pub(super) fn native_obstacles_for(&mut self, id: OrganismId) {
        let cycle = self.native_cycle_or_tick();
        let world_size = [self.config.world_width, self.config.world_height];
        let Ok(robot) = self.slots.get_mut(id) else {
            return;
        };
        if robot.flags.hidden {
            return;
        }
        let radius = robot.materials.radius;
        let mut collisions = 0;
        let mut last_push = 0;

        // The source visits each shape once. A third overlapping shape escapes
        // immediately; it does not retry the scan or resolve that shape's edge.
        for obstacle in &self.obstacles {
            let [x, y] = robot.kinematics.position;
            let [left, top] = obstacle.minimum;
            let [right, bottom] = obstacle.maximum;
            if !(x + radius > left && x - radius < right && y + radius > top && y - radius < bottom)
            {
                continue;
            }
            collisions += 1;
            if collisions >= 3 {
                robot.kinematics.position[0] += (200 * ((cycle % 40) as i32 - 20).signum()) as f32;
                robot.kinematics.position[1] += (200 * ((cycle % 50) as i32 - 25).signum()) as f32;
                break;
            }

            let up = (y + radius) - top;
            let down = bottom - (y - radius);
            let left_distance = (x + radius) - left;
            let right_distance = right - (x - radius);
            let horizontal = (left_distance.min(right_distance) < up.min(down)
                && last_push != 1
                && last_push != 2)
                || last_push == 3
                || last_push == 4;
            let mut touch = None;

            if horizontal {
                if (left_distance <= right_distance || right >= world_size[0]) && left > 0.0 {
                    robot.kinematics.position[0] = left - radius;
                    if x - radius < left {
                        robot.kinematics.resistive_impulse[0] += robot.kinematics.velocity[0] * 0.5;
                        touch = Some([robot.kinematics.position[0] + radius, y]);
                    } else {
                        robot.kinematics.resistive_impulse[0] += left_distance * 0.5;
                    }
                    last_push = 1;
                } else {
                    robot.kinematics.position[0] = right + radius;
                    if x + radius > right {
                        robot.kinematics.resistive_impulse[0] += robot.kinematics.velocity[0] * 0.5;
                        touch = Some([robot.kinematics.position[0] - radius, y]);
                    } else {
                        robot.kinematics.resistive_impulse[0] -= right_distance * 0.5;
                    }
                    last_push = 2;
                }
            } else if (up <= down || bottom >= world_size[1]) && top > 0.0 {
                robot.kinematics.position[1] = top - radius;
                if y - radius < top {
                    robot.kinematics.resistive_impulse[1] += robot.kinematics.velocity[1] * 0.5;
                    touch = Some([x, robot.kinematics.position[1] + radius]);
                } else {
                    robot.kinematics.resistive_impulse[1] += up * 0.5;
                }
                last_push = 3;
            } else {
                robot.kinematics.position[1] = bottom + radius;
                if y + radius > bottom {
                    robot.kinematics.resistive_impulse[1] += robot.kinematics.velocity[1] * 0.5;
                    touch = Some([x, robot.kinematics.position[1] - radius]);
                } else {
                    robot.kinematics.resistive_impulse[1] -= down * 0.5;
                }
                last_push = 4;
            }

            if let Some(point) = touch {
                let position = robot.kinematics.position;
                write_robot_touch(robot, position, point);
            }
            // EYEF is the last cycle's focus-eye memory, not a fresh vision query.
            if robot.program.memory.read(510) == 0 {
                robot.program.memory.write(MEM_REF_TYPE, 1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actor(position: [f32; 2]) -> (Engine, OrganismId) {
        let mut world = Engine::new_native(EngineConfig {
            axis_topology: Some([false; 2]),
            ..EngineConfig::testing()
        })
        .unwrap();
        let id = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), position)
            .unwrap();
        let robot = world.slots.get_mut(id).unwrap();
        robot.materials.radius = 10.0;
        robot.kinematics.velocity = [8.0, -6.0];
        robot.kinematics.resistive_impulse = [0.0; 2];
        robot.program.memory = VmMemory::default();
        robot.set_heading(0.0);
        world.native_phase = Some(native_pipeline::NativeRobotPhase {
            actor: id,
            cost_population: 1,
            chloroplast_total: 0.0,
            radius_bound: 415.0,
        });
        world.native_cycle = Some(0);
        (world, id)
    }

    fn trapped(cycle: Option<i32>) -> (Engine, OrganismId) {
        let (mut world, id) = actor([500.0; 2]);
        world.native_cycle = cycle;
        world.tick = 1_025;
        // IDs are deliberately out of numerical order: source slot order wins.
        world.obstacles = vec![
            Obstacle::new(30, [490.0; 2], [510.0; 2]),
            Obstacle::new(10, [490.0, 470.0], [510.0, 490.0]),
            Obstacle::new(20, [470.0; 2], [490.0; 2]),
            Obstacle::new(40, [270.0; 2], [290.0; 2]),
        ];
        (world, id)
    }

    #[test]
    fn native_obstacle_overlap_is_strict_aabb_including_corners() {
        for (position, expected, hit) in [
            ([492.0; 2], [492.0, 490.0], 1),
            ([490.0, 550.0], [490.0, 550.0], 0),
            ([550.0, 490.0], [550.0, 490.0], 0),
            ([650.0; 2], [650.0; 2], 0),
        ] {
            let (mut world, id) = actor(position);
            world.obstacles = vec![Obstacle::new(1, [500.0; 2], [600.0; 2])];
            world.apply_environment_features(true, false);
            let robot = world.slots.get(id).unwrap();
            assert_eq!(robot.kinematics.position, expected);
            assert_eq!(robot.kinematics.velocity, [8.0, -6.0]);
            assert_eq!(robot.program.memory.read(MEM_HIT), hit);
            assert_eq!(robot.program.memory.read(MEM_REF_TYPE), hit);
            assert_eq!(
                robot.kinematics.resistive_impulse,
                if hit == 1 { [0.0, -3.0] } else { [0.0; 2] }
            );
        }
    }

    #[test]
    fn native_obstacle_crossed_edges_touch_without_reflecting_velocity() {
        for (position, expected, impulse, direction) in [
            ([495.0, 500.0], [490.0, 500.0], [4.0, 0.0], MEM_HIT_UP),
            ([605.0, 500.0], [610.0, 500.0], [4.0, 0.0], MEM_HIT_DOWN),
            ([550.0, 395.0], [550.0, 390.0], [0.0, -3.0], MEM_HIT_RIGHT),
            ([550.0, 605.0], [550.0, 610.0], [0.0, -3.0], MEM_HIT_LEFT),
        ] {
            let (mut world, id) = actor(position);
            let robot = world.slots.get_mut(id).unwrap();
            robot.flags.fixed = true;
            robot.flags.corpse = true;
            robot.kinematics.resistive_impulse = [2.0, 3.0];
            world.obstacles = vec![Obstacle::new(1, [500.0, 400.0], [600.0, 600.0])];
            world.native_obstacles_for(id);
            let robot = world.slots.get(id).unwrap();
            assert_eq!(robot.kinematics.position, expected);
            assert_eq!(robot.kinematics.velocity, [8.0, -6.0]);
            assert_eq!(
                robot.kinematics.resistive_impulse,
                [2.0 + impulse[0], 3.0 + impulse[1]]
            );
            assert_eq!(robot.program.memory.read(MEM_HIT), 1);
            assert_eq!(robot.program.memory.read(direction), 1);
            assert!(robot.flags.fixed && robot.flags.corpse);
        }
    }

    #[test]
    fn native_obstacle_embedded_pushes_add_springs_without_touch() {
        for (position, minimum, maximum, expected, impulse) in [
            (
                [550.0, 500.0],
                [500.0, 400.0],
                [600.0, 600.0],
                [490.0, 500.0],
                [30.0, 0.0],
            ),
            (
                [575.0, 500.0],
                [500.0, 400.0],
                [600.0, 600.0],
                [610.0, 500.0],
                [-17.5, 0.0],
            ),
            (
                [500.0, 425.0],
                [400.0, 400.0],
                [600.0, 500.0],
                [500.0, 390.0],
                [0.0, 17.5],
            ),
            (
                [500.0, 475.0],
                [400.0, 400.0],
                [600.0, 500.0],
                [500.0, 510.0],
                [0.0, -17.5],
            ),
        ] {
            let (mut world, id) = actor(position);
            world.obstacles = vec![Obstacle::new(1, minimum, maximum)];
            world.native_obstacles_for(id);
            let robot = world.slots.get(id).unwrap();
            assert_eq!(robot.kinematics.position, expected);
            assert_eq!(robot.kinematics.resistive_impulse, impulse);
            assert_eq!(robot.kinematics.velocity, [8.0, -6.0]);
            assert_eq!(robot.program.memory.read(MEM_HIT), 0);
            assert_eq!(robot.program.memory.read(MEM_REF_TYPE), 1);
        }
    }

    #[test]
    fn native_obstacle_world_edges_force_the_inward_push() {
        for (position, minimum, maximum, expected, impulse) in [
            (
                [5.0, 500.0],
                [0.0, 400.0],
                [100.0, 600.0],
                [110.0, 500.0],
                [-52.5, 0.0],
            ),
            (
                [995.0, 500.0],
                [900.0, 400.0],
                [1000.0, 600.0],
                [890.0, 500.0],
                [52.5, 0.0],
            ),
            (
                [500.0, 5.0],
                [400.0, 0.0],
                [600.0, 100.0],
                [500.0, 110.0],
                [0.0, -52.5],
            ),
            (
                [500.0, 995.0],
                [400.0, 900.0],
                [600.0, 1000.0],
                [500.0, 890.0],
                [0.0, 52.5],
            ),
        ] {
            let (mut world, id) = actor(position);
            world.obstacles = vec![Obstacle::new(1, minimum, maximum)];
            world.native_obstacles_for(id);
            let robot = world.slots.get(id).unwrap();
            assert_eq!(robot.kinematics.position, expected);
            assert_eq!(robot.kinematics.resistive_impulse, impulse);
            assert_eq!(robot.program.memory.read(MEM_HIT), 0);
        }
    }

    #[test]
    fn native_obstacle_pushes_alternate_axes_and_preserve_existing_eye_reference() {
        let (mut world, id) = trapped(Some(0));
        world.obstacles.truncate(2);
        world
            .slots
            .get_mut(id)
            .unwrap()
            .program
            .memory
            .write(510, 17);
        world
            .slots
            .get_mut(id)
            .unwrap()
            .program
            .memory
            .write(MEM_REF_TYPE, 7);
        world.apply_environment_features(true, false);
        let robot = world.slots.get(id).unwrap();
        assert_eq!(robot.kinematics.position, [480.0; 2]);
        assert_eq!(robot.kinematics.resistive_impulse, [10.0; 2]);
        assert_eq!(robot.program.memory.read(MEM_HIT), 0);
        assert_eq!(robot.program.memory.read(MEM_REF_TYPE), 7);
    }

    #[test]
    fn native_obstacle_third_overlap_escapes_at_literal_native_cycles_and_stops() {
        for (cycle, expected) in [
            (0, [280.0, 280.0]),
            (20, [480.0, 280.0]),
            (25, [680.0, 480.0]),
            (39, [680.0, 680.0]),
            (40, [280.0, 680.0]),
            (50, [280.0, 280.0]),
        ] {
            let (mut world, id) = trapped(Some(cycle));
            world.apply_environment_features(true, false);
            let robot = world.slots.get(id).unwrap();
            assert_eq!(robot.kinematics.position, expected, "native cycle {cycle}");
            assert_eq!(robot.kinematics.resistive_impulse, [10.0; 2]);
            assert_eq!(robot.kinematics.velocity, [8.0, -6.0]);
            assert_eq!(robot.program.memory.read(MEM_HIT), 0);
            assert_eq!(robot.program.memory.read(MEM_REF_TYPE), 1);
        }
    }

    #[test]
    fn native_obstacle_old_save_without_native_cycle_uses_monotonic_tick() {
        let (mut world, id) = trapped(None);
        world.apply_environment_features(true, false);
        // 1025 Mod40 =25; 1025 Mod50 =25.
        assert_eq!(
            world.slots.get(id).unwrap().kinematics.position,
            [680.0, 480.0]
        );
    }

    #[test]
    fn native_obstacle_scan_state_resets_for_each_robot_call() {
        let (mut world, id) = actor([550.0, 500.0]);
        world.obstacles = vec![Obstacle::new(1, [500.0, 400.0], [600.0, 600.0])];
        for _ in 0..3 {
            let robot = world.slots.get_mut(id).unwrap();
            robot.kinematics.position = [550.0, 500.0];
            robot.kinematics.resistive_impulse = [0.0; 2];
            world.native_obstacles_for(id);
            let robot = world.slots.get(id).unwrap();
            assert_eq!(robot.kinematics.position, [490.0, 500.0]);
            assert_eq!(robot.kinematics.resistive_impulse, [30.0, 0.0]);
        }
    }

    #[test]
    fn native_obstacle_escape_is_unclamped_until_the_following_border_phase() {
        let (mut world, id) = actor([100.0; 2]);
        world.obstacles = vec![
            Obstacle::new(1, [90.0; 2], [110.0; 2]),
            Obstacle::new(2, [90.0, 70.0], [110.0, 90.0]),
            Obstacle::new(3, [70.0; 2], [90.0; 2]),
        ];
        world.apply_environment_features(true, false);
        assert_eq!(
            world.slots.get(id).unwrap().kinematics.position,
            [-120.0; 2]
        );
        world.border_collision_phase();
        let robot = world.slots.get(id).unwrap();
        assert_eq!(robot.kinematics.position, [10.0; 2]);
        assert_eq!(robot.kinematics.velocity, [8.0, -6.0]);
        assert_eq!(robot.kinematics.resistive_impulse, [10.4, 9.7]);
        assert_eq!(robot.program.memory.read(MEM_EDGE), 1);
    }

    #[test]
    fn native_obstacle_hidden_robots_are_not_moved() {
        let (mut world, id) = trapped(Some(0));
        world.slots.get_mut(id).unwrap().flags.hidden = true;
        world.native_obstacles_for(id);
        let robot = world.slots.get(id).unwrap();
        assert_eq!(robot.kinematics.position, [500.0; 2]);
        assert_eq!(robot.kinematics.resistive_impulse, [0.0; 2]);
        assert_eq!(robot.program.memory.read(MEM_REF_TYPE), 0);
    }

    #[test]
    fn legacy_obstacle_branch_keeps_its_existing_position_and_velocity_policy() {
        let (mut world, id) = actor([492.0, 493.0]);
        world.native_random = None;
        world.native_phase = None;
        world.native_cycle = None;
        world.obstacles = vec![Obstacle::new(1, [500.0; 2], [600.0; 2])];
        world.apply_environment_features(true, false);
        let robot = world.slots.get(id).unwrap();
        assert_eq!(robot.kinematics.position, [490.0, 493.0]);
        assert_eq!(robot.kinematics.velocity, [-8.0, -6.0]);
        assert_eq!(robot.kinematics.resistive_impulse, [0.0; 2]);
        assert_eq!(robot.program.memory.read(MEM_HIT), 0);
    }
}
