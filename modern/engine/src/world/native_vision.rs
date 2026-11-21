//! Quads.bas vision candidate order and reference selection regressions.
use super::*;

impl Engine {
    pub(super) fn vision_target_slots(
        &self,
        observer_slot: usize,
        position: [f32; 2],
        radius: f32,
    ) -> Vec<usize> {
        let mut candidates = self.sensing_spatial.neighbors(position, radius);
        if self.native_random.is_some() {
            let observer =
                OrganismId::new(observer_slot as u32, self.slots[observer_slot].generation);
            self.native_buckets
                .order_vision_candidates(observer, &mut candidates);
        }
        candidates
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eye_value;
    use crate::world::native_pipeline::NativeRobotPhase;

    fn world() -> Engine {
        Engine::new(EngineConfig {
            world_width: 16_000.0,
            world_height: 12_000.0,
            ..EngineConfig::testing()
        })
        .unwrap()
    }

    fn bot(world: &mut Engine, position: [f32; 2], radius: f32) -> OrganismId {
        let id = world
            .spawn_at(LegacyDna::parse("start stop").unwrap(), position)
            .unwrap();
        world.slots.get_mut(id).unwrap().materials.radius = radius;
        world.set_organism_aim(id, 0).unwrap();
        id
    }

    fn sense(world: &mut Engine) {
        world.clear_after_dna_phase();
        world.native_buckets.synchronize(
            [world.config.world_width, world.config.world_height],
            &world.slots,
        );
        world.spatial_index_phase();
        world.sensing_phase().unwrap();
    }

    fn request_energy_steal(
        world: &mut Engine,
        thief: OrganismId,
        victim: OrganismId,
        amount: i32,
    ) {
        world
            .ties
            .push(TieState::legacy(thief, victim, 500.0, 1, 1));
        let thief_state = world.slots.get_mut(thief).unwrap();
        thief_state.counters.age = 1;
        thief_state.program.memory.write(455, 1);
        thief_state.program.memory.write(452, -1);
        thief_state.program.memory.write(453, -amount);
    }

    #[test]
    fn native_energy_steal_leaves_the_victims_loaded_radius_visible_to_an_earlier_robot() {
        let mut world = world();
        let observer = bot(&mut world, [1000.0, 1000.0], 150.0);
        let thief = bot(&mut world, [1500.0, 1500.0], 100.0);
        let victim = bot(&mut world, [1500.0, 1000.0], 100.0);
        world.set_organism_energy(victim, 50).unwrap();
        world.slots.get_mut(victim).unwrap().materials.chloroplasts = 29_269.0;
        request_energy_steal(&mut world, thief, victim, 25);

        world.tie_exchange_phase(&mut CycleRecorder::disabled(0));
        assert_eq!(world.slots.get(victim).unwrap().materials.energy, 25.0);
        assert_eq!(world.slots.get(victim).unwrap().materials.radius, 100.0);
        assert!(world.slots.get(thief).unwrap().materials.radius > 100.0);
        sense(&mut world);
        assert_eq!(world.memory_at(observer, MEM_EYE1 + 4).unwrap(), 31);

        // Once the victim reaches ManageChlr, its chloroplasts count toward size.
        world.native_phase = Some(NativeRobotPhase {
            actor: victim,
            cost_population: 3,
            chloroplast_total: 29_269.0,
            radius_bound: 415.0,
        });
        world.chloroplast_phase();
        world.native_phase = None;
        sense(&mut world);
        assert_eq!(world.memory_at(observer, MEM_EYE1 + 4).unwrap(), 32_000);
    }

    #[test]
    fn native_energy_steals_publish_kills_before_death_is_processed() {
        let mut world = world();
        let observer = bot(&mut world, [1000.0, 1000.0], 100.0);
        let first_thief = bot(&mut world, [1500.0, 1000.0], 100.0);
        let second_thief = bot(&mut world, [1500.0, 1500.0], 100.0);
        let victim = bot(&mut world, [2000.0, 1000.0], 100.0);
        world.set_organism_energy(victim, 25).unwrap();
        request_energy_steal(&mut world, first_thief, victim, 25);
        request_energy_steal(&mut world, second_thief, victim, 25);

        world.tie_exchange_phase(&mut CycleRecorder::disabled(0));
        assert_eq!(world.slots.get(victim).unwrap().materials.energy, 0.0);
        assert!(!world.slots.get(victim).unwrap().flags.dead);
        assert_eq!(world.memory_at(first_thief, 220).unwrap(), 1);
        assert_eq!(world.memory_at(second_thief, 220).unwrap(), 1);
        sense(&mut world);
        assert_eq!(world.memory_at(observer, MEM_REF_KILLS).unwrap(), 1);
    }

    #[test]
    fn native_energy_steal_preserves_previous_cycle_energy_for_pain_and_pleasure() {
        let mut world = world();
        let thief = bot(&mut world, [1000.0, 1000.0], 100.0);
        let victim = bot(&mut world, [1500.0, 1000.0], 100.0);
        // LoadRobotBody leaves the previous-cycle energy at zero on load.
        for (id, energy) in [(thief, 1000), (victim, 200)] {
            world.set_organism_energy(id, energy).unwrap();
            world.slots.get_mut(id).unwrap().materials.previous_energy = 0.0;
        }
        request_energy_steal(&mut world, thief, victim, 100);

        world.tie_exchange_phase(&mut CycleRecorder::disabled(0));
        world.publish_core_senses_phase();
        for (id, energy) in [(thief, 1070), (victim, 100)] {
            assert_eq!(world.memory_at(id, 203).unwrap(), -energy);
            assert_eq!(world.memory_at(id, 204).unwrap(), energy);
        }
    }

    #[test]
    fn native_eye_distance_matches_original_near_contact() {
        let mut world = world();
        // Darwinbots 2.48.32 cycle 330, absolute robots 11 and 154.
        let observer = bot(
            &mut world,
            [f32::from_bits(0x457a_baec), f32::from_bits(0x4524_2bda)],
            74.352_38,
        );
        let target = bot(
            &mut world,
            [f32::from_bits(0x4581_00da), f32::from_bits(0x452a_bcfa)],
            79.186_82,
        );
        world
            .slots
            .get_mut(observer)
            .unwrap()
            .set_heading(f32::from_bits(0x408f_36de));
        world
            .slots
            .get_mut(target)
            .unwrap()
            .set_heading(f32::from_bits(0x403d_2767));

        sense(&mut world);
        for eye in [0, 1] {
            assert_eq!(world.memory_at(observer, MEM_EYE1 + eye).unwrap(), 11_746);
        }
        for eye in 5..=8 {
            assert_eq!(world.memory_at(target, MEM_EYE1 + eye).unwrap(), 11_746);
        }
    }

    #[test]
    fn equal_integer_eye_strength_uses_bucket_order_before_slot_order() {
        let mut world = world();
        let observer = bot(&mut world, [3990.0, 3999.0], 100.0);
        let radius = (1000.0_f32 * 1000.0 + 1.0).sqrt() - 100.0 - 710.0;
        bot(&mut world, [4990.0, 4000.0], radius);
        bot(&mut world, [4990.0, 3998.0], radius);
        sense(&mut world);
        assert_eq!(world.memory_at(observer, MEM_EYE1 + 4).unwrap(), 4);
        assert_eq!(world.memory_at(observer, MEM_REF_Y).unwrap(), 3998);
    }

    #[test]
    fn vision_keeps_packed_bucket_reentry_order_for_equal_integer_strength() {
        let mut world = world();
        let observer = bot(&mut world, [1000.0; 2], 10.0);
        let first = bot(&mut world, [1730.0, 1000.0], 10.0);
        let second = bot(&mut world, [1730.0, 1000.0], 10.0);
        world.set_organism_energy(second, 2000).unwrap();
        world
            .native_buckets
            .synchronize([16_000.0, 12_000.0], &world.slots);
        world.native_buckets.relocate(first, [9000.0, 1000.0]);
        world.native_buckets.relocate(first, [1730.0, 1000.0]);
        sense(&mut world);
        assert_eq!(world.memory_at(observer, MEM_REF_NRG).unwrap(), 2000);
    }

    #[test]
    fn later_fractional_strength_compares_against_rounded_memory_not_the_prior_float() {
        let mut world = world();
        let observer = bot(&mut world, [1000.0; 2], 10.0);
        let near_edge = 1440.0 / 4.25_f32.sqrt() - 10.0;
        let far_edge = 1440.0 / 4.1_f32.sqrt() - 10.0;
        assert_eq!(eye_value(1440.0, near_edge), 4);
        assert_eq!(eye_value(1440.0, far_edge), 4);
        bot(&mut world, [1020.0 + near_edge, 1000.0], 10.0);
        let farther_x = 1020.0 + far_edge;
        bot(&mut world, [farther_x, 1000.0], 10.0);
        sense(&mut world);
        assert_eq!(world.memory_at(observer, MEM_EYE1 + 4).unwrap(), 4);
        assert_eq!(
            world.memory_at(observer, MEM_REF_X).unwrap(),
            farther_x.round_ties_even() as i32
        );
    }

    #[test]
    fn cancelling_eye_widths_retain_the_native_1440_target_gate() {
        let mut world = world();
        let observer = bot(&mut world, [1000.0; 2], 10.0);
        bot(&mut world, [3100.0, 1000.0], 10.0);
        world.set_organism_memory(observer, 535, -34).unwrap();
        world.set_organism_memory(observer, 531, 34).unwrap();
        let vision =
            VisionSnapshot::from_memory(&world.slots.get(observer).unwrap().program.memory, 0);
        assert!(vision.max_range() > 2080.0);
        assert_eq!(vision.native_target_range(1.0), 1440.0);
        sense(&mut world);
        assert_eq!(world.memory_at(observer, MEM_EYE1 + 4).unwrap(), 0);
        assert_eq!(world.memory_at(observer, MEM_REF_X).unwrap(), 0);
    }

    #[test]
    fn giant_visible_targets_outside_adjacent_native_buckets_are_not_candidates() {
        let mut world = world();
        let observer = bot(&mut world, [1000.0; 2], 100.0);
        let remote = bot(&mut world, [9000.0, 1000.0], 7000.0);
        world
            .native_buckets
            .synchronize([16_000.0, 12_000.0], &world.slots);
        world.spatial_index_phase();
        assert!(
            world
                .sensing_spatial
                .neighbors([1000.0; 2], 8540.0)
                .contains(&(remote.slot() as usize))
        );
        world.sensing_phase().unwrap();
        assert_eq!(world.memory_at(observer, MEM_REF_X).unwrap(), 0);
    }
}
