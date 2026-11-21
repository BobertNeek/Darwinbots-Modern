use super::*;

/// Data captured before a committed KillRobot-equivalent deletion clears it.
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
    energy: i32,
    chloroplasts: i32,
    dna: String,
    mutation_log: Vec<String>,
}

impl Engine {
    pub(crate) fn tick_many_with_death_records(
        &mut self,
        count: u32,
    ) -> Result<Vec<DeathRecord>, EngineError> {
        self.tick_many_recording_deaths_with_redraw(count, NativeRedrawCadence::Off)
    }

    pub(crate) fn tick_many_recording_deaths_with_redraw(
        &mut self,
        count: u32,
        native_redraw: NativeRedrawCadence,
    ) -> Result<Vec<DeathRecord>, EngineError> {
        if !self.config.recording.dead_snapshot_enabled {
            self.tick_many_with_native_redraw(count, native_redraw)?;
            return Ok(Vec::new());
        }
        self.recorded_deaths = Some(Vec::new());
        let result = self.tick_many_with_native_redraw(count, native_redraw);
        let records = self.recorded_deaths.take().unwrap_or_default();
        result.map(|()| records)
    }

    pub(super) fn capture_death_record(&mut self, id: OrganismId) -> Result<(), EngineError> {
        if self.recorded_deaths.is_none() {
            return Ok(());
        }
        let robot = self.slots.get(id)?;
        let dna_length = robot.program.dna.blocks().len() as u32;
        if dna_length == 1
            || (self.config.recording.exclude_repopulating_robots && robot.flags.vegetable)
        {
            return Ok(());
        }
        let parent_id = if robot.identity.parent_absolute_id != 0 {
            robot.identity.parent_absolute_id
        } else {
            robot.identity.parents[0]
                .and_then(|parent| self.slots.get(parent).ok())
                .map_or(0, |state| state.identity.absolute_id)
        };
        let record = DeathRecord {
            tick: self.tick,
            id: robot.identity.absolute_id,
            parent_id,
            founder: native_census_filename(robot, &self.species).to_owned(),
            generation: robot.counters.generation,
            birth_cycle: robot.counters.birth_cycle,
            age: robot.counters.age,
            mutations: robot.program.mutation_metadata.total_mutations,
            new_mutations: robot.program.mutation_metadata.native_last_mutations,
            dna_length,
            offspring: robot.counters.offspring,
            kills: robot.counters.kills,
            energy: resource_readback(robot.materials.energy + robot.materials.body * 10.0),
            chloroplasts: resource_readback(robot.materials.chloroplasts),
            dna: self.export_dna(id)?,
            mutation_log: robot.program.mutation_metadata.detailed_log.clone(),
        };
        self.recorded_deaths.as_mut().unwrap().push(record);
        Ok(())
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

        let records = world.tick_many_with_death_records(3).unwrap();

        let [record] = records.as_slice() else {
            panic!("expected one death record: {records:?}");
        };
        assert_eq!(record.tick, 1);
        assert_eq!(record.id, absolute_id);
        assert_eq!(record.parent_id, 71);
        assert_eq!(record.mutation_log, ["Before death"]);
        assert!(record.dna.contains("start"));
        assert_eq!(world.snapshot().tick, 3);
        assert!(world.snapshot().organisms.is_empty());
        assert!(world.snapshot().corpses.is_empty());
        assert!(world.tick_many_with_death_records(1).unwrap().is_empty());
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

        assert!(world.tick_many_with_death_records(1).unwrap().is_empty());
        assert_eq!(world.snapshot().corpses[0].unique_id, absolute_id);
        let records = world.tick_many_with_death_records(1).unwrap();
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

        let records = world.tick_many_with_death_records(1).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, animal_id);
        assert!(world.snapshot().organisms.is_empty());

        world.config.recording.dead_snapshot_enabled = false;
        let animal = world.spawn_at(dna, [500.0, 500.0]).unwrap();
        world.slots.get_mut(animal).unwrap().flags.dead = true;
        assert!(world.tick_many_with_death_records(1).unwrap().is_empty());
        assert!(world.snapshot().organisms.is_empty());
    }
}
