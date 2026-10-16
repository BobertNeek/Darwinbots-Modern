use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NativeStartBoundary {
    ConfiguredStart,
    SameProcessRound,
    ExternalProcess,
}

impl Engine {
    /// Begin an original 2.48.32 new simulation, before placing any robots.
    /// `new` remains the bare seeded constructor used by legacy APIs.
    pub fn new_native(config: EngineConfig) -> Result<Self, EngineError> {
        let mut engine = Self::new(config)?;
        engine.native_cycle = Some(-1);
        engine.snapshot.native_cycle = Some(-1);
        engine.vegetation_runtime = VegetationRuntime::fresh_native_process();
        engine.initialize_native_start();
        Ok(engine)
    }

    pub(crate) fn initialize_native_load(&mut self) {
        let random = self.native_random.get_or_insert_with(Default::default);
        random.reseed_native_start(self.config.seed);
        // main.frm startloaded draws all three chart colors, even UseOldColor.
        for _ in 0..3 {
            random.unit();
        }
    }

    fn initialize_native_start(&mut self) {
        self.initialize_native_load();
        let random = self.native_random.as_mut().unwrap();
        self.vegetation_runtime.initialize_native_start(
            self.config.vegetation.sun_on_random || self.config.vegetation.weather_enabled,
            random,
        );
        // SimGUID is assigned after optional random sunlight, before placement.
        self.native_sim_guid = Some(random.unit().round_ties_even() as i32);
    }

    pub(crate) fn inherit_native_gaussian_spare(&mut self, previous: &Self) {
        if let (Some(random), Some(previous)) =
            (self.native_random.as_mut(), previous.native_random.as_ref())
        {
            random.inherit_gaussian_spare(previous);
        }
    }

    /// Inspect the native process cache without consuming a random draw.
    pub fn read_native_gaussian_spare(&self) -> Option<f32> {
        self.native_random
            .as_ref()
            .and_then(crate::native_random::NativeRandom::gaussian_spare)
    }

    /// Transfer only the native process cache; uniform RNG and world state stay put.
    pub fn set_native_gaussian_spare(&mut self, spare: Option<f32>) -> Result<(), EngineError> {
        // Reject invalid input before considering either stream kind or mutation.
        if spare.is_some_and(|value| !value.is_finite()) {
            return Err(EngineError::Invariant(
                "native Gaussian spare must be finite".into(),
            ));
        }
        match self.native_random.as_mut() {
            Some(random) => random
                .set_gaussian_spare(spare)
                .map_err(|error| EngineError::Invariant(error.into())),
            None if spare.is_none() => Ok(()),
            None => Err(EngineError::Invariant(
                "a legacy random stream has no native Gaussian cache".into(),
            )),
        }
    }

    /// Inspect VB6's process-global SunChange byte without drawing randomness.
    pub fn read_native_sun_change(&self) -> u8 {
        self.vegetation_runtime.sun_change()
    }

    /// Transfer only SunChange; native save loads retain their own weather state.
    pub fn set_native_sun_change(&mut self, sun_change: u8) {
        self.vegetation_runtime.set_sun_change(sun_change);
    }

    /// Record a host clock input without consuming the simulation stream.
    pub fn set_native_restart_seed(&mut self, seed: u64) {
        self.config.native_restart_seed = Some(seed);
    }

    pub(super) fn prepare_native_restart(&mut self, boundary: NativeStartBoundary) {
        if self.native_cycle.is_some()
            && self
                .local_mode
                .config
                .as_ref()
                .is_some_and(|config| config.workflow == crate::LeagueWorkflow::PopulationSeeding)
            && self.local_mode.series.is_population_census()
        {
            if boundary == NativeStartBoundary::ExternalProcess
                || self.native_population_census.is_none()
            {
                self.native_population_census = Some([0, 0]);
            }
        } else {
            self.native_population_census = None;
        }
        if self.native_cycle.is_some()
            && matches!(
                boundary,
                NativeStartBoundary::ConfiguredStart | NativeStartBoundary::ExternalProcess
            )
        {
            // StartNew_Click initializes -1. In-process continuations retain
            // the caller's counter; only Countpop's surviving-winner path sets0.
            self.native_cycle = Some(-1);
        }
        let Some(random) = self.native_random.as_mut() else {
            return;
        };
        match boundary {
            // OptionsForm.StartNew_Click enters StartSimul with the selected seed.
            // Common.gasdev and other process-static state survive this UI action.
            NativeStartBoundary::ConfiguredStart => {}
            NativeStartBoundary::SameProcessRound => {
                // Only the next F1/autorestart round chooses another Long seed.
                // VB6 promotes Single * Long to Double before Long assignment.
                self.config.seed =
                    (f64::from(random.unit()) * f64::from(i32::MAX)).round_ties_even() as u64;
            }
            NativeStartBoundary::ExternalProcess => {
                self.config.seed = self.config.native_restart_seed.unwrap_or(self.config.seed);
                // Restarter.exe creates a new process with an empty gasdev cache.
                *random = Default::default();
                self.vegetation_runtime = VegetationRuntime::fresh_native_process();
            }
        }
        self.initialize_native_start();
    }

    pub(crate) fn has_native_random(&self) -> bool {
        self.native_random.is_some()
    }

    pub(super) fn next_native_cycle(&self) -> Result<Option<i32>, EngineError> {
        self.native_cycle
            .map(|cycle| {
                cycle.checked_add(1).ok_or_else(|| {
                    EngineError::Invariant("native cycle exceeds signed Long range".into())
                })
            })
            .transpose()
    }

    /// Source cycle for phases which run after Master.UpdateSim increments it.
    /// Actual native cycles are nonnegative; -1 only exists before a first cycle.
    pub(super) fn native_cycle_or_tick(&self) -> u64 {
        self.native_cycle
            .map_or(self.tick, |cycle| cycle.max(0) as u64)
    }

    /// Cost/oscillation preparation runs before the actual cycle increment.
    pub(super) fn next_native_cycle_or_tick(&self) -> u64 {
        self.native_cycle.map_or_else(
            || self.tick.saturating_add(1),
            |cycle| (i64::from(cycle) + 1).max(0) as u64,
        )
    }

    pub(super) fn spawn_native_start_robot(
        &mut self,
        dna: LegacyDna,
        species: SpeciesId,
        legacy_seed: u64,
        position_override: Option<[f32; 2]>,
    ) -> Result<OrganismId, EngineError> {
        if self.native_random.is_none() {
            let position = position_override
                .unwrap_or_else(|| self.native_placement_position(species, legacy_seed));
            let id = self.spawn_species_at_unpublished(dna, species, position)?;
            self.native_placement_heading(id)?;
            return Ok(id);
        }
        if self.available_robot_capacity() == 0 {
            return Err(EngineError::CapacityReached);
        }
        let bounds = self
            .species
            .get(species.0 as usize)
            .ok_or_else(|| EngineError::Invariant("species does not exist".into()))?
            .placement_bounds;
        let bounds = if self.classic_native_species_color(species).is_some() {
            [0.0, 1.0, 0.0, 1.0]
        } else {
            bounds
        };
        let x_extent = self.config.world_width - 60.0;
        let y_extent = self.config.world_height - 60.0;
        let random = self.native_random.as_mut().unwrap();
        // Module1.preparerob: two discarded coordinates, discrete heading,
        // and three discarded colors. loadrobs later overwrites pos/color.
        random.unit();
        random.unit();
        // Random(0, 628) multiplies an Integer span by a Single. Division
        // of its Long result by Integer 100 is Double, stored as Single aim.
        let heading_word = (random.unit() * 629.0).floor() as i32;
        let heading = (f64::from(heading_word) / 100.0) as f32;
        for _ in 0..3 {
            random.unit();
        }
        // main.frm loadrobs passes Variant Singles to Random, so every
        // operation below remains Single and Int floors the final sum.
        let coordinate = |low: f32, high: f32, draw: f32| {
            ((high - low + 1.0) * draw + low).floor() as i32 as f32
        };
        let position = [
            coordinate(bounds[0] * x_extent, bounds[1] * x_extent, random.unit()),
            coordinate(bounds[2] * y_extent, bounds[3] * y_extent, random.unit()),
        ];
        // -32000..32000 overflows an Integer span into Long: the Long *
        // Single exception makes this ninth draw Double arithmetic.
        let timer = random.integer(-32000, 32000);
        let state = u64::from(random.state());
        let id =
            self.spawn_species_at_unpublished(dna, species, position_override.unwrap_or(position))?;
        let robot = self.slots.get_mut(id)?;
        robot.set_heading(heading);
        // VB6 Cos/Sin return Double before VectorSet narrows to Single.
        robot.kinematics.aim_vector = [
            f64::from(heading).cos() as f32,
            f64::from(heading).sin() as f32,
        ];
        robot.program.memory.write(19, robot.materials.aim);
        robot.program.memory.write(12, timer);
        robot.program.rng_state = state;
        Ok(id)
    }

    pub(super) fn native_placement_position(
        &mut self,
        species: SpeciesId,
        legacy_seed: u64,
    ) -> [f32; 2] {
        if let Some(random) = self.native_random.as_mut() {
            let bounds = self.species[species.0 as usize].placement_bounds;
            // aggiungirob passes Long field extents times Single bounds to
            // ByVal Long fRnd: promote the multiplication before CLng.
            let x_extent = f64::from(self.config.world_width) - 60.0;
            let y_extent = f64::from(self.config.world_height) - 60.0;
            return [
                random.rounded_integer(
                    (f64::from(bounds[0]) * x_extent).round_ties_even() as i32,
                    (f64::from(bounds[1]) * x_extent).round_ties_even() as i32,
                ) as f32,
                random.rounded_integer(
                    (f64::from(bounds[2]) * y_extent).round_ties_even() as i32,
                    (f64::from(bounds[3]) * y_extent).round_ties_even() as i32,
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

    fn exported_cycle(world: &Engine) -> i32 {
        let file =
            crate::native_binary::NativeRecordFile::decode(&world.export_native_sim().unwrap())
                .unwrap();
        crate::native_binary::NativeSimulationTail::decode(&file.tail)
            .unwrap()
            .integer("total_cycles")
            .unwrap()
    }

    #[test]
    fn native_cycle_fresh_zero_and_legacy_tick_are_distinct_without_shifting_dna_age() {
        for native in [false, true] {
            let mut world = if native {
                Engine::new_native(EngineConfig::testing()).unwrap()
            } else {
                Engine::new(EngineConfig::testing()).unwrap()
            };
            let id = world
                .spawn_at(
                    LegacyDna::parse("start *.robage 900 store stop").unwrap(),
                    [500.0; 2],
                )
                .unwrap();
            world.update_mutation_settings(false, 1.0, &[]).unwrap();
            assert_eq!(world.snapshot.tick, 0);
            assert_eq!(world.snapshot.native_cycle, native.then_some(-1));
            assert_eq!(exported_cycle(&world), if native { -1 } else { 0 });
            world.execute_robots().unwrap();
            assert_eq!(world.snapshot.tick, 0);
            assert_eq!(world.snapshot.native_cycle, native.then_some(-1));
            world.tick().unwrap();
            assert_eq!(world.snapshot.tick, 1);
            assert_eq!(world.snapshot.native_cycle, native.then_some(0));
            assert_eq!(world.memory_at(id, 900).unwrap(), 0);
            assert_eq!(world.slots.get(id).unwrap().counters.age, 1);
            assert_eq!(exported_cycle(&world), if native { 0 } else { 1 });
            world.tick_many(9).unwrap();
            assert_eq!(world.snapshot.tick, 10);
            assert_eq!(world.snapshot.native_cycle, native.then_some(9));
            assert_eq!(exported_cycle(&world), if native { 9 } else { 10 });
            world.validate_invariants().unwrap();
        }
    }

    #[test]
    fn native_cycle_import_retains_signed_pre_cycle_and_positive_loaded_cursors() {
        use crate::native_binary::{NativeRecordFile, NativeSimulationTail, NativeValue};
        let seed = Engine::new(EngineConfig::testing())
            .unwrap()
            .export_native_sim()
            .unwrap();
        for cycle in [-2_i32, -1, 0, 281] {
            let mut file = NativeRecordFile::decode(&seed).unwrap();
            let mut tail = NativeSimulationTail::decode(&file.tail).unwrap();
            tail.set_value("total_cycles", NativeValue::Long(cycle))
                .unwrap();
            file.tail = tail.encode().unwrap();
            let bytes = file.encode().unwrap();
            if cycle < -1 {
                assert!(Engine::from_native_sim(&bytes).is_err());
                continue;
            }
            let mut world = Engine::from_native_sim(&bytes).unwrap();
            assert_eq!(world.snapshot.native_cycle, Some(cycle));
            assert_eq!(world.snapshot.tick, cycle.max(0) as u64);
            assert_eq!(exported_cycle(&world), cycle);
            world.tick().unwrap();
            assert_eq!(world.snapshot.native_cycle, Some(cycle + 1));
            assert_eq!(world.snapshot.tick, cycle.max(0) as u64 + 1);
            assert_eq!(exported_cycle(&world), cycle + 1);
        }
    }

    #[test]
    fn native_cycle_db3s_keeps_independent_clocks_and_absent_legacy_field() {
        for cycle in [-1, 0, 79] {
            let mut world = Engine::new_native(EngineConfig::testing()).unwrap();
            world.tick = 53;
            world.native_cycle = Some(cycle);
            world.publish_snapshot();
            let bytes = crate::SaveFile::encode(&world).unwrap();
            let mut resumed = crate::SaveFile::decode(&bytes).unwrap();
            assert_eq!(crate::SaveFile::encode(&resumed).unwrap(), bytes);
            assert_eq!(resumed.snapshot.tick, 53);
            assert_eq!(resumed.snapshot.native_cycle, Some(cycle));
            resumed.tick().unwrap();
            assert_eq!(resumed.snapshot.tick, 54);
            assert_eq!(resumed.snapshot.native_cycle, Some(cycle + 1));

            let mut payload: serde_json::Value = serde_json::from_slice(&bytes[10..]).unwrap();
            payload.as_object_mut().unwrap().remove("native_cycle");
            let payload = serde_json::to_vec(&payload).unwrap();
            let mut old = bytes[..6].to_vec();
            old.extend_from_slice(&(payload.len() as u32).to_le_bytes());
            old.extend_from_slice(&payload);
            let mut legacy = crate::SaveFile::decode(&old).unwrap();
            assert_eq!(legacy.snapshot.native_cycle, None);
            assert!(
                serde_json::to_value(&legacy.snapshot)
                    .unwrap()
                    .get("native_cycle")
                    .is_none()
            );
            legacy.prepare_native_restart(NativeStartBoundary::ExternalProcess);
            legacy.publish_snapshot();
            assert_eq!(legacy.snapshot.native_cycle, None);
            legacy.tick().unwrap();
            assert_eq!(legacy.snapshot.tick, 54);
            assert_eq!(exported_cycle(&legacy), 54);
        }
    }

    #[test]
    fn native_cycle_invalid_db3s_and_overflow_fail_before_cost_or_rng_changes() {
        let mut world = Engine::new_native(EngineConfig::testing()).unwrap();
        let mut invalid = world.db3s_v3_payload().unwrap();
        invalid["native_cycle"] = serde_json::json!(-2);
        let invalid = Engine::from_db3s_v3_payload(invalid).unwrap();
        assert!(Engine::restore(invalid).is_err());
        let mut invalid = world.db3s_v3_payload().unwrap();
        invalid["native_population_census"] = serde_json::json!([MAX_ROBOTS + 1, 0]);
        let invalid = Engine::from_db3s_v3_payload(invalid).unwrap();
        assert!(Engine::restore(invalid).is_err());
        world.native_cycle = Some(i32::MAX);
        world.publish_snapshot();
        let before = crate::SaveFile::encode(&world).unwrap();
        let publications = world.snapshot_publication_count();
        assert!(
            world
                .tick()
                .unwrap_err()
                .to_string()
                .contains("signed Long")
        );
        assert_eq!(crate::SaveFile::encode(&world).unwrap(), before);
        assert_eq!(world.snapshot_publication_count(), publications);
        assert_eq!(world.snapshot.tick, 0);
        assert_eq!(world.snapshot.native_cycle, Some(i32::MAX));
    }

    #[test]
    fn native_cycle_checkpoint_restores_native_and_monotonic_clocks_together() {
        let mut world = Engine::new_native(EngineConfig::testing()).unwrap();
        world.tick_many(3).unwrap();
        let before = crate::SaveFile::encode(&world).unwrap();
        let snapshot = world.snapshot.clone();
        let checkpoint = world.cycle_checkpoint();
        world.tick_many(2).unwrap();
        world.prepare_native_restart(NativeStartBoundary::ExternalProcess);
        world.restore_cycle_checkpoint(checkpoint);
        assert_eq!(world.snapshot, snapshot);
        assert_eq!(crate::SaveFile::encode(&world).unwrap(), before);
        world.tick().unwrap();
        assert_eq!(world.snapshot.tick, 4);
        assert_eq!(world.snapshot.native_cycle, Some(3));
    }

    #[test]
    fn native_cycle_start_boundaries_preserve_only_generic_same_process_counter() {
        let mut world = Engine::new_native(EngineConfig::testing()).unwrap();
        world.native_cycle = Some(231);
        world.prepare_native_restart(NativeStartBoundary::SameProcessRound);
        assert_eq!(world.native_cycle, Some(231));
        for boundary in [
            NativeStartBoundary::ConfiguredStart,
            NativeStartBoundary::ExternalProcess,
        ] {
            world.native_cycle = Some(231);
            world.prepare_native_restart(boundary);
            assert_eq!(world.native_cycle, Some(-1));
        }
        world.native_cycle = None;
        world.prepare_native_restart(NativeStartBoundary::ConfiguredStart);
        assert_eq!(world.native_cycle, None);
    }

    fn remove_other_contest_species(world: &mut Engine, winner: SpeciesId) {
        // Keep startup definitions registered when the live losing species dies.
        for species in &mut world.species {
            species.reseed = true;
        }
        let ids: Vec<_> = world
            .slots
            .ids_in_slot_order()
            .into_iter()
            .filter(|id| {
                let robot = world.slots.get(*id).unwrap();
                !robot.flags.vegetable && robot.identity.species != winner
            })
            .collect();
        world.remove_many(&ids).unwrap();
    }

    #[test]
    fn native_cycle_first_contest_sample_is_ninth_displayed_cycle_and_tenth_call() {
        let (mut world, winner) =
            native_contest_world(42, LocalModeKind::Contest, crate::LeagueWorkflow::Direct);
        world.local_mode.config.as_mut().unwrap().maximum_rounds = 1;
        world.start_local_mode().unwrap();
        world.tick_many(9).unwrap();
        assert!(world.local_mode.results.is_empty());
        assert_eq!(world.native_cycle, Some(8));
        remove_other_contest_species(&mut world, winner);
        world.tick().unwrap();
        assert_eq!(world.local_mode.status, LocalModeStatus::Completed);
        assert_eq!(world.local_mode.results[0].winner, Some(winner));
        assert_eq!(world.snapshot.tick, 10);
        assert_eq!(world.snapshot.native_cycle, Some(9));
        assert_eq!(exported_cycle(&world), 9);
    }

    #[test]
    fn native_cycle_nonterminal_f1_winner_resets_zero_but_draw_retains_counter() {
        let (mut world, winner) =
            native_contest_world(42, LocalModeKind::Contest, crate::LeagueWorkflow::Direct);
        world.start_local_mode().unwrap();
        world.tick_many(9).unwrap();
        remove_other_contest_species(&mut world, winner);
        world.tick().unwrap();
        assert_eq!(world.snapshot.tick, 10);
        assert_eq!(world.snapshot.native_cycle, Some(0));
        assert_eq!(world.local_mode.current_round, 2);
        assert_eq!(world.local_mode.cycle_in_round, 0);
        assert_eq!(world.population(), 10);
        world.tick().unwrap();
        assert_eq!(world.snapshot.tick, 11);
        assert_eq!(world.snapshot.native_cycle, Some(1));
        world
            .adjudicate_local_round(None, "both dead source branch".into())
            .unwrap();
        assert_eq!(world.native_cycle, Some(1));
        world.restart_local_mode().unwrap();
        assert_eq!(world.native_cycle, Some(-1));
        assert_eq!(world.tick, 11);
    }

    #[test]
    fn native_cycle_contest_limit_uses_source_counter_without_shifting_ten_call_samples() {
        let (mut world, _) =
            native_contest_world(42, LocalModeKind::Contest, crate::LeagueWorkflow::Direct);
        world.local_mode.config.as_mut().unwrap().maximum_cycles = 2009;
        world.local_mode.config.as_mut().unwrap().maximum_rounds = 1;
        world.start_local_mode().unwrap();
        let last = *world.slots.ids_in_slot_order().last().unwrap();
        world.remove(last).unwrap(); // Two surviving species, populations5 and4.
        world.local_mode.cycle_in_round = 2009;
        world.native_cycle = Some(2009);
        world.advance_local_mode_after_cycle().unwrap();
        assert_eq!(world.local_mode.cycle_in_round, 2010);
        assert!(world.local_mode.results.is_empty());
        world.local_mode.cycle_in_round = 2019;
        world.native_cycle = Some(2019);
        world.advance_local_mode_after_cycle().unwrap();
        assert_eq!(world.local_mode.results.len(), 1);
        assert_eq!(world.local_mode.results[0].reason, "cycle limit");
    }

    #[test]
    fn native_cycle_automatic_both_dead_retries_same_round_without_awarding_a_result() {
        let (mut world, winner) =
            native_contest_world(42, LocalModeKind::Contest, crate::LeagueWorkflow::Direct);
        world
            .local_mode
            .config
            .as_mut()
            .unwrap()
            .restart_on_extinction = false;
        world.config.vegetation.minimum_chloroplast_equivalents = 0;
        for species in &mut world.species {
            species.reseed = true;
        }
        world.set_native_gaussian_spare(Some(1.25)).unwrap();
        world.start_local_mode().unwrap();
        world.tick_many(9).unwrap();
        let ids = world.slots.ids_in_slot_order();
        world.remove_many(&ids).unwrap();
        let previous_population = world.local_mode.contest_runtime.previous_population.clone();
        world.tick().unwrap();
        assert_eq!(world.local_mode.status, LocalModeStatus::Running);
        assert_eq!(world.local_mode.current_round, 1);
        assert_eq!(world.local_mode.restart_count, 0);
        assert!(world.local_mode.results.is_empty());
        assert_eq!(world.local_mode.adjudication, None);
        assert_eq!(world.local_mode.cycle_in_round, 0);
        assert_eq!(
            world.local_mode.contest_runtime.previous_population,
            previous_population
        );
        assert_eq!(world.snapshot.tick, 10);
        assert_eq!(world.snapshot.native_cycle, Some(9));
        assert_eq!(world.population(), 10);
        // No fixture DNA/weather/brownian/repopulation draws between headers.
        assert_eq!(world.config.seed, 883_216_256);
        assert_eq!(world.native_random.as_ref().unwrap().state(), 0x00e6b60c);
        assert_eq!(world.read_native_gaussian_spare(), Some(1.25));
        world.tick().unwrap();
        assert_eq!(world.snapshot.native_cycle, Some(10));
        assert_eq!(world.local_mode.cycle_in_round, 1);
        world
            .adjudicate_local_round(Some(winner), "previous round retained through retry".into())
            .unwrap();
        let results = world.local_mode.results.clone();
        let adjudication = world.local_mode.adjudication.clone();
        let ids = world.slots.ids_in_slot_order();
        world.remove_many(&ids).unwrap();
        world.tick_many(10).unwrap();
        assert_eq!(world.local_mode.current_round, 2);
        assert_eq!(world.local_mode.results, results);
        assert_eq!(world.local_mode.adjudication, adjudication);
        assert_eq!(world.local_mode.restart_count, 0);
        assert_eq!(world.native_cycle, Some(10));
        assert_eq!(world.read_native_gaussian_spare(), Some(1.25));
    }

    #[test]
    fn native_cycle_both_dead_retry_keeps_legacy_and_manual_adjudication_choices() {
        for native in [false, true] {
            let (mut world, _) =
                native_contest_world(42, LocalModeKind::Contest, crate::LeagueWorkflow::Direct);
            if !native {
                world.native_cycle = None;
            }
            for species in &mut world.species {
                species.reseed = true;
            }
            world.start_local_mode().unwrap();
            if native {
                world
                    .adjudicate_local_round(None, "explicit operator draw".into())
                    .unwrap();
                assert_eq!(world.local_mode.results.len(), 1);
                assert_eq!(world.local_mode.current_round, 2);
                assert!(!world.local_mode.results[0].automatic);
            } else {
                let ids = world.slots.ids_in_slot_order();
                world.remove_many(&ids).unwrap();
                world.tick_many(10).unwrap();
                assert_eq!(world.local_mode.results.len(), 1);
                assert_eq!(world.local_mode.current_round, 2);
                assert!(world.local_mode.results[0].automatic);
                assert_eq!(world.native_cycle, None);
            }
        }
    }

    #[test]
    fn native_cycle_controls_first_cost_oscillation_and_tide_phases() {
        let mut world = Engine::new_native(EngineConfig::testing()).unwrap();
        world
            .spawn_at(LegacyDna::parse("start stop").unwrap(), [500.0; 2])
            .unwrap();
        world.config.advanced_mutation.oscillation = crate::MutationOscillationSettings {
            mode: crate::MutationOscillationMode::Sawtooth,
            high_cycles: 1,
            low_cycles: 1,
            ..Default::default()
        };
        world.config.vegetation.tides = 4;
        world.update_native_mutation_oscillation();
        world.update_dynamic_cost_state();
        assert_eq!(world.global_mutation_multiplier, 16.0);
        assert_eq!(world.dynamic_cost_state.population_history[0], 1);
        world.tick().unwrap();
        assert_eq!(world.native_cycle, Some(0));
        assert_eq!(world.tide_scaling().to_bits(), 0x3f3504f3);
        world.tick().unwrap();
        assert_eq!(world.native_cycle, Some(1));
        assert_eq!(world.global_mutation_multiplier, 0.0625);
        assert_eq!(world.tide_scaling(), 1.0);
    }

    #[test]
    fn native_cycle_births_record_source_cycle_instead_of_lifetime_tick() {
        let mut world = Engine::new_native(EngineConfig::testing()).unwrap();
        world.update_mutation_settings(false, 1.0, &[]).unwrap();
        let parent = world
            .spawn_at(
                LegacyDna::parse("cond *.robage 0 = start 50 .repro store stop").unwrap(),
                [500.0; 2],
            )
            .unwrap();
        world.tick().unwrap();
        assert_eq!(world.stats.births, 1);
        let child = world
            .slots
            .ids_in_slot_order()
            .into_iter()
            .find(|id| *id != parent)
            .unwrap();
        assert_eq!(world.slots.get(child).unwrap().counters.birth_cycle, 0);
        world.tick = 500;
        world.native_cycle = Some(7);
        let clone = world.clone_organism(parent, [600.0; 2]).unwrap();
        let manual = world.manual_reproduce(parent, None, [700.0; 2]).unwrap();
        for id in [clone, manual] {
            assert_eq!(world.slots.get(id).unwrap().counters.birth_cycle, 7);
        }
        let file =
            crate::native_binary::NativeRecordFile::decode(&world.export_native_sim().unwrap())
                .unwrap();
        assert_eq!(file.robots[1].long("birth_cycle").unwrap(), 0);
        assert_eq!(file.robots[2].long("birth_cycle").unwrap(), 7);
        assert_eq!(file.robots[3].long("birth_cycle").unwrap(), 7);
    }

    #[test]
    fn native_population_seeding_records_every_candidate_at_exact_two_thousand_without_rounds() {
        let (mut world, first) = native_contest_world(
            42,
            LocalModeKind::League,
            crate::LeagueWorkflow::PopulationSeeding,
        );
        world.local_mode.config.as_mut().unwrap().seeding_opponent = Some(SpeciesId(u32::MAX)); // ignored by this workflow
        world.config.vegetation.minimum_chloroplast_equivalents = 0;
        world.start_local_mode().unwrap();
        assert_eq!(world.population(), 5);
        assert!(
            world
                .snapshot
                .organisms
                .iter()
                .all(|robot| robot.species == first)
        );
        assert_eq!(world.local_mode.series.active_seed, Some(first));
        assert_eq!(world.local_mode.series.active_match, None);
        assert!(!world.native_f1_enabled());
        world.native_cycle = Some(1998);
        world.tick().unwrap();
        assert_eq!(world.native_cycle, Some(1999));
        assert!(world.local_mode.series.population_seeds.is_empty());
        world.set_native_gaussian_spare(Some(2.5)).unwrap();
        world.set_native_restart_seed(77);
        world.tick().unwrap();
        assert_eq!(
            world.local_mode.series.population_seeds,
            [crate::PopulationSeed {
                species: first,
                population: 5,
                native_cycle: 2000,
                file_name: "5.txt".into(),
            }]
        );
        let second = world.local_mode.series.active_seed.unwrap();
        assert_ne!(second, first);
        assert!(
            world
                .snapshot
                .organisms
                .iter()
                .all(|robot| robot.species == second)
        );
        assert_eq!(world.native_cycle, Some(-1));
        assert_eq!(world.config.seed, 77);
        assert_eq!(world.read_native_gaussian_spare(), None);
        assert_eq!(world.native_population_census, Some([0, 0]));
        assert_eq!(world.local_mode.results.len(), 0);
        assert_eq!(world.local_mode.adjudication, None);
        assert_eq!(world.local_mode.current_round, 2);
        let startup = world.export_species_template_dna(second).unwrap();
        let id = world.slots.ids_in_slot_order()[0];
        world
            .replace_dna(id, LegacyDna::parse("start 999 900 store stop").unwrap())
            .unwrap();
        let ids = world.slots.ids_in_slot_order();
        world.remove_many(&ids).unwrap();
        world.native_cycle = Some(1999);
        world.tick().unwrap();
        assert_eq!(world.local_mode.status, LocalModeStatus::Running);
        assert_eq!(world.native_cycle, Some(-1));
        assert_eq!(world.local_mode.series.active_seed, None);
        assert_eq!(
            world.local_mode.series.population_seeding_phase,
            Some(crate::PopulationSeedingPhase::Tournament)
        );
        assert!(world.native_f1_enabled());
        assert_eq!(
            world.local_mode.series.population_seeds[1],
            crate::PopulationSeed {
                species: second,
                population: 0,
                native_cycle: 2000,
                file_name: "0.txt".into(),
            }
        );
        assert_eq!(world.export_species_template_dna(second).unwrap(), startup);
        let restored = crate::SaveFile::decode(&crate::SaveFile::encode(&world).unwrap()).unwrap();
        assert_eq!(
            restored.export_species_template_dna(second).unwrap(),
            startup
        );
        assert_eq!(
            restored.local_mode.series.population_seeds,
            world.local_mode.series.population_seeds
        );
        assert!(
            restored
                .export_species_template_dna(SpeciesId(u32::MAX))
                .is_err()
        );
        assert!(
            restored
                .export_species_template_dna(SpeciesId::default())
                .is_err()
        );
    }

    fn reproducing_population_seed() -> Engine {
        let (mut world, first) = native_contest_world(
            42,
            LocalModeKind::League,
            crate::LeagueWorkflow::PopulationSeeding,
        );
        world.local_mode.config.as_mut().unwrap().entrant_order = vec![first];
        world.species_templates[first.0 as usize]
            .as_mut()
            .unwrap()
            .dna = LegacyDna::parse("cond *.robage 1 = start 50 .repro store stop").unwrap();
        world.config.vegetation.minimum_chloroplast_equivalents = 0;
        world.start_local_mode().unwrap();
        world.native_cycle = Some(1997);
        world.publish_snapshot();
        world
    }

    #[test]
    fn native_population_seed_uses_previous_mid_cycle_census_with_unpublished_births_and_resume() {
        let mut batched = reproducing_population_seed();
        batched.tick_many(3).unwrap();
        assert_eq!(batched.native_cycle, Some(2000));
        assert_eq!(batched.local_mode.series.population_seeds[0].population, 5);
        assert_eq!(batched.native_population_census, Some([10, 5]));
        assert_eq!(batched.population(), 10);
        assert_eq!(batched.stats.births, 5);

        let mut world = reproducing_population_seed();
        world.tick_many(2).unwrap();
        assert_eq!(world.native_cycle, Some(1999));
        assert_eq!(world.native_population_census, Some([5, 5]));
        assert_eq!(world.population(), 10);
        assert_eq!(world.stats.births, 5);
        let bytes = crate::SaveFile::encode(&world).unwrap();
        let mut resumed = crate::SaveFile::decode(&bytes).unwrap();
        let checkpoint = world.cycle_checkpoint();
        world.tick().unwrap();
        world.restore_cycle_checkpoint(checkpoint);
        assert_eq!(crate::SaveFile::encode(&world).unwrap(), bytes);
        for engine in [&mut world, &mut resumed] {
            engine.tick().unwrap();
            assert_eq!(
                engine.local_mode.series.population_seeds,
                batched.local_mode.series.population_seeds
            );
            assert_eq!(
                engine.native_population_census,
                batched.native_population_census
            );
        }
    }

    #[test]
    fn native_population_seeding_has_no_f1_extinction_cap_or_manual_winner_path() {
        let (mut world, _) = native_contest_world(
            42,
            LocalModeKind::League,
            crate::LeagueWorkflow::PopulationSeeding,
        );
        world.local_mode.config.as_mut().unwrap().maximum_population = 1;
        world.local_mode.config.as_mut().unwrap().maximum_cycles = 1;
        world.start_local_mode().unwrap();
        assert_eq!(world.population(), 5);
        let before = crate::SaveFile::encode(&world).unwrap();
        assert!(
            world
                .adjudicate_local_round(None, "no winner in population seeding".into())
                .is_err()
        );
        assert_eq!(crate::SaveFile::encode(&world).unwrap(), before);
        let ids = world.slots.ids_in_slot_order();
        world.remove_many(&ids).unwrap();
        world.native_cycle = Some(1998);
        world.tick().unwrap();
        assert_eq!(world.population(), 0);
        assert!(world.local_mode.series.population_seeds.is_empty());
        assert!(world.local_mode.results.is_empty());
        assert_eq!(world.local_mode.current_round, 1);
        world.native_cycle = Some(2000);
        world.tick().unwrap(); // actual source2001 does not retroactively score
        assert_eq!(world.native_cycle, Some(2001));
        assert!(world.local_mode.series.population_seeds.is_empty());
    }

    #[test]
    fn native_sun_change_handoff_preserves_uniform_gaussian_guid_and_other_weather() {
        let mut world = Engine::new_native(EngineConfig {
            seed: 42,
            ..EngineConfig::testing()
        })
        .unwrap();
        world.set_native_gaussian_spare(Some(1.25)).unwrap();
        assert_eq!(world.read_native_sun_change(), 0);
        for change in [0_u8, 10, 22, 255] {
            let mut expected = serde_json::to_value(&world).unwrap();
            expected["vegetation_runtime"]["sun_change"] = serde_json::json!(change);
            world.set_native_sun_change(change);
            assert_eq!(serde_json::to_value(&world).unwrap(), expected);
            let saved = crate::SaveFile::encode(&world).unwrap();
            assert_eq!(world.read_native_sun_change(), change);
            assert_eq!(world.read_native_sun_change(), change);
            assert_eq!(crate::SaveFile::encode(&world).unwrap(), saved);
            assert_eq!(world.native_random.as_ref().unwrap().state(), 0x0008d82a);
            assert_eq!(world.read_native_gaussian_spare(), Some(1.25));
            assert_eq!(world.native_sim_guid, Some(0));
        }
        world.native_random = None;
        let mut expected = serde_json::to_value(&world).unwrap();
        expected["vegetation_runtime"]["sun_change"] = serde_json::json!(22);
        world.set_native_sun_change(22);
        assert_eq!(world.read_native_sun_change(), 22);
        assert!(world.native_random.is_none());
        assert_eq!(serde_json::to_value(&world).unwrap(), expected);
    }

    #[test]
    fn native_gaussian_spare_read_set_change_only_the_existing_cache() {
        let mut world = Engine::new_native(EngineConfig {
            seed: 42,
            ..EngineConfig::testing()
        })
        .unwrap();
        for (spare, bits) in [
            (Some(1.25), Some(0x3fa00000)),
            (Some(0.0), Some(0x00000000)),
            (Some(-0.0), Some(0x80000000)),
            (None, None),
        ] {
            let mut expected = serde_json::to_value(&world).unwrap();
            expected["native_random"]["gaussian_spare"] = serde_json::json!(spare);
            world.set_native_gaussian_spare(spare).unwrap();
            assert_eq!(serde_json::to_value(&world).unwrap(), expected);
            let saved = crate::SaveFile::encode(&world).unwrap();
            assert_eq!(world.read_native_gaussian_spare().map(f32::to_bits), bits);
            assert_eq!(world.read_native_gaussian_spare().map(f32::to_bits), bits);
            assert_eq!(crate::SaveFile::encode(&world).unwrap(), saved);
            assert_eq!(world.native_random.as_ref().unwrap().state(), 0x0008d82a);
        }
        world.set_native_gaussian_spare(Some(1.25)).unwrap();
        let random = world.native_random.as_mut().unwrap();
        assert_eq!(random.gaussian(2.0, 3.0), 5.5);
        assert_eq!(random.state(), 0x0008d82a);
        assert_eq!(random.integer(0, 32000), 18780);
    }

    #[test]
    fn native_gaussian_spare_validation_is_atomic_and_does_not_enable_legacy_streams() {
        let mut world = Engine::new_native(EngineConfig::testing()).unwrap();
        world.set_native_gaussian_spare(Some(1.25)).unwrap();
        let before = crate::SaveFile::encode(&world).unwrap();
        for spare in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let error = world.set_native_gaussian_spare(Some(spare)).unwrap_err();
            assert!(error.to_string().contains("finite"));
            assert_eq!(crate::SaveFile::encode(&world).unwrap(), before);
        }
        world.native_random = None;
        let legacy = crate::SaveFile::encode(&world).unwrap();
        assert_eq!(world.read_native_gaussian_spare(), None);
        world.set_native_gaussian_spare(None).unwrap();
        assert_eq!(crate::SaveFile::encode(&world).unwrap(), legacy);
        for spare in [1.25, 0.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert!(world.set_native_gaussian_spare(Some(spare)).is_err());
            assert!(world.native_random.is_none());
            assert_eq!(crate::SaveFile::encode(&world).unwrap(), legacy);
        }
        for boundary in [
            NativeStartBoundary::ConfiguredStart,
            NativeStartBoundary::SameProcessRound,
            NativeStartBoundary::ExternalProcess,
        ] {
            world.prepare_native_restart(boundary);
            assert_eq!(crate::SaveFile::encode(&world).unwrap(), legacy);
        }
    }

    #[test]
    fn configured_native_start_uses_selected_seed_and_keeps_current_gaussian() {
        let mut world = Engine::new_native(EngineConfig {
            seed: 42,
            ..EngineConfig::testing()
        })
        .unwrap();
        world.set_native_gaussian_spare(Some(1.25)).unwrap();
        world.native_random.as_mut().unwrap().unit();
        world.native_random.as_mut().unwrap().unit();
        world.set_native_restart_seed(77);
        world.prepare_native_restart(NativeStartBoundary::ConfiguredStart);
        assert_eq!(world.config.seed, 42);
        assert_eq!(world.config.native_restart_seed, Some(77));
        assert_eq!(world.native_sim_guid, Some(0));
        assert_eq!(world.read_native_gaussian_spare(), Some(1.25));
        let random = world.native_random.as_mut().unwrap();
        assert_eq!(random.state(), 0x0008d82a);
        assert_eq!(random.gaussian(2.0, 3.0), 5.5);
        assert_eq!(random.state(), 0x0008d82a);
        assert_eq!(random.integer(0, 32000), 18780);
    }

    #[test]
    fn external_native_restart_weather_aliases_share_one_literal_sun_header() {
        for (weather, sun) in [(false, false), (true, false), (false, true), (true, true)] {
            let mut config = EngineConfig::testing();
            config.seed = 77;
            config.vegetation.weather_enabled = weather;
            config.vegetation.sun_on_random = sun;
            let mut world = Engine::new_native(config).unwrap();
            world.set_native_gaussian_spare(Some(1.25)).unwrap();
            world.set_native_restart_seed(42);
            world.prepare_native_restart(NativeStartBoundary::ExternalProcess);
            assert_eq!(world.config.seed, 42);
            assert_eq!(world.read_native_gaussian_spare(), None);
            let sunlight = serde_json::to_value(&world.vegetation_runtime).unwrap();
            let (cursor, guid, next_word) = if weather || sun {
                assert_eq!(
                    sunlight["sun_position"].as_f64().unwrap().to_bits(),
                    0x3fe786fe80000000
                );
                assert_eq!(
                    sunlight["sun_range"].as_f64().unwrap().to_bits(),
                    0x3fe0000000000000
                );
                assert_eq!(sunlight["sun_change"], 10);
                (0x008fc6e7, 1, 6581)
            } else {
                assert_eq!(sunlight["sun_position"], 0.5);
                assert_eq!(sunlight["sun_range"], 1.0);
                assert_eq!(sunlight["sun_change"], 0);
                (0x0008d82a, 0, 18780)
            };
            assert_eq!(world.native_sim_guid, Some(guid));
            assert_eq!(world.native_random.as_ref().unwrap().state(), cursor);
            assert_eq!(
                world.native_random.as_mut().unwrap().integer(0, 32000),
                next_word
            );
        }
    }

    fn native_contest_world(
        seed: u64,
        kind: LocalModeKind,
        workflow: crate::LeagueWorkflow,
    ) -> (Engine, SpeciesId) {
        let mut world = Engine::new_native(EngineConfig {
            seed,
            ..EngineConfig::testing()
        })
        .unwrap();
        let dna = LegacyDna::parse("stop").unwrap();
        let mut entrants = Vec::new();
        for name in ["first", "second"] {
            let species = world.register_species(SpeciesDefinition {
                name: name.into(),
                ..Default::default()
            });
            world
                .spawn_species_batch(dna.clone(), species, [[500.0; 2]], 1000)
                .unwrap();
            entrants.push(species);
        }
        world
            .configure_local_mode(LocalModeConfig {
                kind,
                workflow,
                entrant_order: entrants.clone(),
                minimum_rounds: 2,
                maximum_rounds: 100,
                maximum_population: 10,
                ..Default::default()
            })
            .unwrap();
        // Disable randomized sun after applying the F1 preset to isolate header4.
        world.config.vegetation.sun_on_random = false;
        world.config.vegetation.weather_enabled = false;
        (world, entrants[0])
    }

    #[test]
    fn public_direct_contest_start_restart_and_next_round_use_distinct_seed_boundaries() {
        for kind in [LocalModeKind::League, LocalModeKind::Contest] {
            let (mut world, winner) = native_contest_world(42, kind, crate::LeagueWorkflow::Direct);
            world.set_native_gaussian_spare(Some(1.25)).unwrap();
            world.set_native_restart_seed(77);
            for restart in [false, true] {
                if restart {
                    world.restart_local_mode().unwrap();
                } else {
                    world.start_local_mode().unwrap();
                }
                // Header4 plus ten robots * nine draws, independently calculated.
                assert_eq!(world.population(), 10);
                assert_eq!(world.config.seed, 42);
                assert_eq!(world.config.native_restart_seed, Some(77));
                assert_eq!(world.read_native_gaussian_spare(), Some(1.25));
                assert_eq!(world.native_random.as_ref().unwrap().state(), 0x0071d70c);
            }
            world
                .adjudicate_local_round(Some(winner), "next literal F1 round".into())
                .unwrap();
            // The next draw from0x0071d70c is0x0069499f. Its Double product
            // rounds to883216256, then header4+90 draws ends at0x00e6b60c.
            assert_eq!(world.config.seed, 883_216_256);
            assert_eq!(world.read_native_gaussian_spare(), Some(1.25));
            assert_eq!(world.native_random.as_ref().unwrap().state(), 0x00e6b60c);
        }
    }

    #[test]
    fn public_tournament_entry_and_restart_are_external_process_boundaries() {
        let (mut world, _) =
            native_contest_world(77, LocalModeKind::League, crate::LeagueWorkflow::Tournament);
        world.set_native_restart_seed(42);
        world.set_native_gaussian_spare(Some(1.25)).unwrap();
        world.start_local_mode().unwrap();
        assert_eq!(world.config.seed, 42);
        assert_eq!(world.read_native_gaussian_spare(), None);
        assert_eq!(world.native_random.as_ref().unwrap().state(), 0x0071d70c);
        world.set_native_gaussian_spare(Some(2.5)).unwrap();
        world.config.seed = 77;
        world.restart_local_mode().unwrap();
        assert_eq!(world.config.seed, 42);
        assert_eq!(world.read_native_gaussian_spare(), None);
        assert_eq!(world.native_random.as_ref().unwrap().state(), 0x0071d70c);
    }

    #[test]
    fn native_load_and_same_process_start_preserve_weather_change_but_new_process_resets_it() {
        let mut world = Engine::new_native(EngineConfig {
            seed: 42,
            ..EngineConfig::testing()
        })
        .unwrap();
        assert_eq!(
            serde_json::to_value(&world.vegetation_runtime).unwrap()["sun_change"],
            0
        );
        world.vegetation_runtime = serde_json::from_value(serde_json::json!({
            "sun_change": 22, "sun_position": 0.125, "sun_range": 0.75,
            "day_night_counter": 23, "repopulation_counter": 17,
            "native_chloroplast_population": 9,
        }))
        .unwrap();
        let loaded_weather = serde_json::to_value(&world.vegetation_runtime).unwrap();
        let guid = world.native_sim_guid;
        world.initialize_native_load();
        assert_eq!(
            serde_json::to_value(&world.vegetation_runtime).unwrap(),
            loaded_weather
        );
        assert_eq!(world.native_sim_guid, guid);
        world.prepare_native_restart(NativeStartBoundary::ConfiguredStart);
        let configured = serde_json::to_value(&world.vegetation_runtime).unwrap();
        assert_eq!(configured["sun_change"], 22);
        assert_eq!(configured["sun_position"], 0.5);
        assert_eq!(configured["sun_range"], 1.0);
        assert_eq!(configured["day_night_counter"], 23);
        world.prepare_native_restart(NativeStartBoundary::SameProcessRound);
        let same_process = serde_json::to_value(&world.vegetation_runtime).unwrap();
        assert_eq!(same_process["sun_change"], 22);
        assert_eq!(same_process["sun_position"], 0.5);
        assert_eq!(same_process["sun_range"], 1.0);
        assert_eq!(same_process["day_night_counter"], 23);
        world.set_native_restart_seed(42);
        world.prepare_native_restart(NativeStartBoundary::ExternalProcess);
        assert_eq!(
            serde_json::to_value(&world.vegetation_runtime).unwrap()["sun_change"],
            0
        );
        assert_eq!(
            serde_json::to_value(&world.vegetation_runtime).unwrap()["day_night_counter"],
            0
        );
    }

    #[test]
    fn new_native_guid_survives_original_export_and_load_without_another_guid_draw() {
        let world = Engine::new_native(EngineConfig {
            seed: 0,
            ..EngineConfig::testing()
        })
        .unwrap();
        assert_eq!(world.native_sim_guid, Some(1));
        let bytes = world.export_native_sim().unwrap();
        let records = crate::native_binary::NativeRecordFile::decode(&bytes).unwrap();
        let tail = crate::native_binary::NativeSimulationTail::decode(&records.tail).unwrap();
        assert_eq!(tail.integer("sim_guid").unwrap(), 1);
        let loaded = Engine::from_native_sim(&bytes).unwrap();
        let again = loaded.export_native_sim().unwrap();
        let records = crate::native_binary::NativeRecordFile::decode(&again).unwrap();
        let tail = crate::native_binary::NativeSimulationTail::decode(&records.tail).unwrap();
        assert_eq!(tail.integer("sim_guid").unwrap(), 1);
        assert_eq!(loaded.native_random.as_ref().unwrap().state(), 0x00165533);
    }

    #[test]
    fn fresh_native_start_consumes_header_before_public_dna_execution() {
        for (seed, weather, sun, cursor, value, guid) in [
            (42, false, false, 0x0008d82a, 18780, 0),
            (42, true, false, 0x008fc6e7, 6581, 1),
            (42, false, true, 0x008fc6e7, 6581, 1),
            (42, true, true, 0x008fc6e7, 6581, 1),
            (0, false, false, 0x00892b2a, 4158, 1),
            (u64::from(u32::MAX), false, false, 0x008e202a, 4800, 1),
        ] {
            let mut config = EngineConfig::testing();
            config.seed = seed;
            config.vegetation.weather_enabled = weather;
            config.vegetation.sun_on_random = sun;
            let mut world = Engine::new_native(config).unwrap();
            assert_eq!(world.native_random.as_ref().unwrap().state(), cursor);
            assert_eq!(world.native_sim_guid, Some(guid));
            let sunlight = serde_json::to_value(&world.vegetation_runtime).unwrap();
            if weather || sun {
                assert_eq!(
                    sunlight["sun_position"].as_f64().unwrap().to_bits(),
                    0x3fe786fe80000000
                );
                assert_eq!(
                    sunlight["sun_range"].as_f64().unwrap().to_bits(),
                    0x3fe0000000000000
                );
                assert_eq!(sunlight["sun_change"], 10);
            } else {
                assert_eq!(sunlight["sun_position"], 0.5);
                assert_eq!(sunlight["sun_range"], 1.0);
                assert_eq!(sunlight["sun_change"], 0);
            }
            let id = world
                .spawn_at(
                    LegacyDna::parse("start 32000 rnd 900 store stop").unwrap(),
                    [500.0; 2],
                )
                .unwrap();
            world.execute_robots().unwrap();
            assert_eq!(world.memory_at(id, 900).unwrap(), value);
        }
    }

    #[test]
    fn same_process_round_uses_double_long_seed_and_preserves_cached_gaussian() {
        let mut world = Engine::new_native(EngineConfig {
            seed: 42,
            ..EngineConfig::testing()
        })
        .unwrap();
        world.native_random = Some(
            serde_json::from_value(serde_json::json!({
                "state": 0x0008d82a_u32, "gaussian_spare": 1.25
            }))
            .unwrap(),
        );
        world.prepare_native_restart(NativeStartBoundary::SameProcessRound);
        assert_eq!(world.config.seed, 1_260_331_647);
        assert_eq!(world.native_random.as_ref().unwrap().state(), 0x00e3292a);
        let random = world.native_random.as_mut().unwrap();
        assert_eq!(random.gaussian(2.0, 3.0), 5.5);
        assert_eq!(random.state(), 0x00e3292a);
        assert_eq!(random.integer(0, 32000), 17412);
    }

    #[test]
    fn external_restart_uses_recorded_clock_seed_and_clears_process_gaussian() {
        let mut world = Engine::new_native(EngineConfig {
            seed: 0,
            ..EngineConfig::testing()
        })
        .unwrap();
        world.native_random = Some(
            serde_json::from_value(serde_json::json!({
                "state": 0x00123456_u32, "gaussian_spare": 1.25
            }))
            .unwrap(),
        );
        world.set_native_restart_seed(42);
        let saved = crate::SaveFile::encode(&world).unwrap();
        let mut resumed = crate::SaveFile::decode(&saved).unwrap();
        for engine in [&mut world, &mut resumed] {
            engine.prepare_native_restart(NativeStartBoundary::ExternalProcess);
            assert_eq!(engine.config.seed, 42);
            assert_eq!(engine.native_random.as_ref().unwrap().state(), 0x0008d82a);
            assert!(
                serde_json::to_value(&engine.native_random).unwrap()["gaussian_spare"].is_null()
            );
            assert_eq!(
                engine.native_random.as_mut().unwrap().integer(0, 32000),
                18780
            );
        }
        assert_eq!(world.native_random, resumed.native_random);
    }

    #[test]
    fn native_restart_guid_weather_seed_and_rng_roll_back_together() {
        let mut world = Engine::new_native(EngineConfig {
            seed: 42,
            ..EngineConfig::testing()
        })
        .unwrap();
        let before = serde_json::to_value(&world).unwrap();
        let checkpoint = world.cycle_checkpoint();
        world.set_native_restart_seed(0);
        world.config.vegetation.sun_on_random = true;
        world.prepare_native_restart(NativeStartBoundary::ExternalProcess);
        assert_eq!(world.native_sim_guid, Some(0));
        world.restore_cycle_checkpoint(checkpoint);
        let after = serde_json::to_value(&world).unwrap();
        for key in [
            "native_sim_guid",
            "native_random",
            "vegetation_runtime",
            "config",
        ] {
            assert_eq!(after[key], before[key], "rollback {key}");
        }
    }

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
    fn native_start_batch_interleaves_nine_draw_robots_before_later_vm_draws() {
        let mut world = Engine::new_native(EngineConfig {
            seed: 42,
            world_width: 16000.0,
            world_height: 12000.0,
            ..EngineConfig::testing()
        })
        .unwrap();
        let species = world.register_species(SpeciesDefinition {
            placement_bounds: [0.25, 0.5, 0.5, 0.75],
            ..Default::default()
        });
        let ids = world
            .spawn_species_batch_with_native_placement(
                LegacyDna::parse("start 32000 rnd 900 store stop").unwrap(),
                species,
                vec![[0.0; 2]; 2],
                3000,
                true,
            )
            .unwrap();
        // Independent VBMath/Single arithmetic literals: header4, nine draws
        // per robot, then DNA draws 23 and 24 in slot order.
        for (index, position, heading_bits, timer, cursor) in [
            (0, [4673.0, 7883.0], 0x4061eb85, -17308, 0x003ac52d),
            (1, [4481.0, 6544.0], 0x40400000, -13403, 0x004a6384),
        ] {
            let robot = world.slots.get(ids[index]).unwrap();
            assert_eq!(robot.kinematics.position, position);
            assert_eq!(robot.heading().to_bits(), heading_bits);
            assert_eq!(robot.program.memory.read(12), timer);
            assert_eq!(robot.program.rng_state, cursor);
        }
        assert_eq!(world.native_random.as_ref().unwrap().state(), 0x004a6384);
        world
            .execute_dna_phase(&mut CycleRecorder::disabled(1))
            .unwrap();
        assert_eq!(world.memory_at(ids[0], 900).unwrap(), 24940);
        assert_eq!(world.memory_at(ids[1], 900).unwrap(), 28835);
        assert_eq!(world.native_random.as_ref().unwrap().state(), 0x00e6ae1e);
    }

    #[test]
    fn native_start_robot_keeps_species_order_and_literal_heading_timer() {
        let mut world = Engine::new_native(EngineConfig {
            seed: 42,
            world_width: 16000.0,
            world_height: 12000.0,
            ..EngineConfig::testing()
        })
        .unwrap();
        let first_species = world.register_species(SpeciesDefinition {
            placement_bounds: [0.25, 0.5, 0.5, 0.75],
            ..Default::default()
        });
        let second_species = world.register_species(SpeciesDefinition {
            placement_bounds: [0.1, 0.2, 0.2, 0.3],
            ..Default::default()
        });
        let dna = LegacyDna::parse("stop").unwrap();
        let first = world
            .spawn_native_start_robot(dna.clone(), first_species, 1, None)
            .unwrap();
        let second = world
            .spawn_native_start_robot(dna, second_species, 2, None)
            .unwrap();
        assert!(first.slot() < second.slot());
        for (id, species, position, heading_bits, aim, timer, cursor) in [
            (
                first,
                first_species,
                [4673.0, 7883.0],
                0x4061eb85,
                706,
                -17308,
                0x003ac52d,
            ),
            (
                second,
                second_species,
                [1792.0, 2617.0],
                0x40400000,
                600,
                -13403,
                0x004a6384,
            ),
        ] {
            let robot = world.slots.get(id).unwrap();
            assert_eq!(robot.identity.species, species);
            assert_eq!(robot.kinematics.position, position);
            assert_eq!(robot.heading().to_bits(), heading_bits);
            assert_eq!(robot.program.memory.read(19), aim);
            assert_eq!(robot.program.memory.read(12), timer);
            assert_eq!(robot.program.rng_state, cursor);
        }
        assert_eq!(world.native_random.as_ref().unwrap().state(), 0x004a6384);
    }

    #[test]
    fn native_start_coordinates_use_single_bounds_and_floor_without_rounding_endpoints() {
        for (low, high, x) in [
            (0.0, f32::from_bits(0x3d9faf42), 215.0),
            (0.5001, 0.5001, 7971.0),
        ] {
            let mut world = Engine::new_native(EngineConfig {
                seed: 42,
                world_width: 16000.0,
                world_height: 12000.0,
                ..EngineConfig::testing()
            })
            .unwrap();
            let species = world.register_species(SpeciesDefinition {
                placement_bounds: [low, high, 0.5, 0.75],
                ..Default::default()
            });
            let id = world
                .spawn_native_start_robot(LegacyDna::parse("stop").unwrap(), species, 0, None)
                .unwrap();
            // First fixture: Single gives215, Double214.99999792971357.
            // Second fixture: flooring fractional endpoints gives7971;
            // pre-rounding the endpoints would give7972.
            assert_eq!(
                world.slots.get(id).unwrap().kinematics.position,
                [x, 7883.0]
            );
            assert_eq!(world.native_random.as_ref().unwrap().state(), 0x003ac52d);
        }
    }

    #[test]
    fn native_start_heading_is_single_but_timer_is_double_and_override_keeps_all_draws() {
        for (cursor, heading_bits, timer, after) in [
            (0x003b30f1_u32, 0x3e9eb852, 18802, 0x00cb34b8),
            (0x0010a717_u32, 0x4041eb85, -32000, 0x00000106),
        ] {
            let mut world = Engine::new_native(EngineConfig::testing()).unwrap();
            world.native_random = Some(
                serde_json::from_value(serde_json::json!({
                    "state": cursor, "gaussian_spare": 1.25,
                }))
                .unwrap(),
            );
            let id = world
                .spawn_native_start_robot(
                    LegacyDna::parse("stop").unwrap(),
                    SpeciesId::default(),
                    0,
                    Some([321.0, 654.0]),
                )
                .unwrap();
            let robot = world.slots.get(id).unwrap();
            // Heading fixture's Single product is31 vs Double30.9999991655.
            // Timer fixture is-31999.0005337 in Double vs-31999 in Single.
            assert_eq!(robot.heading().to_bits(), heading_bits);
            assert_eq!(robot.program.memory.read(12), timer);
            assert_eq!(robot.kinematics.position, [321.0, 654.0]);
            assert_eq!(robot.program.memory.read(MEM_XPOS), 321);
            assert_eq!(robot.program.memory.read(MEM_YPOS), 654);
            assert_eq!(robot.program.rng_state, u64::from(after));
            assert_eq!(world.native_random.as_ref().unwrap().state(), after);
            let random = world.native_random.as_mut().unwrap();
            assert_eq!(random.gaussian(2.0, 3.0), 5.5);
            assert_eq!(random.state(), after);
        }
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
