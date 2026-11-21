//! Same-process extinction restart for a world with its original startup roster.
use super::*;
use crate::native_binary::NativeSimulationTail;

#[derive(Deserialize)]
pub(crate) struct NativeRestartBinding {
    pub(crate) lineage_id: u64,
    pub(crate) dna: String,
}

#[derive(Default, Serialize)]
pub(crate) struct NativeRestartSources {
    requested: bool,
    sources: Vec<NativeRestartSource>,
}

#[derive(Serialize)]
struct NativeRestartSource {
    lineage_id: u64,
    filename: String,
    directory: String,
    #[serde(skip_serializing)]
    species: SpeciesId,
    #[serde(skip_serializing)]
    quantity: usize,
    #[serde(skip_serializing)]
    start_energy: i32,
}

#[derive(Clone, Serialize, Deserialize)]
struct BoundRestartSource {
    filename: String,
    dna: LegacyDna,
    #[serde(default, skip_serializing_if = "is_false")]
    repopulation_only: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    start_energy: Option<i32>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct OrdinaryRestart {
    pub(super) enabled: bool,
    pub(super) pending: bool,
    pub(super) count: u64,
    starting_obstacles: Vec<Obstacle>,
    // Imported templates cloned from live genomes cannot authorize startup.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    imported_sources: BTreeMap<u64, BoundRestartSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    imported_preference_override: Option<bool>,
}

impl OrdinaryRestart {
    pub(super) fn is_default(&self) -> bool {
        !self.enabled
            && !self.pending
            && self.count == 0
            && self.starting_obstacles.is_empty()
            && self.imported_sources.is_empty()
            && self.imported_preference_override.is_none()
    }
}

impl Engine {
    fn can_configure_ordinary_restart(&self) -> bool {
        self.native_random.is_some()
            && !self.native_f1_enabled()
            && self.local_mode.config.is_none()
            && self.local_mode.status == LocalModeStatus::Idle
    }

    pub(super) fn ordinary_restart_available(&self) -> bool {
        self.can_configure_ordinary_restart()
            && (self.native_origin.is_none()
                || self.species.iter().enumerate().any(|(index, definition)| {
                    self.ordinary_restart
                        .imported_sources
                        .get(&definition.lineage_id)
                        .is_some_and(|source| {
                            !source.repopulation_only && source.filename == definition.name
                        })
                        && self.active_startup_species(index)
                }))
    }

    fn active_startup_species(&self, index: usize) -> bool {
        !self
            .species_templates
            .get(index)
            .and_then(Option::as_ref)
            .is_some_and(|template| template.retired)
    }

    pub(crate) fn read_native_restart_sources(&self) -> Result<NativeRestartSources, EngineError> {
        if !self.can_configure_ordinary_restart() {
            return Ok(NativeRestartSources::default());
        }
        self.native_species_sources(true)
    }

    fn native_species_sources(
        &self,
        include_repopulation: bool,
    ) -> Result<NativeRestartSources, EngineError> {
        let Some(origin) = self
            .native_origin
            .as_ref()
            .filter(|origin| !origin.tail.is_empty())
        else {
            return Ok(NativeRestartSources::default());
        };
        let tail = NativeSimulationTail::decode(&origin.tail)?;
        let count = tail.integer("species_count")? as u64;
        let requested = self
            .ordinary_restart
            .imported_preference_override
            .unwrap_or(tail.integer("restart")? != 0);
        let mut sources = Vec::new();
        for (index, definition) in self.species.iter().enumerate() {
            if !(1..=count).contains(&definition.lineage_id) || !self.active_startup_species(index)
            {
                continue;
            }
            let prefix = format!("species[{}].", definition.lineage_id - 1);
            let quantity = tail.integer(&format!("{prefix}quantity"))?;
            if quantity <= 0 && !(include_repopulation && definition.vegetable && definition.reseed)
            {
                continue;
            }
            let filename = native_io::text(tail.bytes(&format!("{prefix}name"))?);
            if filename != definition.name {
                continue;
            }
            sources.push(NativeRestartSource {
                lineage_id: definition.lineage_id,
                filename,
                directory: native_io::text(tail.bytes(&format!("{prefix}path"))?),
                species: SpeciesId(index as u32),
                quantity: quantity.max(0) as usize,
                start_energy: tail.integer(&format!("{prefix}start_energy"))?,
            });
        }
        Ok(NativeRestartSources { requested, sources })
    }

    pub(crate) fn bind_native_restart_sources(
        &mut self,
        bindings: Vec<NativeRestartBinding>,
    ) -> Result<usize, EngineError> {
        if self.native_origin.is_none() || !self.can_configure_ordinary_restart() {
            return Err(EngineError::Invariant(
                "No original startup roster can be bound.".into(),
            ));
        }
        let manifest = self.native_species_sources(true)?;
        let by_lineage: BTreeMap<_, _> = manifest
            .sources
            .into_iter()
            .map(|source| (source.lineage_id, source))
            .collect();
        let mut seen = HashSet::new();
        let mut sources = BTreeMap::new();
        for binding in bindings {
            let Some(input) = by_lineage
                .get(&binding.lineage_id)
                .filter(|_| seen.insert(binding.lineage_id))
            else {
                return Err(EngineError::Invariant(
                    "Unknown or duplicate native restart species.".into(),
                ));
            };
            if binding.dna.trim().is_empty() {
                continue;
            }
            if let Ok(dna) = self.load_dna_source(&binding.dna) {
                sources.insert(
                    binding.lineage_id,
                    BoundRestartSource {
                        filename: input.filename.clone(),
                        dna,
                        repopulation_only: input.quantity == 0,
                        start_energy: Some(input.start_energy),
                    },
                );
            }
        }
        let bound = sources.len();
        self.ordinary_restart.imported_sources = sources;
        self.ordinary_restart.enabled = manifest.requested && self.ordinary_restart_available();
        if !self.ordinary_restart.enabled {
            self.ordinary_restart.pending = false;
        }
        self.publish_snapshot();
        Ok(bound)
    }

    pub(super) fn bound_native_repopulation_source(
        &self,
        species: &SpeciesDefinition,
    ) -> Option<(&LegacyDna, Option<i32>)> {
        if self.native_random.is_none() {
            return None;
        }
        self.ordinary_restart
            .imported_sources
            .get(&species.lineage_id)
            .filter(|source| source.filename == species.name)
            .map(|source| (&source.dna, source.start_energy))
    }

    pub(super) fn native_restart_export_override(&self) -> Option<bool> {
        if self.native_origin.is_some() {
            self.ordinary_restart.imported_preference_override
        } else {
            Some(self.ordinary_restart.enabled)
        }
    }

    pub(crate) fn set_ordinary_restart(&mut self, enabled: bool) -> Result<(), EngineError> {
        if enabled && !self.ordinary_restart_available() {
            return Err(EngineError::Invariant(
                "Ordinary extinction restart requires a native world with its startup roster."
                    .into(),
            ));
        }
        if self.native_origin.is_some() {
            self.ordinary_restart.imported_preference_override = Some(enabled);
            self.ordinary_restart.enabled = enabled;
            if !enabled {
                self.ordinary_restart.pending = false;
            }
            self.publish_snapshot();
        } else if self.ordinary_restart.enabled != enabled {
            self.ordinary_restart = if enabled {
                OrdinaryRestart {
                    enabled: true,
                    starting_obstacles: self.obstacles.clone(),
                    ..OrdinaryRestart::default()
                }
            } else {
                OrdinaryRestart::default()
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

        if self.native_origin.is_some() {
            for entry in self.native_species_sources(false)?.sources {
                if self.available_robot_capacity() == 0 {
                    break;
                }
                if let Some(source) = self
                    .ordinary_restart
                    .imported_sources
                    .get(&entry.lineage_id)
                {
                    self.spawn_restart_species(
                        entry.species,
                        &source.dna.clone(),
                        entry.quantity,
                        entry.start_energy,
                    )?;
                    self.species[entry.species.0 as usize].reseed = true;
                } else {
                    // RobScriptLoad calls preparerob before attempting LoadDNA.
                    if let Some(random) = self.native_random.as_mut() {
                        for _ in 0..6 {
                            random.unit();
                        }
                    }
                    self.species[entry.species.0 as usize].reseed = false;
                }
            }
        } else {
            for (index, template) in self.species_templates.clone().into_iter().enumerate() {
                if let Some(template) = template.filter(|template| !template.retired) {
                    self.spawn_restart_species(
                        SpeciesId(index as u32),
                        &template.dna,
                        template.initial_population,
                        template.initial_energy,
                    )?;
                }
            }
        }
        Ok(())
    }

    fn spawn_restart_species(
        &mut self,
        species: SpeciesId,
        dna: &LegacyDna,
        quantity: usize,
        energy: i32,
    ) -> Result<(), EngineError> {
        for _ in 0..quantity.min(self.available_robot_capacity()) {
            let id = self.spawn_native_start_robot(dna.clone(), species, 0, None)?;
            let robot = self.slots.get_mut(id)?;
            robot.materials.energy = energy as f32;
            robot.materials.synchronize_energy();
            robot.program.memory.write(MEM_NRG, energy);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SaveFile;
    use crate::native_binary::{NativeRecordFile, NativeValue};

    const C281: &[u8] = include_bytes!("../../tests/fixtures/db24832-original-user.sim");
    const ALGA: &str = include_str!("../../../../Installer/bots/Alga_Minimalis_Chloroplastus.txt");
    const ANIMAL: &str = include_str!("../../../../Installer/bots/The One (1G)(Shen)-23.04.05.txt");

    fn binding(lineage_id: u64, dna: &str) -> NativeRestartBinding {
        NativeRestartBinding {
            lineage_id,
            dna: dna.into(),
        }
    }

    fn native_restart_flag(save: &[u8]) -> i32 {
        let file = NativeRecordFile::decode(save).unwrap();
        NativeSimulationTail::decode(&file.tail)
            .unwrap()
            .integer("restart")
            .unwrap()
    }

    fn trigger_one_repopulation_birth(world: &mut Engine) -> OrganismId {
        let existing = world.slots.ids_in_slot_order();
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
        added[0]
    }

    #[test]
    fn native_repopulation_uses_bound_file_instead_of_imported_survivor() {
        let mut file = NativeRecordFile::decode(C281).unwrap();
        let survivor = file
            .robots
            .iter_mut()
            .find(|robot| {
                robot.bytes("species_name").unwrap() == b"Alga_Minimalis_Chloroplastus.txt"
            })
            .unwrap();
        let mut dna = survivor.bytes("dna").unwrap().to_vec();
        assert_eq!(&dna[22..24], &160_i16.to_le_bytes());
        dna[22..24].copy_from_slice(&159_i16.to_le_bytes());
        survivor.set_value("dna", NativeValue::Bytes(dna)).unwrap();

        let mut world = Engine::from_native_sim(&file.encode().unwrap()).unwrap();
        let original = world.load_dna_source(ALGA).unwrap();
        let survivor = world
            .slots
            .ids_in_slot_order()
            .into_iter()
            .find(|id| world.slots.get(*id).unwrap().flags.vegetable)
            .unwrap();
        let changed = world.slots.get(survivor).unwrap().program.dna.clone();
        assert_ne!(changed.blocks(), original.blocks());
        assert_eq!(
            world.repopulation_source(0).unwrap().0.blocks(),
            changed.blocks()
        );

        assert_eq!(
            world
                .bind_native_restart_sources(vec![binding(1, ALGA)])
                .unwrap(),
            1
        );
        let mut restored = SaveFile::decode(&SaveFile::encode(&world).unwrap()).unwrap();
        for world in [&mut world, &mut restored] {
            let newborn = trigger_one_repopulation_birth(world);
            let newborn = world.slots.get(newborn).unwrap();
            assert_eq!(newborn.program.dna.blocks(), original.blocks());
            assert_eq!(newborn.materials.energy, 3000.0);
            assert_eq!(
                world.slots.get(survivor).unwrap().program.dna.blocks(),
                changed.blocks()
            );
        }
    }

    #[test]
    fn native_species_comment_edits_keep_the_bound_source_until_dna_is_replaced() {
        const BOUND: &str = "start 19 .up store stop";
        const AUTHORED: &str = "start 23 .up store stop";
        let mut world = Engine::from_native_sim(C281).unwrap();
        assert_eq!(
            world
                .bind_native_restart_sources(vec![binding(1, BOUND)])
                .unwrap(),
            1
        );
        let view = world.configuration_view().unwrap();
        let mut rows: Vec<crate::SpeciesSettings> = view["species"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["dna"].is_string())
            .map(|row| serde_json::from_value(row.clone()).unwrap())
            .collect();
        let alga = rows
            .iter()
            .position(|row| row.species_id == Some(SpeciesId(0)))
            .unwrap();
        let bound = world.load_dna_source(BOUND).unwrap();
        assert_ne!(
            world.load_dna_source(&rows[alga].dna).unwrap().blocks(),
            bound.blocks()
        );

        let living = world.snapshot().organisms.clone();
        rows[alga].comment = "Edited without replacing its DNA".into();
        world.update_species_settings(rows.clone()).unwrap();
        assert_eq!(world.snapshot().organisms, living);
        assert!(
            world.species_templates[0]
                .as_ref()
                .unwrap()
                .source_text
                .is_none()
        );
        assert_eq!(
            world.species_templates[0].as_ref().unwrap().comment,
            rows[alga].comment
        );
        let mut resumed = SaveFile::decode(&SaveFile::encode(&world).unwrap()).unwrap();
        let newborn = trigger_one_repopulation_birth(&mut resumed);
        assert_eq!(resumed.dna(newborn).unwrap().blocks(), bound.blocks());

        rows[alga].dna = AUTHORED.into();
        world.update_species_settings(rows).unwrap();
        assert_eq!(world.snapshot().organisms, living);
        let mut resumed = SaveFile::decode(&SaveFile::encode(&world).unwrap()).unwrap();
        let newborn = trigger_one_repopulation_birth(&mut resumed);
        let authored = resumed.load_dna_source(AUTHORED).unwrap();
        assert_eq!(resumed.dna(newborn).unwrap().blocks(), authored.blocks());
    }

    #[test]
    fn native_repopulation_binds_zero_quantity_without_enabling_restart() {
        let mut file = NativeRecordFile::decode(C281).unwrap();
        file.robots.clear();
        let mut tail = NativeSimulationTail::decode(&file.tail).unwrap();
        for (field, value) in [
            ("species[0].quantity", 0),
            ("species[0].start_energy", 2750),
            ("restart", -1),
        ] {
            tail.set_value(field, NativeValue::Word(value)).unwrap();
        }
        file.tail = tail.encode().unwrap();
        let mut world = Engine::from_native_sim(&file.encode().unwrap()).unwrap();
        assert!(world.species_templates[0].is_none());
        let manifest = world.read_native_restart_sources().unwrap();
        assert!(manifest.requested);
        assert!(
            manifest
                .sources
                .iter()
                .any(|source| source.lineage_id == 1 && source.quantity == 0)
        );
        assert_eq!(
            world
                .bind_native_restart_sources(vec![binding(1, ALGA)])
                .unwrap(),
            1
        );
        assert!(!world.snapshot().ordinary_restart_available);
        assert!(!world.snapshot().ordinary_restart_on_extinction);
        assert_eq!(native_restart_flag(&world.export_native_sim().unwrap()), -1);
        let mut world = SaveFile::decode(&SaveFile::encode(&world).unwrap()).unwrap();
        let newborn = trigger_one_repopulation_birth(&mut world);
        let newborn = world.slots.get(newborn).unwrap();
        assert!(newborn.flags.vegetable);
        assert_eq!(newborn.materials.energy, 2750.0);
        assert_eq!(
            newborn.program.dna.blocks(),
            world.load_dna_source(ALGA).unwrap().blocks()
        );
        assert!(!world.snapshot().ordinary_restart_available);
    }

    #[test]
    fn original_c281_binds_restart_files_without_changing_live_genomes() {
        let mut world = Engine::from_native_sim(C281).unwrap();
        let manifest = world.read_native_restart_sources().unwrap();
        assert!(!manifest.requested);
        assert_eq!(
            manifest
                .sources
                .iter()
                .map(|row| (
                    row.lineage_id,
                    row.filename.as_str(),
                    row.quantity,
                    row.start_energy
                ))
                .collect::<Vec<_>>(),
            [
                (1, "Alga_Minimalis_Chloroplastus.txt", 5, 3000),
                (2, "The One (1G)(Shen)-23.04.05.txt", 5, 3000)
            ]
        );
        assert!(
            manifest
                .sources
                .iter()
                .all(|row| row.directory.ends_with(r"\oracle-run\Robots"))
        );

        let animals = world
            .slots
            .ids_in_slot_order()
            .into_iter()
            .filter(|id| !world.slots.get(*id).unwrap().flags.vegetable)
            .collect::<Vec<_>>();
        assert_eq!(world.population(), 196);
        let edited = world.load_dna_source("start 41 700 store stop").unwrap();
        world.replace_dna(animals[0], edited.clone()).unwrap();
        let old_snapshot = world.snapshot().organisms.clone();
        let old_rng = world.native_random.as_ref().unwrap().state();
        assert_eq!(
            world
                .bind_native_restart_sources(vec![binding(2, ANIMAL), binding(1, ALGA)])
                .unwrap(),
            2
        );
        assert!(world.snapshot().ordinary_restart_available);
        assert!(!world.snapshot().ordinary_restart_on_extinction);
        assert_eq!(world.snapshot().organisms, old_snapshot);
        assert_eq!(
            world.slots.get(animals[0]).unwrap().program.dna.blocks(),
            edited.blocks()
        );
        assert_eq!(world.native_random.as_ref().unwrap().state(), old_rng);
        assert_eq!(native_restart_flag(&world.export_native_sim().unwrap()), 0);

        world.set_ordinary_restart(true).unwrap();
        assert_eq!(native_restart_flag(&world.export_native_sim().unwrap()), -1);
        world.remove_many(&animals).unwrap();
        world.tick().unwrap();
        assert_eq!(world.snapshot().native_cycle, Some(282));
        assert!(world.snapshot().ordinary_restart_pending);
        let mut restored = SaveFile::decode(&SaveFile::encode(&world).unwrap()).unwrap();
        let expected_dna = [
            world.load_dna_source(ALGA).unwrap(),
            world.load_dna_source(ANIMAL).unwrap(),
        ];
        for world in [&mut world, &mut restored] {
            assert!(world.finish_ordinary_restart().unwrap());
            assert_eq!(world.snapshot().ordinary_restart_count, 1);
            assert_eq!(world.snapshot().native_cycle, Some(282));
            assert_eq!(world.population(), 10);
            for (index, id) in world.slots.ids_in_slot_order().into_iter().enumerate() {
                let robot = world.slots.get(id).unwrap();
                assert!(robot.identity.absolute_id > 294);
                assert_eq!(robot.materials.energy, 3000.0);
                assert_eq!(robot.materials.body, 1000.0);
                assert_eq!(robot.flags.vegetable, index < 5);
                assert_eq!(robot.program.dna.blocks(), expected_dna[index / 5].blocks());
            }
        }
        assert_eq!(
            SaveFile::encode(&world).unwrap(),
            SaveFile::encode(&restored).unwrap()
        );
        world.set_ordinary_restart(false).unwrap();
        assert!(world.snapshot().ordinary_restart_available);
        assert_eq!(native_restart_flag(&world.export_native_sim().unwrap()), 0);
    }

    #[test]
    fn original_requested_restart_skips_missing_sources_independently() {
        let mut file = NativeRecordFile::decode(C281).unwrap();
        let mut tail = NativeSimulationTail::decode(&file.tail).unwrap();
        tail.set_value("restart", NativeValue::Word(-1)).unwrap();
        file.tail = tail.encode().unwrap();
        let save = file.encode().unwrap();
        let mut world = Engine::from_native_sim(&save).unwrap();
        assert!(world.read_native_restart_sources().unwrap().requested);
        assert_eq!(
            world
                .bind_native_restart_sources(vec![binding(1, "def")])
                .unwrap(),
            0
        );
        assert!(!world.snapshot().ordinary_restart_available);
        assert!(!world.snapshot().ordinary_restart_on_extinction);
        assert!(world.set_ordinary_restart(true).is_err());
        assert_eq!(native_restart_flag(&world.export_native_sim().unwrap()), -1);

        assert_eq!(
            world
                .bind_native_restart_sources(vec![binding(2, ANIMAL)])
                .unwrap(),
            1
        );
        assert!(world.snapshot().ordinary_restart_on_extinction);
        world.remove_many(&world.slots.ids_in_slot_order()).unwrap();
        world.tick().unwrap();
        assert!(world.snapshot().ordinary_restart_pending);
        assert!(world.finish_ordinary_restart().unwrap());
        assert_eq!(world.population(), 5);
        assert!(
            world
                .snapshot()
                .organisms
                .iter()
                .all(|robot| !robot.vegetable)
        );
    }

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
