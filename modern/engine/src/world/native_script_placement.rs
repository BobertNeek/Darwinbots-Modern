//! Globals.aggiungirob: script loading precedes the final placement heading.

use super::*;
use crate::native_binary::{NativeMutationSpareEntries, NativeSimulationTail};

impl Engine {
    pub(super) fn native_script_origin_tail(
        &self,
    ) -> Result<Option<NativeSimulationTail>, EngineError> {
        self.native_origin
            .as_ref()
            .filter(|origin| self.native_random.is_some() && !origin.tail.is_empty())
            .map(|origin| NativeSimulationTail::decode(&origin.tail))
            .transpose()
    }

    pub(super) fn spawn_native_script_robot(
        &mut self,
        dna: LegacyDna,
        species: SpeciesId,
        position: [f32; 2],
        origin: Option<&NativeSimulationTail>,
    ) -> Result<OrganismId, EngineError> {
        if self.native_random.is_none() {
            let id = self.spawn_species_at_unpublished(dna, species, position)?;
            self.native_placement_heading(id)?;
            return Ok(id);
        }
        if self.available_robot_capacity() == 0 {
            return Err(EngineError::CapacityReached);
        }
        let Some(definition) = self.species.get(species.0 as usize) else {
            return Err(EngineError::Invariant("species does not exist".into()));
        };
        let spare_entries = script_mutation_spare_entries(origin, definition)?;

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
        robot.native_mutation_spare_entries = spare_entries.map(Box::new);
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

fn script_mutation_spare_entries(
    origin: Option<&NativeSimulationTail>,
    definition: &SpeciesDefinition,
) -> Result<Option<NativeMutationSpareEntries>, EngineError> {
    let (Some(origin), Some(index)) = (origin, definition.lineage_id.checked_sub(1)) else {
        return Ok(None);
    };
    if index >= origin.integer("species_count")? as u64 {
        return Ok(None);
    }
    let row = format!("species[{index}].");
    if native_io::text(origin.bytes(&format!("{row}name"))?) != definition.name {
        return Ok(None);
    }
    // aggiungirob copies the entire species Mutables. The live profile already
    // supplies modeled values; only the unmodeled fields come from the save.
    NativeMutationSpareEntries::read(
        origin.bytes(&format!("{row}mutation_rates"))?,
        origin.bytes(&format!("{row}mutation_shapes"))?,
    )
    .map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SaveFile;
    use crate::native_binary::{NativeRecordFile, NativeValue};

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
                None,
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
            .spawn_native_script_robot(dna, species, [321.0, 654.0], None)
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
            .spawn_native_script_robot(dna.clone(), SpeciesId::default(), [321.0, 654.0], None)
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
    fn native_repopulation_copies_the_archived_species_mutation_spares() {
        let mut file = NativeRecordFile::decode(include_bytes!(
            "../../tests/fixtures/db24832-original-user.sim"
        ))
        .unwrap();
        let tail = NativeSimulationTail::decode(&file.tail).unwrap();
        let rates = tail.bytes("species[0].mutation_rates").unwrap();
        let shapes = tail.bytes("species[0].mutation_shapes").unwrap();
        assert_eq!(&rates[44..48], &11_000_f32.to_le_bytes());
        assert_eq!(&shapes[88..92], &1_f32.to_le_bytes());

        // The resident's reserved fields can differ from the species' settings.
        let resident = file
            .robots
            .iter_mut()
            .find(|robot| {
                robot.bytes("species_name").unwrap() == b"Alga_Minimalis_Chloroplastus.txt"
            })
            .unwrap();
        let mut resident_rates = resident.bytes("mutation_rates").unwrap().to_vec();
        resident_rates[44..48].copy_from_slice(&7_777_f32.to_le_bytes());
        resident
            .set_value("mutation_rates", NativeValue::Bytes(resident_rates))
            .unwrap();

        let mut world = Engine::from_native_sim(&file.encode().unwrap()).unwrap();
        world.species[0]
            .mutation_profile
            .as_mut()
            .unwrap()
            .point
            .one_in = 45_000.0;
        let mut unrelated = world.species[0].clone();
        unrelated.name = "Renamed.txt".into();
        assert!(
            script_mutation_spare_entries(Some(&tail), &unrelated)
                .unwrap()
                .is_none()
        );
        unrelated.name.clone_from(&world.species[0].name);
        unrelated.lineage_id = world.species[1].lineage_id;
        assert!(
            script_mutation_spare_entries(Some(&tail), &unrelated)
                .unwrap()
                .is_none()
        );

        let existing = world.slots.ids_in_slot_order();
        world.config.vegetation.start_chloroplasts = 0;
        world.config.vegetation.minimum_chloroplast_equivalents = 100;
        world.config.vegetation.repopulation_amount = 1;
        world.config.vegetation.repopulation_cooldown = 1;
        world.vegetation_runtime.set_native_repopulation_counter(0);
        world.vegetation_repopulation_phase().unwrap();
        let added = world
            .slots
            .ids_in_slot_order()
            .into_iter()
            .filter(|id| !existing.contains(id))
            .collect::<Vec<_>>();
        assert_eq!(added.len(), 1);
        let restored = SaveFile::decode(&SaveFile::encode(&world).unwrap()).unwrap();
        for world in [&world, &restored] {
            let saved = world.export_native_organisms(&added).unwrap();
            let saved = NativeRecordFile::decode(&saved).unwrap();
            let robot = &saved.robots[0];
            let exported = robot.bytes("mutation_rates").unwrap();
            assert_eq!(&exported[..4], &45_000_f32.to_le_bytes());
            assert_eq!(&exported[44..], &rates[44..]);
            assert_eq!(
                &robot.bytes("mutation_shapes").unwrap()[88..],
                &shapes[88..]
            );
        }
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
