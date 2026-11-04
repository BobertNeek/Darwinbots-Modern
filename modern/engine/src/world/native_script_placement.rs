//! Globals.aggiungirob: script loading precedes the final placement heading.

use super::*;

impl Engine {
    pub(super) fn spawn_native_script_robot(
        &mut self,
        dna: LegacyDna,
        species: SpeciesId,
        position: [f32; 2],
    ) -> Result<OrganismId, EngineError> {
        if self.native_random.is_none() {
            let id = self.spawn_species_at_unpublished(dna, species, position)?;
            self.native_placement_heading(id)?;
            return Ok(id);
        }
        if self.available_robot_capacity() == 0 {
            return Err(EngineError::CapacityReached);
        }
        if self.species.get(species.0 as usize).is_none() {
            return Err(EngineError::Invariant("species does not exist".into()));
        }

        let random = self.native_random.as_mut().unwrap();
        // Module1.preparerob consumes two discarded positions, one discrete
        // heading and three discarded colors before aggiungirob overwrites them.
        random.unit();
        random.unit();
        let preparation_word = (random.unit() * 629.0).floor() as i32;
        let preparation_heading = (f64::from(preparation_word) / 100.0) as f32;
        for _ in 0..3 {
            random.unit();
        }
        let id = self.spawn_species_at_unpublished(dna, species, position)?;
        let random = self.native_random.as_mut().unwrap();
        let heading = random.unit() * std::f32::consts::PI * 2.0;
        let state = u64::from(random.state());

        let robot = self.slots.get_mut(id)?;
        robot.program.memory = VmMemory::default();
        robot
            .program
            .memory
            .write(MEM_FIXED, i32::from(robot.flags.fixed));
        robot.set_heading(heading);
        // aggiungirob changes aim without updating preparerob's aimvector.
        // VB6 Cos/Sin return Double before VectorSet narrows to Single.
        robot.kinematics.aim_vector = [
            f64::from(preparation_heading).cos() as f32,
            f64::from(preparation_heading).sin() as f32,
        ];
        robot.program.memory.write(19, robot.materials.aim);
        robot
            .program
            .memory
            .write(MEM_DNA_LENGTH, robot.program.dna.blocks().len() as i32);
        robot
            .program
            .memory
            .write(MEM_GENES, robot.program.dna.gene_count());
        // Senses.makeoccurrlist explicitly republishes ownvars 721..731.
        robot.refresh_occurrence_census();
        robot.program.rng_state = state;
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_script_placement_consumes_seven_draws_before_public_dna() {
        let mut world = Engine::new_native(EngineConfig {
            seed: 42,
            ..EngineConfig::testing()
        })
        .unwrap();
        assert_eq!(world.native_random.as_ref().unwrap().state(), 0x0008d82a);
        let id = world
            .spawn_native_script_robot(
                LegacyDna::parse("start 32000 rnd 900 store stop").unwrap(),
                SpeciesId::default(),
                [321.0, 654.0],
            )
            .unwrap();
        let robot = world.slots.get(id).unwrap();
        assert_eq!(robot.kinematics.position, [321.0, 654.0]);
        assert_eq!(robot.heading().to_bits(), 0x3f8b0382);
        assert_eq!(robot.program.memory.read(19), 217);
        assert_eq!(robot.program.rng_state, 0x002c3fdb);
        assert_eq!(world.native_random.as_ref().unwrap().state(), 0x002c3fdb);
        assert_eq!(
            robot.kinematics.aim_vector,
            [
                f64::from(3.53_f32).cos() as f32,
                f64::from(3.53_f32).sin() as f32,
            ]
        );
        world.execute_robots().unwrap();
        assert_eq!(world.memory_at(id, 900).unwrap(), 20_506);
        assert_eq!(world.native_random.as_ref().unwrap().state(), 0x00a40b32);
    }

    #[test]
    fn script_memory_erase_keeps_materials_but_only_publishes_source_cells() {
        let mut world = Engine::new_native(EngineConfig {
            seed: 42,
            ..EngineConfig::testing()
        })
        .unwrap();
        let species = world.register_species(SpeciesDefinition {
            vegetable: true,
            restrictions: Some(RestrictionColumn {
                fixed_in_place: true,
                ..RestrictionColumn::default()
            }),
            ..SpeciesDefinition::default()
        });
        let dna = LegacyDna::parse("start 1 .up store stop").unwrap();
        let dna_length = dna.blocks().len() as i32;
        let id = world
            .spawn_native_script_robot(dna, species, [321.0, 654.0])
            .unwrap();
        let robot = world.slots.get(id).unwrap();
        assert_eq!(robot.materials.energy, 1_000.0);
        assert_eq!(robot.materials.body, 1_000.0);
        assert_eq!(
            robot.materials.chloroplasts,
            world.config.vegetation.start_chloroplasts as f32
        );
        for address in [10, 12, 18, 215, 217, 219, 310, 311, 468, 920] {
            assert_eq!(robot.program.memory.read(address), 0, "cell {address}");
        }
        assert_eq!(robot.program.memory.read(216), 1);
        assert_eq!(robot.program.memory.read(19), 217);
        assert_eq!(robot.program.memory.read(336), dna_length);
        assert_eq!(robot.program.memory.read(339), 1);
        assert_eq!(robot.program.memory.read(721), 1);
        assert_eq!(robot.senses.vision_reference_occurrences[0], 1);
        let expected_cells = [19, 216, 336, 339, 721];
        assert_eq!(
            robot
                .program
                .memory
                .entries()
                .map(|(address, _)| i32::from(address))
                .collect::<Vec<_>>(),
            expected_cells
        );
    }

    #[test]
    fn legacy_script_placement_keeps_the_existing_spawn_state() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        world.native_random = None;
        let mut control = Engine::new(EngineConfig::testing()).unwrap();
        control.native_random = None;
        let dna = LegacyDna::parse("start 1 .up store stop").unwrap();
        let inserted = world
            .spawn_native_script_robot(dna.clone(), SpeciesId::default(), [321.0, 654.0])
            .unwrap();
        let expected = control
            .spawn_species_at_unpublished(dna, SpeciesId::default(), [321.0, 654.0])
            .unwrap();
        assert_eq!(
            serde_json::to_value(world.slots.get(inserted).unwrap()).unwrap(),
            serde_json::to_value(control.slots.get(expected).unwrap()).unwrap()
        );
        assert_eq!(world.memory_at(inserted, MEM_NRG).unwrap(), 1_000);
        assert_eq!(world.memory_at(inserted, 468).unwrap(), 32_000);
        assert!(world.native_random.is_none());
    }

    #[test]
    fn unsuccessful_native_repopulation_still_consumes_the_caller_coordinates() {
        let mut world = Engine::new_native(EngineConfig {
            seed: 42,
            vegetation: VegetationSettings {
                minimum_chloroplast_equivalents: 1,
                repopulation_amount: 2,
                repopulation_cooldown: 1,
                ..VegetationSettings::default()
            },
            ..EngineConfig::testing()
        })
        .unwrap();
        assert_eq!(world.native_random.as_ref().unwrap().state(), 0x0008d82a);
        world.vegetation_repopulation_phase().unwrap();
        assert_eq!(world.population(), 0);
        assert_eq!(world.stats.reseeds, 0);
        assert_eq!(world.native_random.as_ref().unwrap().state(), 0x0034a60e);
        assert_eq!(
            serde_json::to_value(&world.vegetation_runtime).unwrap()["repopulation_counter"],
            0
        );
    }
}
