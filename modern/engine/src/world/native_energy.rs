//! Vegs.bas process-global energy ring and Master.UpdateSim's delayed readback.
use super::*;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct NativeEnergyRuntime {
    pub(crate) ring: Vec<i32>,
    pub(crate) current_index: u8,
    pub(crate) displayed: i32,
}

impl Default for NativeEnergyRuntime {
    fn default() -> Self {
        Self {
            ring: vec![0; 101],
            current_index: 0,
            displayed: 0,
        }
    }
}

impl NativeEnergyRuntime {
    pub(super) fn validate(&self) -> Result<(), EngineError> {
        if self.ring.len() != 101 || self.current_index >= 100 {
            return Err(EngineError::Invariant(
                "native energy ring needs 101 Long cells and an index in 0..100".into(),
            ));
        }
        Ok(())
    }

    fn begin_cycle(&mut self, cycle: u64) {
        // Master.bas236-238 reads the OLD cursor before selecting and clearing the new one.
        self.displayed = self.ring[usize::from(self.current_index)];
        self.current_index = (cycle % 100) as u8;
        self.ring[usize::from(self.current_index)] = 0;
    }

    fn assign_current(&mut self, value: f64) -> Result<(), EngineError> {
        let value = value.round_ties_even();
        if !value.is_finite() || value < f64::from(i32::MIN) || value > f64::from(i32::MAX) {
            return Err(EngineError::Invariant(
                "native total simulation energy exceeds Long range".into(),
            ));
        }
        self.ring[usize::from(self.current_index)] = value as i32;
        Ok(())
    }

    fn add_shot(&mut self, energy: f32) -> Result<(), EngineError> {
        // Long + Single is promoted before assignment back to Long, once per visited -2 shot.
        self.assign_current(
            f64::from(self.ring[usize::from(self.current_index)]) + f64::from(energy),
        )
    }

    fn add_actor(&mut self, energy: f32, body: f32) -> Result<(), EngineError> {
        // Robots.bas1640: body * Integer10 is Single; the Long-led sum is Double.
        let body_energy = body * 10.0_f32;
        let sum = (f64::from(self.ring[usize::from(self.current_index)]) + f64::from(energy))
            + f64::from(body_energy);
        self.assign_current(sum)
    }

    fn delta(&self) -> i64 {
        // Normal-range Long subtraction is integer-valued despite the Double status destination.
        // i64 transports it exactly; overflowing VB6 UI subtraction is not certified by this projection.
        i64::from(self.displayed)
            - i64::from(self.ring[(usize::from(self.current_index) + 98) % 100])
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct NativeProcessCaches {
    #[serde(default = "default_true")]
    pub(crate) death_poffs_enabled: bool,
    pub(crate) light_availability: f64,
    pub(crate) energy: NativeEnergyRuntime,
    #[serde(default)]
    pub(crate) shots_this_cycle: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) repopulation_counter: Option<i32>,
}

impl Engine {
    pub(super) fn begin_native_energy_cycle(&mut self) -> Result<(), EngineError> {
        if self.native_random.is_some() {
            self.native_energy.validate()?;
            let cycle = self.native_cycle_or_tick();
            self.native_energy.begin_cycle(cycle);
        }
        self.project_native_energy_stats();
        Ok(())
    }

    pub(super) fn add_native_shot_energy(&mut self, energy: f32) -> Result<(), EngineError> {
        if self.native_random.is_some() {
            self.native_energy.add_shot(energy)?;
        }
        Ok(())
    }

    pub(super) fn add_native_actor_energy(&mut self, id: OrganismId) -> Result<(), EngineError> {
        if self.native_random.is_none() {
            return Ok(());
        }
        if let Ok(robot) = self.slots.get(id)
            && robot.flags.exists
        {
            let (energy, body) = (robot.materials.energy, robot.materials.body);
            self.native_energy.add_actor(energy, body)?;
        }
        Ok(())
    }

    pub(super) fn native_energy_displayed(&self) -> i32 {
        self.native_energy.displayed
    }

    pub(super) fn project_native_energy_stats(&mut self) {
        if self.native_random.is_some() {
            self.stats.native_energy_displayed = Some(self.native_energy.displayed);
            self.stats.native_energy_delta = Some(self.native_energy.delta());
        } else {
            self.stats.native_energy_displayed = None;
            self.stats.native_energy_delta = None;
        }
    }

    pub(super) fn reset_native_energy_process(&mut self) {
        self.native_energy = NativeEnergyRuntime::default();
        self.stats.native_shots_visited = self.native_random.is_some().then_some(0);
        self.snapshot.stats.native_shots_visited = self.stats.native_shots_visited;
        self.project_native_energy_stats();
    }

    pub(crate) fn read_native_process_caches(&self) -> Result<NativeProcessCaches, EngineError> {
        Ok(NativeProcessCaches {
            death_poffs_enabled: self.death_poffs_enabled,
            shots_this_cycle: self.stats.native_shots_visited.unwrap_or(0),
            light_availability: self.native_light_availability(),
            energy: self.native_energy.clone(),
            repopulation_counter: if self.native_random.is_some() {
                Some(self.vegetation_runtime.native_repopulation_counter()?)
            } else {
                None
            },
        })
    }

    pub(crate) fn set_native_process_caches(
        &mut self,
        caches: NativeProcessCaches,
    ) -> Result<(), EngineError> {
        if !caches.light_availability.is_finite()
            || !(0.0..=1.0).contains(&caches.light_availability)
        {
            return Err(EngineError::Invariant(
                "native light availability must be finite and in 0..=1".into(),
            ));
        }
        caches.energy.validate()?;
        if caches.shots_this_cycle > i32::MAX as u64 {
            return Err(EngineError::Invariant(
                "native ShotsThisCycle exceeds Long range".into(),
            ));
        }
        if let Some(counter) = caches.repopulation_counter {
            self.vegetation_runtime
                .set_native_repopulation_counter(counter);
        }
        self.set_death_poffs_enabled(caches.death_poffs_enabled);
        self.set_native_light_availability(caches.light_availability);
        self.native_energy = caches.energy;
        self.stats.native_shots_visited = self
            .native_random
            .is_some()
            .then_some(caches.shots_this_cycle);
        self.project_native_energy_stats();
        // A context handoff is observable immediately without a cycle, publication, or RNG draw.
        self.snapshot.stats.native_shots_visited = self.stats.native_shots_visited;
        self.snapshot.stats.native_energy_displayed = self.stats.native_energy_displayed;
        self.snapshot.stats.native_energy_delta = self.stats.native_energy_delta;
        Ok(())
    }

    pub(crate) fn inherit_native_process_caches_after_sim_load(
        &mut self,
        previous: &Self,
    ) -> Result<(), EngineError> {
        let mut caches = previous.read_native_process_caches()?;
        // startloaded explicitly resets Vegs.cooldown even in the same process.
        // Other process caches survive loading the original SIM format.
        caches.repopulation_counter = None;
        self.set_native_process_caches(caches)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_long_assignments_round_each_add_and_preserve_long_precision() {
        let mut state = NativeEnergyRuntime::default();
        state.add_shot(0.5).unwrap();
        state.add_shot(0.5).unwrap();
        assert_eq!(state.ring[0], 0); // Rounding the final sum once would give1.
        state.add_shot(1.5).unwrap();
        assert_eq!(state.ring[0], 2);
        state.ring[0] = 0;
        state.add_actor(0.5, 0.0).unwrap();
        state.add_actor(0.5, 0.0).unwrap();
        assert_eq!(state.ring[0], 0); // Round at EACH actor assignment, not after summing all actors.
        state.add_actor(1.5, 0.0).unwrap();
        assert_eq!(state.ring[0], 2);
        state.ring[0] = 16_777_217;
        state.add_actor(0.0, 0.0).unwrap();
        assert_eq!(state.ring[0], 16_777_217);
        state.ring[0] = 0;
        state.add_actor(0.0, 0.05).unwrap();
        assert_eq!(state.ring[0], 0); // body*10 first rounds to Single0.5.
        state.ring[0] = i32::MAX;
        let before = state.clone();
        assert!(state.add_shot(1.0).is_err());
        assert_eq!(state, before);
        assert!(state.add_actor(f32::NAN, 0.0).is_err());
        assert_eq!(state, before);
    }

    #[test]
    fn ring_reads_previous_cursor_before_clear_and_keeps_exact_hundred_cycle_wrap() {
        let mut state = NativeEnergyRuntime::default();
        state.begin_cycle(0);
        assert_eq!((state.displayed, state.delta()), (0, 0));
        for cycle in 0..=102 {
            if cycle > 0 {
                state.begin_cycle(cycle);
                assert_eq!(state.displayed, cycle as i32 * 10);
                assert_eq!(state.delta(), 10);
            }
            state.add_shot((cycle + 1) as f32 * 10.0).unwrap();
        }
        assert_eq!(state.current_index, 2);
        assert_eq!(state.ring[100], 0);
    }

    #[test]
    fn invalid_context_rejects_process_cache_parts_without_mutation_or_randomness() {
        let mut world = Engine::new_native(EngineConfig::testing()).unwrap();
        let mut original = world.read_native_process_caches().unwrap();
        original.shots_this_cycle = 17;
        world.set_native_process_caches(original.clone()).unwrap();
        let save = crate::SaveFile::encode(&world).unwrap();
        let snapshot = serde_json::to_value(world.snapshot()).unwrap();
        let publications = world.snapshot_publication_count();
        for invalid in [
            NativeProcessCaches {
                death_poffs_enabled: true,
                shots_this_cycle: 0,
                light_availability: f64::NAN,
                energy: original.energy.clone(),
            },
            NativeProcessCaches {
                death_poffs_enabled: true,
                shots_this_cycle: 0,
                light_availability: -0.1,
                energy: original.energy.clone(),
            },
            NativeProcessCaches {
                death_poffs_enabled: true,
                shots_this_cycle: 0,
                light_availability: 1.1,
                energy: original.energy.clone(),
            },
            NativeProcessCaches {
                death_poffs_enabled: true,
                shots_this_cycle: 0,
                light_availability: 0.75,
                energy: NativeEnergyRuntime {
                    ring: vec![0; 100],
                    ..original.energy.clone()
                },
            },
            NativeProcessCaches {
                death_poffs_enabled: true,
                shots_this_cycle: 0,
                light_availability: 0.75,
                energy: NativeEnergyRuntime {
                    current_index: 100,
                    ..original.energy.clone()
                },
            },
            NativeProcessCaches {
                death_poffs_enabled: true,
                shots_this_cycle: i32::MAX as u64 + 1,
                light_availability: 0.75,
                energy: original.energy.clone(),
            },
        ] {
            assert!(world.set_native_process_caches(invalid).is_err());
            assert_eq!(world.read_native_process_caches().unwrap(), original);
            assert_eq!(crate::SaveFile::encode(&world).unwrap(), save);
            assert_eq!(serde_json::to_value(world.snapshot()).unwrap(), snapshot);
            assert_eq!(world.snapshot_publication_count(), publications);
        }
        let mut upper = original.clone();
        upper.shots_this_cycle = i32::MAX as u64;
        world.set_native_process_caches(upper.clone()).unwrap();
        assert_eq!(
            world.snapshot().stats.native_shots_visited,
            Some(i32::MAX as u64)
        );
        upper.shots_this_cycle = 0;
        world.set_native_process_caches(upper).unwrap();
        assert_eq!(world.snapshot().stats.native_shots_visited, Some(0));
    }

    #[test]
    fn native_sun_thresholds_read_delayed_energy_before_current_feed() {
        let mut config = EngineConfig::testing();
        config.vegetation.max_energy_per_tick = 7;
        config.vegetation.feeding_to_body = 0.0;
        config.vegetation.daytime = false;
        config.vegetation.sun_up_enabled = true;
        config.vegetation.sun_up_threshold = 100;
        config.vegetation.sun_down_enabled = true;
        config.vegetation.sun_down_threshold = 1_500;
        config.vegetation.sun_threshold_mode = 0;
        let mut world = Engine::new_native(config).unwrap();
        let id = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [500.0; 2])
            .unwrap();
        let robot = world.slots.get_mut(id).unwrap();
        robot.flags.can_execute_dna = false;
        robot.flags.can_mutate = false;
        robot.materials.energy = 1_000.0;
        robot.materials.body = 100.0;
        robot.materials.chloroplasts = 16_000.0;
        world.tick().unwrap();
        let fed = world.slots.get(id).unwrap().materials.energy;
        assert!(fed > 1_000.0); // Current resources are above SunUp; previous displayed0 is below it.
        assert_eq!(world.native_energy.ring[0], 2_000); // Before late FeedVeg.
        assert_eq!(world.snapshot().stats.native_energy_displayed, Some(0));
        world.tick().unwrap();
        assert_eq!(world.slots.get(id).unwrap().materials.energy, fed); // Previous2000 triggers SunDown.
        assert_eq!(world.snapshot().stats.native_energy_displayed, Some(2_000));
        assert_eq!(world.snapshot().stats.native_energy_delta, Some(2_000));
    }

    #[test]
    fn total_energy_phase_counts_hidden_and_queued_death_before_removal() {
        let mut config = EngineConfig::testing();
        config.vegetation.max_energy_per_tick = 0;
        config.corpse_decay.enabled = false;
        let mut world = Engine::new_native(config).unwrap();
        let mut ids = Vec::new();
        for (index, energy, body) in [(0, 1_000.0, 100.0), (1, 50.0, 10.0), (2, 0.25, 10.0)] {
            let id = world
                .spawn_at(
                    LegacyDna::parse("stop").unwrap(),
                    [100.0 + index as f32 * 300.0, 500.0],
                )
                .unwrap();
            let robot = world.slots.get_mut(id).unwrap();
            robot.flags.can_execute_dna = false;
            robot.flags.can_mutate = false;
            robot.materials.energy = energy;
            robot.materials.body = body;
            robot.flags.hidden = index == 1;
            ids.push(id);
        }
        world.tick().unwrap();
        assert!(world.slots.get(ids[2]).is_err());
        assert!(world.slots.get(ids[1]).unwrap().flags.exists);
        assert_eq!(world.native_energy.ring[0], 2_250);
        assert_eq!(world.snapshot().stats.native_energy_displayed, Some(0));
        assert_eq!(world.stats.native_shots_visited, Some(0));
        assert_eq!(world.snapshot().shots.len(), 20);
        assert!(
            world
                .snapshot()
                .shots
                .iter()
                .all(|shot| shot.kind == -100 && shot.age == 0)
        );
        world.tick().unwrap();
        assert_eq!(world.stats.native_shots_visited, Some(20));
        assert_eq!(world.native_energy.displayed, 2_250);
        assert_eq!(
            world.native_energy.ring[1], 2_150,
            "ornamental281energy does not enter the -2shot energy sum"
        );
    }

    #[test]
    fn cache_survives_db3s_and_cycle_checkpoint_and_same_process_starts_only() {
        let mut world = Engine::new_native(EngineConfig::testing()).unwrap();
        world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [500.0; 2])
            .unwrap();
        let mut caches = world.read_native_process_caches().unwrap();
        caches.light_availability = 0.125;
        caches.shots_this_cycle = 17;
        caches.death_poffs_enabled = false;
        caches.repopulation_counter = Some(-7);
        caches.energy.ring[4] = 321;
        caches.energy.current_index = 4;
        caches.energy.displayed = 123;
        world.set_native_process_caches(caches.clone()).unwrap();
        let checkpoint = crate::SaveFile::encode(&world).unwrap();
        assert_eq!(
            world
                .read_native_process_caches()
                .unwrap()
                .repopulation_counter,
            Some(-7)
        );
        assert_eq!(crate::SaveFile::encode(&world).unwrap(), checkpoint);
        let mut restored = crate::SaveFile::decode(&checkpoint).unwrap();
        assert_eq!(restored.read_native_process_caches().unwrap(), caches);
        assert_eq!(restored.snapshot().stats.native_shots_visited, Some(17));
        let rollback = restored.cycle_checkpoint();
        restored.begin_native_energy_cycle().unwrap();
        restored.add_native_shot_energy(17.0).unwrap();
        restored.set_native_light_availability(0.75);
        restored.stats.native_shots_visited = Some(0);
        restored.set_death_poffs_enabled(true);
        restored
            .vegetation_runtime
            .set_native_repopulation_counter(123);
        restored.restore_cycle_checkpoint(rollback);
        assert_eq!(restored.read_native_process_caches().unwrap(), caches);
        for boundary in [
            native_random_world::NativeStartBoundary::ConfiguredStart,
            native_random_world::NativeStartBoundary::SameProcessRound,
        ] {
            restored.prepare_native_restart(boundary);
            assert_eq!(restored.read_native_process_caches().unwrap(), caches);
        }
        restored.prepare_native_restart(native_random_world::NativeStartBoundary::ExternalProcess);
        assert_eq!(
            restored.read_native_process_caches().unwrap(),
            NativeProcessCaches {
                death_poffs_enabled: true,
                shots_this_cycle: 0,
                repopulation_counter: Some(0),
                light_availability: 0.0,
                energy: NativeEnergyRuntime::default()
            }
        );
        assert_eq!(restored.snapshot().stats.native_shots_visited, Some(0));
    }

    #[test]
    fn absent_old_checkpoint_cache_defaults_zero_but_malformed_saved_ring_is_rejected() {
        let world = Engine::new_native(EngineConfig::testing()).unwrap();
        let mut payload = world.db3s_v3_payload().unwrap();
        payload.as_object_mut().unwrap().remove("native_energy");
        let old = Engine::restore(Engine::from_db3s_v3_payload(payload.clone()).unwrap()).unwrap();
        let old_context: NativeProcessCaches = serde_json::from_value(serde_json::json!({
            "light_availability":0.0,"energy":{"ring":vec![0;101],"current_index":0,"displayed":0}
        }))
        .unwrap();
        assert!(old_context.death_poffs_enabled);
        assert!(old.death_poffs_enabled);
        assert!(old.snapshot().death_poffs_enabled);
        assert!(Snapshot::default().death_poffs_enabled);
        let mut old_snapshot = serde_json::to_value(old.snapshot()).unwrap();
        old_snapshot
            .as_object_mut()
            .unwrap()
            .remove("death_poffs_enabled");
        let old_snapshot: Snapshot = serde_json::from_value(old_snapshot).unwrap();
        assert!(old_snapshot.death_poffs_enabled);
        assert!(
            serde_json::to_value(&old)
                .unwrap()
                .get("death_poffs_enabled")
                .is_none()
        );
        assert_eq!(old_context.shots_this_cycle, 0);
        assert_eq!(old_context.repopulation_counter, None);
        assert_eq!(
            old.read_native_process_caches()
                .unwrap()
                .repopulation_counter,
            Some(0)
        );
        assert_eq!(
            old.read_native_process_caches().unwrap().shots_this_cycle,
            0
        );
        assert_eq!(old.native_energy, NativeEnergyRuntime::default());
        let frame = |payload: &serde_json::Value| {
            let encoded = serde_json::to_vec(payload).unwrap();
            let mut bytes = b"DB3S".to_vec();
            bytes.extend_from_slice(&3_u16.to_le_bytes());
            bytes.extend_from_slice(&(encoded.len() as u32).to_le_bytes());
            bytes.extend_from_slice(&encoded);
            bytes
        };
        for (length, current_index) in [
            (0, 0),
            (1, 0),
            (100, 0),
            (102, 0),
            (1, 98),
            (101, 100),
            (101, 255),
        ] {
            let mut malformed = payload.clone();
            malformed["native_energy"] = serde_json::json!({
                "ring": vec![0; length], "current_index": current_index, "displayed": 0
            });
            let error = Engine::restore(Engine::from_db3s_v3_payload(malformed.clone()).unwrap())
                .err()
                .expect("malformed energy must return an error before publication");
            assert!(
                matches!(error, EngineError::InvalidSave(ref message) if message.contains("native energy cache"))
            );
            let error = crate::SaveFile::decode(&frame(&malformed))
                .err()
                .expect("public decode must reject malformed energy before publication");
            assert!(
                matches!(error, EngineError::InvalidSave(ref message) if message.contains("native energy cache"))
            );
        }
        // A boundary index remains valid: the source delta reads ring[(99+98)%100].
        let mut ring = vec![0; 101];
        ring[97] = 5;
        payload["native_energy"] = serde_json::json!({
            "ring": ring, "current_index": 99, "displayed": 7
        });
        let valid = crate::SaveFile::decode(&frame(&payload)).unwrap();
        assert_eq!(valid.native_energy.current_index, 99);
        assert_eq!(valid.snapshot().stats.native_energy_displayed, Some(7));
        assert_eq!(valid.snapshot().stats.native_energy_delta, Some(2));
    }

    #[test]
    fn legacy_route_keeps_energy_totals_and_omits_native_status_projections() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        world.native_random = None;
        world.native_energy.displayed = 123;
        world.stats.native_energy_displayed = Some(123);
        world.stats.native_energy_delta = Some(456);
        let state = world.native_energy.clone();
        world.begin_native_energy_cycle().unwrap();
        world.add_native_shot_energy(7.0).unwrap();
        assert_eq!(world.native_energy, state);
        assert_eq!(world.stats.native_energy_displayed, None);
        assert_eq!(world.stats.native_energy_delta, None);
        let mut legacy_context = world.read_native_process_caches().unwrap();
        legacy_context.shots_this_cycle = 17;
        world.set_native_process_caches(legacy_context).unwrap();
        assert_eq!(world.stats.native_shots_visited, None);
        assert_eq!(world.snapshot().stats.native_shots_visited, None);
        let encoded = serde_json::to_value(&world.stats).unwrap();
        assert!(encoded.get("native_energy_displayed").is_none());
        assert!(encoded.get("native_energy_delta").is_none());
    }
}
