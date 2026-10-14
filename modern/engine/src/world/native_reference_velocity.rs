//! Senses.bas lookoccurr and Ties.bas ReadTRefVars numeric boundaries.
use crate::biology::resource_readback;

pub(super) fn optical_velocity(
    target: [f32; 2],
    heading: f32,
    own_up: i32,
    own_dx: i32,
    previous_sx: i32,
) -> [i32; 5] {
    let angle = f64::from(heading);
    // lookoccurr declares X/Y Single before assigning the Integer memory cells.
    let up = (f64::from(target[0]) * angle.cos()
        - f64::from(target[1]) * angle.sin()
        - f64::from(own_up)) as f32;
    let dx = (f64::from(target[1]) * angle.cos() + f64::from(target[0]) * angle.sin()
        - f64::from(own_dx)) as f32;
    let up = resource_readback(up.clamp(-32000.0, 32000.0));
    let dx = resource_readback(dx.clamp(-32000.0, 32000.0));
    let scalar = ((f64::from(up).powi(2) + f64::from(dx).powi(2)).sqrt() as f32).min(32000.0);
    // The source negates the prior SX cell, rather than the newly calculated DX.
    [resource_readback(scalar), -previous_sx, dx, -up, up]
}

pub(super) fn tie_relative_velocity(
    target: [f32; 2],
    heading: f32,
    own_up: i32,
    own_dx: i32,
) -> [i32; 4] {
    let target = target.map(|component| component.clamp(-16000.0, 16000.0));
    let angle = f64::from(heading);
    // Ties assigns the Double expression straight to Integer memory, without X/Y Single.
    let integer = |value: f64| value.round_ties_even() as i32;
    let up = integer(
        f64::from(target[0]) * angle.cos() - f64::from(target[1]) * angle.sin() - f64::from(own_up),
    );
    let dx = integer(
        f64::from(target[1]) * angle.cos() + f64::from(target[0]) * angle.sin() - f64::from(own_dx),
    );
    [-dx, dx, -up, up]
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use super::*;

    fn pair() -> (Engine, OrganismId, OrganismId) {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let observer = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [500.0, 500.0])
            .unwrap();
        let target = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [1000.0, 500.0])
            .unwrap();
        world.slots.get_mut(observer).unwrap().set_heading(0.0);
        (world, observer, target)
    }

    #[test]
    fn focus_eye_publishes_relative_velocity_and_scalar_through_save_resume() {
        let (mut world, observer, target) = pair();
        let robot = world.slots.get_mut(observer).unwrap();
        robot.kinematics.velocity = [4.49, 3.49];
        robot.program.memory.write(MEM_VEL_UP, 4);
        robot.program.memory.write(MEM_VEL_DX, 3);
        robot.program.memory.write(MEM_REF_VEL_SX, 17);
        world.slots.get_mut(target).unwrap().kinematics.velocity = [5.51, 3.51];
        world.spatial_index_phase();
        world.sensing_phase().unwrap();
        for (address, value) in [(695, 2), (696, -17), (697, 1), (698, -2), (699, 2)] {
            assert_eq!(world.memory_at(observer, address).unwrap(), value);
        }
        let mut resumed =
            crate::SaveFile::decode(&crate::SaveFile::encode(&world).unwrap()).unwrap();
        resumed.spatial_index_phase();
        resumed.sensing_phase().unwrap();
        assert_eq!(resumed.memory_at(observer, MEM_REF_VEL_SX).unwrap(), 17);
        assert_eq!(resumed.memory_at(observer, MEM_REF_VEL_SCALAR).unwrap(), 2);
        resumed.slots.get_mut(target).unwrap().kinematics.position = [4000.0, 4000.0];
        resumed.spatial_index_phase();
        resumed.sensing_phase().unwrap();
        for address in 695..=699 {
            assert_eq!(resumed.memory_at(observer, address).unwrap(), 0);
        }
    }

    #[test]
    fn tie_reference_copies_published_target_cells_and_clamps_physical_motion() {
        let (mut world, observer, target) = pair();
        let robot = world.slots.get_mut(observer).unwrap();
        robot.counters.simulation_age = 2;
        robot.program.memory.write(MEM_VEL_UP, 4);
        robot.program.memory.write(MEM_VEL_DX, 3);
        let target_robot = world.slots.get_mut(target).unwrap();
        target_robot.counters.simulation_age = 2;
        target_robot.kinematics.velocity = [20000.0, -20000.0];
        for (address, value) in [(196, 13), (197, -8), (198, 8), (199, -10), (200, 10)] {
            target_robot.program.memory.write(address, value);
        }
        world
            .ties
            .push(TieState::new(observer, target, 500.0, 7, 8));
        world.refresh_tie_projections_with_references(true, &HashMap::new());
        assert_eq!(
            world.slots.get(target).unwrap().kinematics.velocity,
            [16000.0, -16000.0]
        );
        for (address, value) in [
            (440, 16003),
            (441, -16003),
            (442, -15996),
            (443, 15996),
            (444, 13),
            (445, -8),
            (446, 8),
            (447, -10),
            (448, 10),
        ] {
            assert_eq!(world.memory_at(observer, address).unwrap(), value);
        }
    }

    #[test]
    fn tie_integer_assignment_preserves_values_above_the_dna_store_limit() {
        let (mut world, observer, target) = pair();
        let robot = world.slots.get_mut(observer).unwrap();
        robot.counters.simulation_age = 2;
        robot.program.memory.write(MEM_VEL_UP, -16627);
        world.slots.get_mut(target).unwrap().kinematics.velocity = [16000.0, 0.0];
        world
            .ties
            .push(TieState::new(observer, target, 500.0, 7, 8));
        world.refresh_tie_projections_with_references(true, &HashMap::new());
        assert_eq!(world.memory_at(observer, 443).unwrap(), 32627);
        assert_eq!(world.memory_at(observer, 442).unwrap(), -32627);
    }
    #[test]
    fn optical_reference_subtracts_rounded_observer_cells_not_raw_velocity() {
        assert_eq!(
            optical_velocity([5.51, 3.51], 0.0, 4, 3, 7),
            [2, -7, 1, -2, 2]
        );
    }
    #[test]
    fn optical_scalar_uses_the_rounded_components() {
        assert_eq!(
            optical_velocity([0.49, 0.49], 0.0, 0, 0, 0),
            [0, 0, 0, 0, 0]
        );
    }
    #[test]
    fn optical_sx_negates_its_prior_value_on_each_lookup() {
        let first = optical_velocity([10.0, 5.0], 0.0, 0, 0, 17);
        let second = optical_velocity([10.0, 5.0], 0.0, 0, 0, first[1]);
        assert_eq!(first[1], -17);
        assert_eq!(second[1], 17);
    }
    #[test]
    fn optical_reference_is_signed_and_clamped_before_scalar_calculation() {
        assert_eq!(
            optical_velocity([40000.0, -40000.0], 0.0, 0, 0, 0),
            [32000, 0, -32000, -32000, 32000]
        );
    }
    #[test]
    fn tie_reference_uses_direct_integer_projection_and_the_native_velocity_limit() {
        assert_eq!(
            tie_relative_velocity([5.51, 3.51], 0.0, 4, 3),
            [-1, 1, -2, 2]
        );
        assert_eq!(
            tie_relative_velocity([20000.0, -20000.0], 0.0, 0, 0),
            [16000, -16000, -16000, 16000]
        );
    }
}
