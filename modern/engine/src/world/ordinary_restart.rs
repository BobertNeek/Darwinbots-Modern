//! Same-process extinction restart for a world with its original startup roster.
use super::*;

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct OrdinaryRestart {
    pub(super) enabled: bool,
    pub(super) pending: bool,
    pub(super) count: u64,
    starting_obstacles: Vec<Obstacle>,
}

impl OrdinaryRestart {
    pub(super) fn is_default(&self) -> bool {
        !self.enabled && !self.pending && self.count == 0 && self.starting_obstacles.is_empty()
    }
}

impl Engine {
    pub(super) fn ordinary_restart_available(&self) -> bool {
        self.native_random.is_some()
            && self.native_origin.is_none()
            && !self.native_f1_enabled()
            && self.local_mode.config.is_none()
            && self.local_mode.status == LocalModeStatus::Idle
    }

    pub(crate) fn set_ordinary_restart(&mut self, enabled: bool) -> Result<(), EngineError> {
        if enabled && !self.ordinary_restart_available() {
            return Err(EngineError::Invariant(
                "Ordinary extinction restart requires a native world with its startup roster."
                    .into(),
            ));
        }
        if self.ordinary_restart.enabled != enabled {
            self.ordinary_restart = OrdinaryRestart {
                enabled,
                starting_obstacles: if enabled {
                    self.obstacles.clone()
                } else {
                    Vec::new()
                },
                ..OrdinaryRestart::default()
            };
            self.publish_snapshot();
        }
        Ok(())
    }

    /// Finish the previously completed cycle's restart after the host records
    /// deaths and saves any checkpoint due for that cycle.
    pub(crate) fn finish_ordinary_restart(&mut self) -> Result<bool, EngineError> {
        self.finish_pending_ordinary_restart(true)
    }

    pub(super) fn finish_pending_ordinary_restart(
        &mut self,
        publish: bool,
    ) -> Result<bool, EngineError> {
        if !self.ordinary_restart.pending {
            return Ok(false);
        }
        if !self.ordinary_restart.enabled || !self.ordinary_restart_available() {
            self.ordinary_restart.pending = false;
            if publish {
                self.publish_snapshot();
            }
            return Ok(false);
        }
        let checkpoint = self.cycle_checkpoint();
        if let Err(error) = self.restart_ordinary_world() {
            self.restore_cycle_checkpoint(checkpoint);
            return Err(error);
        }
        self.ordinary_restart.pending = false;
        self.ordinary_restart.count = self.ordinary_restart.count.saturating_add(1);
        if publish {
            self.publish_snapshot();
        }
        Ok(true)
    }

    fn restart_ordinary_world(&mut self) -> Result<(), EngineError> {
        self.prepare_native_restart(NativeStartBoundary::SameProcessRound);
        self.clear_restart_actors()?;
        // Reproduce the original fresh slot order while preserving modern
        // handle generations and the process's absolute ID counter.
        self.slots
            .free_slots
            .sort_unstable_by(|left, right| right.cmp(left));
        self.stats.births = 0;
        self.player_bot.focus = None;
        self.obstacles
            .clone_from(&self.ordinary_restart.starting_obstacles);
        self.pending_gpu_render_instances = None;
        self.pending_gpu_collision_pairs = None;

        for (index, template) in self.species_templates.clone().into_iter().enumerate() {
            let Some(template) = template.filter(|template| !template.retired) else {
                continue;
            };
            let species = SpeciesId(index as u32);
            for _ in 0..template.initial_population {
                if self.available_robot_capacity() == 0 {
                    return Ok(());
                }
                let id = self.spawn_native_start_robot(template.dna.clone(), species, 0, None)?;
                let robot = self.slots.get_mut(id)?;
                robot.materials.energy = template.initial_energy as f32;
                robot.materials.synchronize_energy();
                robot.program.memory.write(MEM_NRG, template.initial_energy);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SaveFile;

    #[test]
    fn census_waits_for_the_next_cycle_and_recovery_preserves_the_restart_boundary() {
        let mut world = Engine::new_native(EngineConfig {
            corpse_decay: CorpseDecaySettings {
                enabled: false,
                ..Default::default()
            },
            ..EngineConfig::testing()
        })
        .unwrap();
        let animal_species = world.register_species(SpeciesDefinition {
            name: "Animal.txt".into(),
            placement_bounds: [0.25, 0.25, 0.75, 0.75],
            ..SpeciesDefinition::default()
        });
        let animal_dna = LegacyDna::parse("start 11 700 store stop").unwrap();
        let animal = world
            .spawn_species_batch(animal_dna.clone(), animal_species, [[300.0; 2]], 763)
            .unwrap()[0];
        let animal_absolute_id = world.slots.get(animal).unwrap().identity.absolute_id;
        let plant_species = world.register_species(SpeciesDefinition {
            name: "Plant.txt".into(),
            vegetable: true,
            ..SpeciesDefinition::default()
        });
        let plant = world
            .spawn_species_batch(
                LegacyDna::parse("stop").unwrap(),
                plant_species,
                [[700.0; 2]],
                850,
            )
            .unwrap()[0];
        let starting_shape = Obstacle::new(1, [100.0; 2], [150.0; 2]);
        world.obstacles.push(starting_shape.clone());
        world.set_ordinary_restart(true).unwrap();
        world.obstacles[0] = Obstacle::new(1, [200.0; 2], [250.0; 2]);
        world.stats.births = 17;
        world
            .replace_dna(animal, LegacyDna::parse("start 99 700 store stop").unwrap())
            .unwrap();
        world.slots.get_mut(animal).unwrap().materials.energy = 0.0;

        world.tick().unwrap();
        assert!(world.slots.get(animal).is_err());
        assert!(world.slots.get(plant).is_ok());
        assert!(!world.snapshot().ordinary_restart_pending);

        world.tick().unwrap();
        assert!(world.snapshot().ordinary_restart_pending);
        assert_eq!(world.snapshot().ordinary_restart_count, 0);
        assert_eq!(world.snapshot().stats.births, 17);
        assert!(world.slots.get(plant).is_ok());
        let tick = world.snapshot().tick;
        let native_cycle = world.snapshot().native_cycle;
        let mut stream = world.native_random.clone().unwrap();
        let expected_seed =
            (f64::from(stream.unit()) * f64::from(i32::MAX)).round_ties_even() as u64;

        let mut restored = SaveFile::decode(&SaveFile::encode(&world).unwrap()).unwrap();
        assert!(restored.snapshot().ordinary_restart_pending);
        for world in [&mut world, &mut restored] {
            assert!(world.finish_ordinary_restart().unwrap());
            assert!(!world.finish_ordinary_restart().unwrap());
            assert!(!world.snapshot().ordinary_restart_pending);
            assert_eq!(world.snapshot().ordinary_restart_count, 1);
            assert_eq!(world.snapshot().tick, tick);
            assert_eq!(world.snapshot().native_cycle, native_cycle);
            assert_eq!(world.snapshot().stats.births, 0);
            assert_eq!(world.config.seed, expected_seed);
            assert_eq!(world.obstacles, [starting_shape.clone()]);
            assert!(world.slots.get(plant).is_err());
            let ids = world.slots.ids_in_slot_order();
            assert_eq!(ids.len(), 2);
            let newborn = world.slots.get(ids[0]).unwrap();
            assert!(newborn.identity.absolute_id > animal_absolute_id);
            assert_eq!(newborn.program.dna.blocks(), animal_dna.blocks());
            assert_eq!(newborn.materials.energy, 763.0);
            assert_eq!(newborn.kinematics.position, [235.0, 705.0]);
            assert!(world.slots.get(ids[1]).unwrap().flags.vegetable);
        }
        assert_eq!(
            SaveFile::encode(&world).unwrap(),
            SaveFile::encode(&restored).unwrap()
        );
    }

    #[test]
    fn empty_world_restarts_at_most_once_per_completed_cycle() {
        let mut world = Engine::new_native(EngineConfig::testing()).unwrap();
        let old_save = SaveFile::encode(&world).unwrap();
        assert!(
            !world
                .db3s_v3_payload()
                .unwrap()
                .as_object()
                .unwrap()
                .contains_key("ordinary_restart")
        );
        world = SaveFile::decode(&old_save).unwrap();
        assert!(!world.snapshot().ordinary_restart_on_extinction);
        world.set_ordinary_restart(true).unwrap();
        world.tick_many(3).unwrap();
        assert_eq!(world.snapshot().tick, 3);
        assert_eq!(world.snapshot().native_cycle, Some(2));
        assert_eq!(world.snapshot().ordinary_restart_count, 2);
        assert!(world.snapshot().ordinary_restart_pending);
        assert!(world.finish_ordinary_restart().unwrap());
        assert_eq!(world.snapshot().ordinary_restart_count, 3);
        world.set_ordinary_restart(false).unwrap();
        world.tick().unwrap();
        assert!(!world.snapshot().ordinary_restart_pending);
    }

    #[test]
    fn native_imports_and_f1_do_not_enable_ordinary_restart() {
        let bytes = include_bytes!("../../tests/fixtures/db24832-original-user.sim");
        let mut imported = Engine::from_native_sim(bytes).unwrap();
        assert!(!imported.snapshot().ordinary_restart_available);
        assert!(imported.set_ordinary_restart(true).is_err());

        let mut f1 = Engine::new_native(EngineConfig::testing()).unwrap();
        f1.native_f1 = Some(true);
        assert!(f1.set_ordinary_restart(true).is_err());
        f1.tick().unwrap();
        assert!(!f1.snapshot().ordinary_restart_pending);
    }
}
