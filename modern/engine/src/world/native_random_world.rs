use super::*;

impl Engine {
    pub(crate) fn has_native_random(&self) -> bool {
        self.native_random.is_some()
    }
    pub(super) fn native_placement_position(
        &mut self,
        species: SpeciesId,
        legacy_seed: u64,
    ) -> [f32; 2] {
        if let Some(random) = self.native_random.as_mut() {
            let bounds = self.species[species.0 as usize].placement_bounds;
            let x_extent = self.config.world_width - 60.0;
            let y_extent = self.config.world_height - 60.0;
            return [
                random.rounded_integer(
                    (bounds[0] * x_extent).round_ties_even() as i32,
                    (bounds[1] * x_extent).round_ties_even() as i32,
                ) as f32,
                random.rounded_integer(
                    (bounds[2] * y_extent).round_ties_even() as i32,
                    (bounds[3] * y_extent).round_ties_even() as i32,
                ) as f32,
            ];
        }
        [
            unit_coordinate(legacy_seed, self.config.world_width),
            unit_coordinate(advance_random(legacy_seed), self.config.world_height),
        ]
    }

    pub(super) fn native_placement_heading(&mut self, id: OrganismId) -> Result<(), EngineError> {
        if let Some(random) = self.native_random.as_mut() {
            let angle = random.unit() * std::f32::consts::PI * 2.0;
            let state = u64::from(random.state());
            let robot = self.slots.get_mut(id)?;
            robot.set_heading(angle);
            robot.program.rng_state = state;
        }
        Ok(())
    }

    pub(super) fn refresh_native_chloroplast_population(&mut self) -> usize {
        // Master.bas accumulates AllChlr into a Long after each Single add.
        let sum = self
            .slots
            .iter()
            .filter_map(|slot| slot.organism.as_ref())
            .filter(|robot| !robot.flags.hidden)
            .fold(0_i64, |total, robot| {
                (total as f64 + f64::from(robot.materials.chloroplasts.max(0.0))).round_ties_even()
                    as i64
            });
        let population = (sum as f64 / 16_000.0).round_ties_even().max(0.0) as usize;
        self.vegetation_runtime
            .set_native_chloroplast_population(population);
        population
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::native_random::NativeRandom;

    #[test]
    fn dna_random_draws_share_one_native_stream_in_slot_order() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let dna = LegacyDna::parse("start 100 rnd 900 store stop").unwrap();
        let first = world.spawn_at(dna.clone(), [100.0; 2]).unwrap();
        let second = world.spawn_at(dna, [700.0; 2]).unwrap();
        let mut expected = NativeRandom::with_user_seed(world.config.seed);
        let first_value = expected.integer(0, 100);
        let second_value = expected.integer(0, 100);
        world
            .execute_dna_phase(&mut CycleRecorder::disabled(1))
            .unwrap();
        assert_eq!(world.memory_at(first, 900).unwrap(), first_value);
        assert_eq!(world.memory_at(second, 900).unwrap(), second_value);
        assert_eq!(
            world.native_random.as_ref().unwrap().state(),
            expected.state()
        );
    }

    #[test]
    fn save_resume_preserves_the_global_stream_across_later_dna_draws() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let id = world
            .spawn_at(
                LegacyDna::parse("start 32000 rnd 900 store stop").unwrap(),
                [500.0; 2],
            )
            .unwrap();
        world
            .execute_dna_phase(&mut CycleRecorder::disabled(1))
            .unwrap();
        let mut resumed =
            crate::SaveFile::decode(&crate::SaveFile::encode(&world).unwrap()).unwrap();
        for cycle in 2..12 {
            world
                .execute_dna_phase(&mut CycleRecorder::disabled(cycle))
                .unwrap();
            resumed
                .execute_dna_phase(&mut CycleRecorder::disabled(cycle))
                .unwrap();
            assert_eq!(
                world.memory_at(id, 900).unwrap(),
                resumed.memory_at(id, 900).unwrap()
            );
            assert_eq!(world.native_random, resumed.native_random);
        }
    }
    #[test]
    fn native_placement_interleaves_coordinates_heading_and_later_vm_draws() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let species = world.register_species(SpeciesDefinition {
            placement_bounds: [0.25, 0.5, 0.5, 0.75],
            ..Default::default()
        });
        let mut expected = world.native_random.clone().unwrap();
        let xlo = (0.25 * (world.config.world_width - 60.0)).round_ties_even() as i32;
        let xhi = (0.5 * (world.config.world_width - 60.0)).round_ties_even() as i32;
        let ylo = (0.5 * (world.config.world_height - 60.0)).round_ties_even() as i32;
        let yhi = (0.75 * (world.config.world_height - 60.0)).round_ties_even() as i32;
        let mut positions = Vec::new();
        let mut headings = Vec::new();
        for _ in 0..2 {
            positions.push([
                expected.rounded_integer(xlo, xhi) as f32,
                expected.rounded_integer(ylo, yhi) as f32,
            ]);
            headings.push(expected.unit() * std::f32::consts::PI * 2.0);
        }
        let ids = world
            .spawn_species_batch_with_native_placement(
                LegacyDna::parse("start 100 rnd 900 store stop").unwrap(),
                species,
                vec![[0.0; 2]; 2],
                3000,
                true,
            )
            .unwrap();
        for index in 0..2 {
            assert_eq!(
                world.slots.get(ids[index]).unwrap().kinematics.position,
                positions[index]
            );
            assert_eq!(
                world.slots.get(ids[index]).unwrap().heading(),
                headings[index]
            );
        }
        let first = expected.integer(0, 100);
        let second = expected.integer(0, 100);
        world
            .execute_dna_phase(&mut CycleRecorder::disabled(1))
            .unwrap();
        assert_eq!(world.memory_at(ids[0], 900).unwrap(), first);
        assert_eq!(world.memory_at(ids[1], 900).unwrap(), second);
        assert_eq!(
            world.native_random.as_ref().unwrap().state(),
            expected.state()
        );
    }

    #[test]
    fn reproduction_keeps_two_requests_and_zero_timer_sperm_until_next_cycle() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let dna = LegacyDna::parse("start stop").unwrap();
        let id = world.spawn_at(dna.clone(), [500.0; 2]).unwrap();
        let robot = world.slots.get_mut(id).unwrap();
        robot.program.memory.write(300, 50);
        robot.program.memory.write(302, 25);
        robot.effects.stored_sperm = Some(StoredGeneticMaterial { dna, source: None });
        robot.effects.fertilization_timer = 1;
        world.advance_reproductive_timers();
        let queues = world.prepare_lifecycle_queues(&mut CycleRecorder::disabled(1));
        assert_eq!(queues.births.len(), 2);
        assert!(!queues.births[0].sexual);
        assert!(queues.births[1].sexual);
        assert_eq!(world.memory_at(id, 303).unwrap(), 0);
        assert!(world.slots.get(id).unwrap().effects.stored_sperm.is_some());
        world.advance_reproductive_timers();
        world.advance_reproductive_timers();
        assert!(world.slots.get(id).unwrap().effects.stored_sperm.is_none());
    }
}
