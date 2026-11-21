//! Robots.SetAimFunc: Single assignments, VB Mod, turn fees and momentum.

use super::*;
use crate::AimCommands;

fn angle_normalize(mut angle: f32) -> f32 {
    while angle < 0.0 {
        angle += std::f32::consts::TAU;
    }
    while angle > std::f32::consts::TAU {
        angle -= std::f32::consts::TAU;
    }
    angle
}

fn angle_difference(first: f32, second: f32) -> f32 {
    let mut difference = first - second;
    if difference > std::f32::consts::PI {
        difference = -(std::f32::consts::TAU - difference);
    }
    if difference < -std::f32::consts::PI {
        difference += std::f32::consts::TAU;
    }
    difference
}

pub(super) fn apply(
    robot: &mut RobotState,
    commands: AimCommands,
    rotation_cost: f64,
    multiplier: f64,
    native: bool,
) {
    let heading = robot.heading();
    let heading_units = heading * 200.0;
    let mut difference = commands.turn_left as f32 - commands.turn_right as f32;
    let mut extra_turn = 0.0;
    let desired_units = if commands.set == heading_units.round_ties_even() as i32 {
        heading_units + difference
    } else {
        difference =
            -angle_difference(heading, angle_normalize(commands.set as f32 / 200.0)) * 200.0;
        let direction = if difference == 0.0 {
            0.0
        } else {
            difference.signum()
        };
        extra_turn = ((heading_units - commands.set as f32) / 1256.0)
            .round_ties_even()
            .abs()
            * 1256.0
            * direction;
        commands.set as f32
    };
    let turn_radians = f64::from((difference + extra_turn) / 200.0);
    let rounded_turn = (turn_radians * 1000.0).round_ties_even() / 1000.0;
    // Round returns Double; the subtraction is assigned back into Single nrg.
    robot.materials.energy = (f64::from(robot.materials.energy)
        - (rounded_turn * rotation_cost * multiplier).abs()) as f32;

    // VB Mod converts its floating operands to whole numbers first.
    let base = (desired_units.round_ties_even() as i32).rem_euclid(1256) as f32 / 200.0;
    let mut momentum = robot.kinematics.angular_momentum;
    while momentum > std::f32::consts::TAU {
        momentum -= std::f32::consts::TAU;
    }
    while momentum < -std::f32::consts::TAU {
        momentum += std::f32::consts::TAU;
    }
    robot.set_heading_for_profile(base + momentum, native);
    if momentum > 0.0 && difference < 0.0 {
        momentum = (momentum + (difference + extra_turn) / 200.0).max(0.0);
    }
    if momentum < 0.0 && difference > 0.0 {
        momentum = (momentum + (difference + extra_turn) / 200.0).min(0.0);
    }
    robot.kinematics.angular_momentum = momentum;
    robot.program.memory.write(5, 0);
    robot.program.memory.write(6, 0);
    robot.program.memory.write(18, robot.materials.aim);
    robot.program.memory.write(19, robot.materials.aim);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CycleEvent;

    fn robot(heading: f32, momentum: f32) -> RobotState {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let id = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [500.0; 2])
            .unwrap();
        let mut robot = world.slots.get(id).unwrap().clone();
        robot.set_heading(heading);
        robot.kinematics.angular_momentum = momentum;
        robot
    }

    #[test]
    fn full_native_tick_consumes_dna_aim_after_tie_resource_readback() {
        let mut world = Engine::new_native(EngineConfig {
            world_width: 3_000.0,
            world_height: 2_000.0,
            ..EngineConfig::testing()
        })
        .unwrap();
        let source = world
            .spawn_at(
                LegacyDna::parse(
                    "start 631 .setaim store 11 .tienum store -4 .tieloc store 60 .tieval store stop",
                )
                .unwrap(),
                [1_000.0, 1_000.0],
            )
            .unwrap();
        let target = world
            .spawn_at(
                LegacyDna::parse("start 438 .setaim store stop").unwrap(),
                [1_300.0, 1_000.0],
            )
            .unwrap();
        world
            .ties
            .push(TieState::legacy(source, target, 300.0, 11, 1));
        world.refresh_tie_projections();
        world.slots.get_mut(source).unwrap().materials.waste = 80.0;
        world.set_organism_aim(source, 25).unwrap();
        assert_eq!(world.memory_at(source, MEM_SET_AIM).unwrap(), 25);

        let trace = world.tick_with_trace().unwrap();
        assert!(trace.events.iter().any(|event| {
            matches!(event, CycleEvent::Effect { source: Some(from), target: to, kind, .. }
                if *from == source && *to == target && kind == "tie_resource_waste")
        }));
        for (id, expected) in [(source, 631), (target, 438)] {
            let robot = world.slots.get(id).unwrap();
            assert_eq!(robot.heading(), expected as f32 / 200.0);
            assert_eq!(robot.program.memory.read(18), expected);
            assert_eq!(robot.program.memory.read(MEM_SET_AIM), expected);
        }
    }

    #[test]
    fn genuine_cycle_282_aims_and_tie_angles_match_the_original_capture() {
        // Original 2.48.32 cycle-282 capture SHA-256
        // c49c002f66898a52db81645b78b11ecbb2ed1941dbeea03fe7d53dd3b24b9e91.
        let input = include_bytes!("../../tests/fixtures/db24832-original-user.sim");
        let mut world = Engine::from_native_sim(input).unwrap();
        world.tick().unwrap();
        assert_eq!(world.native_cycle, Some(282));
        for (absolute_id, heading, aim, tie_angle) in [
            (202, 0.6875525_f32, 138, -1),
            (223, 2.6895747, 538, 1),
            (275, 1.4810013, 296, -1),
        ] {
            let robot = world
                .slots
                .iter()
                .filter_map(|slot| slot.organism.as_ref())
                .find(|robot| robot.identity.absolute_id == absolute_id)
                .unwrap();
            assert_eq!(robot.heading(), heading, "absolute ID {absolute_id}");
            assert_eq!(robot.program.memory.read(18), aim);
            assert_eq!(robot.program.memory.read(MEM_SET_AIM), aim);
            assert_eq!(robot.program.memory.read(MEM_TIE_ANGLE), tie_angle);
        }
    }

    #[test]
    fn native_aim_refresh_rounds_the_double_trig_results_to_single() {
        // Original Darwinbots 2.48.32 cycle 293, absolute robot 263.
        let heading = f32::from_bits(0x4056_bbfc);
        let base = 671.0_f32 / 200.0;
        let momentum = heading - base;
        for (native, expected) in [
            (false, [heading.cos().to_bits(), heading.sin().to_bits()]),
            (true, [0xbf7a_2e35, 0xbe59_1915]),
        ] {
            let mut current = robot(base, momentum);
            apply(
                &mut current,
                AimCommands {
                    set: 671,
                    ..Default::default()
                },
                0.0,
                1.0,
                native,
            );
            assert_eq!(current.heading(), heading);
            assert_eq!(current.kinematics.aim_vector.map(f32::to_bits), expected);
        }
    }

    #[test]
    fn native_aim_rounds_the_combined_single_before_mod_and_uses_1256() {
        let mut current = robot(0.0025, 0.0);
        apply(
            &mut current,
            AimCommands {
                turn_left: 1,
                ..Default::default()
            },
            0.0,
            1.0,
            true,
        );
        assert_eq!(current.heading(), 0.01);
        assert_eq!(current.program.memory.read(19), 2);
        apply(
            &mut current,
            AimCommands {
                set: 1256,
                ..Default::default()
            },
            0.0,
            1.0,
            true,
        );
        assert_eq!(current.heading(), 0.0);
        apply(
            &mut current,
            AimCommands {
                set: -1,
                ..Default::default()
            },
            0.0,
            1.0,
            true,
        );
        assert_eq!(current.heading(), 6.275);
    }

    #[test]
    fn native_aim_keeps_full_turn_momentum_and_only_opposing_turns_reduce_it() {
        let mut current = robot(0.0, 2.0);
        apply(&mut current, AimCommands::default(), 0.0, 1.0, true);
        assert_eq!(current.heading(), 2.0);
        assert_eq!(current.kinematics.angular_momentum, 2.0);
        apply(
            &mut current,
            AimCommands {
                set: 400,
                turn_right: 200,
                ..Default::default()
            },
            0.0,
            1.0,
            true,
        );
        assert_eq!(
            current.heading(),
            3.0,
            "momentum is added before opposing reduction"
        );
        assert_eq!(current.kinematics.angular_momentum, 1.0);
        apply(
            &mut current,
            AimCommands {
                set: 600,
                turn_left: 100,
                ..Default::default()
            },
            0.0,
            1.0,
            true,
        );
        assert_eq!(
            current.kinematics.angular_momentum, 1.0,
            "voluntary turn adds no momentum"
        );
    }

    #[test]
    fn native_turn_fee_uses_net_radians_and_absolute_cost_product() {
        let mut current = robot(0.0, 0.0);
        current.materials.energy = 100.0;
        apply(
            &mut current,
            AimCommands {
                turn_left: 100,
                turn_right: 100,
                ..Default::default()
            },
            20.0,
            1.0,
            true,
        );
        assert_eq!(current.materials.energy, 100.0, "opposite requests cancel");
        apply(
            &mut current,
            AimCommands {
                turn_left: 200,
                ..Default::default()
            },
            -20.0,
            1.0,
            true,
        );
        assert_eq!(
            current.materials.energy, 80.0,
            "one radian is charged even for a negative cost product"
        );
        let mut same_direction = robot(std::f32::consts::TAU, 0.0);
        same_direction.materials.energy = 100.0;
        apply(&mut same_direction, AimCommands::default(), 20.0, 1.0, true);
        assert_eq!(same_direction.materials.energy, 100.0, "VB Sgn(0) is zero");
    }

    #[test]
    fn dna_setaim_1256_runs_through_the_world_and_persists_the_native_zero_readbacks() {
        let mut world = Engine::new(EngineConfig {
            brownian_motion: 0.0,
            ..EngineConfig::testing()
        })
        .unwrap();
        let id = world
            .spawn_at(
                LegacyDna::parse("start 1256 .setaim store stop").unwrap(),
                [500.0; 2],
            )
            .unwrap();
        world.tick().unwrap();
        assert_eq!(world.slots.get(id).unwrap().heading(), 0.0);
        assert_eq!(world.memory_at(id, 18).unwrap(), 0);
        assert_eq!(world.memory_at(id, 19).unwrap(), 0);
        let restored = crate::SaveFile::decode(&crate::SaveFile::encode(&world).unwrap()).unwrap();
        assert_eq!(restored.slots.get(id).unwrap().heading(), 0.0);
    }
}
