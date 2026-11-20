use super::*;

/// One SNP row from the live arena, including DNA and mutation history.
#[derive(Debug, PartialEq, Serialize)]
pub(crate) struct DeathRecord {
    tick: u64,
    id: u64,
    parent_id: u64,
    founder: String,
    generation: u32,
    birth_cycle: u64,
    age: u64,
    mutations: u64,
    new_mutations: u64,
    dna_length: u32,
    offspring: u64,
    kills: u64,
    fitness: f64,
    energy: f32,
    chloroplasts: f32,
    dna: String,
    mutation_log: Vec<String>,
}

pub(super) struct DeathRecording {
    weight: u16,
    exclude_vegetables: bool,
    records: Vec<DeathRecord>,
}

impl Engine {
    pub(crate) fn tick_many_with_death_records(
        &mut self,
        count: u32,
        weight: u16,
    ) -> Result<Vec<DeathRecord>, EngineError> {
        if !self.config.recording.dead_snapshot_enabled {
            self.tick_many(count)?;
            return Ok(Vec::new());
        }
        self.with_death_recording(
            weight,
            self.config.recording.exclude_repopulating_robots,
            |engine| engine.tick_many(count),
        )
    }

    pub(crate) fn remove_many_recording_deaths(
        &mut self,
        ids: &[OrganismId],
        suppress_death_poffs: bool,
        weight: u16,
        exclude_vegetables: bool,
    ) -> Result<Vec<DeathRecord>, EngineError> {
        self.with_death_recording(weight, exclude_vegetables, |engine| {
            engine.remove_many_at_boundary(ids, suppress_death_poffs, true)?;
            engine.publish_snapshot();
            Ok(())
        })
    }

    fn with_death_recording(
        &mut self,
        weight: u16,
        exclude_vegetables: bool,
        action: impl FnOnce(&mut Self) -> Result<(), EngineError>,
    ) -> Result<Vec<DeathRecord>, EngineError> {
        self.recorded_deaths = Some(DeathRecording {
            weight,
            exclude_vegetables,
            records: Vec::new(),
        });
        let result = action(self);
        let recording = self.recorded_deaths.take().unwrap();
        result.map(|()| recording.records)
    }

    pub(crate) fn living_genome_records(
        &self,
        weight: u16,
    ) -> Result<Vec<DeathRecord>, EngineError> {
        let scores = self.native_family_scores();
        let weight = self.recording_fitness_weight(weight);
        self.slots
            .ids_in_slot_order()
            .into_iter()
            .filter(|id| self.slots.get(*id).unwrap().program.dna.blocks().len() > 1)
            .map(|id| self.genome_record(id, scores[id.slot() as usize], weight))
            .collect()
    }

    pub(super) fn capture_death_record(&mut self, id: OrganismId) -> Result<(), EngineError> {
        let Some(recording) = &self.recorded_deaths else {
            return Ok(());
        };
        let robot = self.slots.get(id)?;
        if robot.program.dna.blocks().len() == 1
            || (recording.exclude_vegetables && robot.flags.vegetable)
        {
            return Ok(());
        }
        let weight = self.recording_fitness_weight(recording.weight);
        let family = self.native_family_scores()[id.slot() as usize];
        let record = self.genome_record(id, family, weight)?;
        self.recorded_deaths.as_mut().unwrap().records.push(record);
        Ok(())
    }

    fn recording_fitness_weight(&self, fallback: u16) -> u16 {
        self.local_mode
            .evolution
            .as_ref()
            .filter(|state| {
                self.local_mode.status == LocalModeStatus::Running
                    && state.config.mode == crate::EvolutionMode::Zerobot
            })
            .map_or(fallback, |state| state.fitness_weight)
            .min(200)
    }

    fn genome_record(
        &self,
        id: OrganismId,
        family: (u64, f64, bool),
        weight: u16,
    ) -> Result<DeathRecord, EngineError> {
        let robot = self.slots.get(id)?;
        let (family_count, family_energy, _) = family;
        let population_exponent = f64::from((200 - weight).min(100)) / 100.0;
        let energy_exponent = f64::from(weight.min(100)) / 100.0;
        // Database.bas uses score's family count and energy even when the
        // Find Best action would reject that family in a Zerobot search.
        let fitness = (family_count as f64).powf(population_exponent)
            * family_energy.max(0.0).powf(energy_exponent);
        Ok(DeathRecord {
            tick: self.tick,
            id: robot.identity.absolute_id,
            parent_id: self.absolute_parent_id(robot),
            founder: native_census_filename(robot, &self.species).to_owned(),
            generation: robot.counters.generation,
            birth_cycle: robot.counters.birth_cycle,
            age: robot.counters.age,
            mutations: robot.program.mutation_metadata.total_mutations,
            new_mutations: robot.program.mutation_metadata.native_last_mutations,
            dna_length: robot.program.dna.blocks().len() as u32,
            offspring: robot.counters.offspring,
            kills: robot.counters.kills,
            fitness,
            energy: robot.materials.energy + robot.materials.body * 10.0_f32,
            chloroplasts: robot.materials.chloroplasts,
            dna: self.export_dna(id)?,
            mutation_log: robot.program.mutation_metadata.detailed_log.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world(corpses: bool) -> Engine {
        Engine::new(EngineConfig {
            metabolism_cost: 1,
            recording: RecordingSettings {
                dead_snapshot_enabled: true,
                ..Default::default()
            },
            corpse_decay: CorpseDecaySettings {
                enabled: corpses,
                packet_size: 32_000,
                period: 1,
                ..Default::default()
            },
            ..EngineConfig::testing()
        })
        .unwrap()
    }

    #[test]
    fn snp_scores_original_families_with_fractional_resources_in_deletion_order() {
        let mut world = world(false);
        let dna = LegacyDna::parse("start\nstop").unwrap();
        let parent = world.spawn_at(dna.clone(), [500.0; 2]).unwrap();
        let child = world.spawn_at(dna, [900.0; 2]).unwrap();
        let single_block = world
            .spawn_at(LegacyDna::parse("").unwrap(), [1300.0; 2])
            .unwrap();
        let root = world.slots.get_mut(parent).unwrap();
        let parent_absolute_id = root.identity.absolute_id;
        root.identity.species_file = "Original founder.txt".into();
        root.materials.energy = 100.25;
        root.materials.body = 1.25;
        root.materials.chloroplasts = 0.375;
        let descendant = world.slots.get_mut(child).unwrap();
        descendant.identity.parent_absolute_id = parent_absolute_id;
        descendant.materials.energy = 50.5;
        descendant.materials.body = 1.25;
        descendant.flags.vegetable = true;

        let living = world.living_genome_records(50).unwrap();
        assert_eq!(living.len(), 2);
        assert_eq!(living[0].founder, "Original founder.txt");
        assert_eq!(living[0].energy, 112.75);
        assert_eq!(living[0].chloroplasts, 0.375);
        assert_eq!(living[0].fitness, 2.0 * 175.75_f64.sqrt());
        assert_eq!(living[1].fitness, 63.0_f64.sqrt());
        assert_eq!(living[1].parent_id, parent_absolute_id);

        let deleted = world
            .remove_many_recording_deaths(&[child, parent, single_block], true, 50, true)
            .unwrap();
        assert_eq!(deleted.len(), 1);
        assert_eq!(deleted[0].id, parent_absolute_id);
        assert_eq!(deleted[0].fitness, 112.75_f64.sqrt());
        assert_eq!(world.population(), 0);
    }

    #[test]
    fn inbound_file_teleport_preserves_the_active_death_recording() {
        let root = tempfile::tempdir().unwrap();
        let mut source = world(false);
        let dna = LegacyDna::parse("start\nstop").unwrap();
        let transfer = source.spawn_at(dna.clone(), [1000.0; 2]).unwrap();
        std::fs::write(
            root.path().join("transfer.dbo"),
            source.export_native_organisms(&[transfer]).unwrap(),
        )
        .unwrap();

        let mut destination = world(false);
        let resident = destination.spawn_at(dna, [500.0; 2]).unwrap();
        let robot = destination.slots.get_mut(resident).unwrap();
        let resident_id = robot.identity.absolute_id;
        robot.materials.energy = 0.0;
        destination
            .add_file_teleporter(
                FileTeleporter::new(1, [1000.0; 2], 30.0)
                    .with_paths(root.path(), "")
                    .with_polling(1, 1),
            )
            .unwrap();

        let deaths = destination.tick_many_with_death_records(1, 0).unwrap();
        assert_eq!(destination.snapshot().file_teleporters[0].received, 1);
        assert_eq!(destination.population(), 1);
        assert_eq!(deaths.len(), 1);
        assert_eq!(deaths[0].id, resident_id);
    }

    #[test]
    fn no_corpse_death_is_captured_at_its_own_cycle_before_batch_publication() {
        let mut world = world(false);
        let id = world
            .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [500.0, 500.0])
            .unwrap();
        let robot = world.slots.get_mut(id).unwrap();
        let absolute_id = robot.identity.absolute_id;
        robot.identity.parent_absolute_id = 71;
        robot.program.mutation_metadata.detailed_log = vec!["Before death".into()];
        robot.materials.energy = 0.0;

        let records = world.tick_many_with_death_records(3, 0).unwrap();

        let [record] = records.as_slice() else {
            panic!("expected one death record: {records:?}");
        };
        assert_eq!(record.tick, 1);
        assert_eq!(record.id, absolute_id);
        assert_eq!(record.parent_id, 71);
        assert_eq!(record.mutation_log, ["Before death"]);
        assert_eq!(record.fitness, 1.0);
        assert!(record.dna.contains("start"));
        assert_eq!(world.snapshot().tick, 3);
        assert!(world.snapshot().organisms.is_empty());
        assert!(world.snapshot().corpses.is_empty());
        assert!(
            world
                .tick_many_with_death_records(1, 100)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn corpse_conversion_waits_for_actual_removal_to_capture_genome() {
        let mut world = world(true);
        let id = world
            .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [500.0, 500.0])
            .unwrap();
        let robot = world.slots.get_mut(id).unwrap();
        let absolute_id = robot.identity.absolute_id;
        robot.materials.energy = 0.0;
        robot.counters.age = 1;
        robot.program.mutation_metadata.detailed_log = vec!["Retained through decay".into()];

        assert!(
            world
                .tick_many_with_death_records(1, 100)
                .unwrap()
                .is_empty()
        );
        assert_eq!(world.snapshot().corpses[0].unique_id, absolute_id);
        let records = world.tick_many_with_death_records(1, 100).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, absolute_id);
        assert_eq!(records[0].founder, "Corpse");
        assert_eq!(records[0].mutation_log, ["Retained through decay"]);
        assert!(world.snapshot().corpses.is_empty());
    }

    #[test]
    fn recording_switch_and_vegetable_filter_leave_simulation_deaths_intact() {
        let mut world = world(false);
        world.config.recording.exclude_repopulating_robots = true;
        let dna = LegacyDna::parse("start\nstop").unwrap();
        let animal = world.spawn_at(dna.clone(), [500.0, 500.0]).unwrap();
        let vegetable = world.spawn_at(dna.clone(), [900.0, 900.0]).unwrap();
        let animal_id = world.slots.get(animal).unwrap().identity.absolute_id;
        world.slots.get_mut(vegetable).unwrap().flags.vegetable = true;
        for id in [animal, vegetable] {
            world.slots.get_mut(id).unwrap().flags.dead = true;
        }

        let records = world.tick_many_with_death_records(1, 100).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, animal_id);
        assert!(world.snapshot().organisms.is_empty());

        world.config.recording.dead_snapshot_enabled = false;
        let animal = world.spawn_at(dna, [500.0, 500.0]).unwrap();
        world.slots.get_mut(animal).unwrap().flags.dead = true;
        assert!(
            world
                .tick_many_with_death_records(1, 100)
                .unwrap()
                .is_empty()
        );
        assert!(world.snapshot().organisms.is_empty());
    }
}
