//! Retain the complete robot at death and materialize it for native exchange.

use super::*;
#[cfg(test)]
use crate::SaveFile;

pub(super) struct NativeCorpseTransfer {
    pub bytes: Vec<u8>,
    pub living: Vec<OrganismId>,
    pub archived: Vec<usize>,
}

impl Engine {
    pub(super) fn decay_native_corpses(&mut self) {
        for id in self.slots.ids_in_slot_order() {
            if !self
                .slots
                .get(id)
                .is_ok_and(|robot| robot.flags.corpse && !robot.flags.hidden)
            {
                continue;
            }
            let virtual_body = self.virtual_body_for(id);
            let robot = self.slots.get_mut(id).unwrap();
            robot.counters.decay_timer = robot.counters.decay_timer.saturating_add(1);
            if u64::from(robot.counters.decay_timer) < self.config.corpse_decay.period {
                continue;
            }
            robot.counters.decay_timer = 0;
            let unit =
                world_random_unit(&mut self.native_random, &mut robot.program.rng_state) as f32;
            robot.set_heading(unit * 2.0 * std::f32::consts::PI);
            let body = robot.materials.body.max(0.0);
            let packet = self.config.corpse_decay.packet_size.max(0) as f32;
            let value = if f64::from(body) > f64::from(packet) / 10.0 {
                packet
            } else {
                body
            };
            let kind = match self.config.corpse_decay.mode {
                crate::CorpseDecayMode::None => None,
                crate::CorpseDecayMode::WasteShot => Some(-4),
                crate::CorpseDecayMode::EnergyShot => Some(-2),
            };
            if let Some(kind) = kind.filter(|_| value != 0.0) {
                let mut angle = robot.heading();
                if robot.program.memory.read(900) != 0 {
                    angle = (angle - std::f32::consts::PI).rem_euclid(std::f32::consts::TAU);
                    robot.program.memory.write(900, 0);
                }
                let aimed = robot.program.memory.read(901);
                if aimed != 0 {
                    angle = (f64::from(robot.heading()) - f64::from(aimed % 1256) / 200.0) as f32;
                    robot.program.memory.write(901, 0);
                }
                // newshot consumes the unused range draw before angle scatter.
                world_random_unit(&mut self.native_random, &mut robot.program.rng_state);
                let scatter = world_random_integer(
                    &mut self.native_random,
                    &mut robot.program.rng_state,
                    -20,
                    20,
                );
                angle = (f64::from(angle) + f64::from(scatter) / 200.0) as f32;
                let slot = self.projectiles.spawn(
                    ProjectileSpawn {
                        owner: id,
                        owner_position: robot.kinematics.position,
                        owner_velocity: robot.kinematics.velocity,
                        owner_actual_velocity: robot.kinematics.actual_velocity,
                        correct_position: false,
                        owner_radius: robot.materials.radius,
                        owner_virtual_body: virtual_body,
                        range_multiplier: 1.0,
                        angle,
                        kind,
                        value: value.floor(),
                        payload: None,
                        source_species_name: Some("Corpse".into()),
                        toxin_memory_location: robot.program.memory.read(835),
                        toxin_memory_value: robot.program.memory.read(836),
                        color: robot.phenotype.color,
                        source_vegetable: false,
                        stored: false,
                        gene_number: 0,
                        dna_length: 0,
                    },
                    &self.config.shots,
                );
                // Decay calls newshot directly; it does not replace nrg with
                // the explicit donation amount used by ManageShoot.
                self.projectiles.energies[slot as usize] =
                    self.projectiles.ranges[slot as usize] * 40.0;
                self.stats.shots_fired = self.stats.shots_fired.saturating_add(1);
            }
            robot.materials.body = (f64::from(body) - f64::from(packet) / 10.0) as f32;
            robot.materials.radius = crate::physics::organism_radius(
                robot.materials.body,
                0.0,
                self.config.fixed_bot_radii,
            );
        }
    }

    pub(super) fn become_native_corpse(&mut self, id: OrganismId) -> Result<(), EngineError> {
        let robot = self.slots.get_mut(id)?;
        robot.flags.corpse = true;
        robot.flags.dead = false;
        robot.flags.vegetable = false;
        robot.flags.fixed = false;
        robot.kinematics.native_fixed_state = Some(false);
        robot.flags.can_execute_dna = false;
        robot.flags.can_move = false;
        robot.flags.can_see = false;
        robot.flags.virus_immune = true;
        robot.flags.can_receive_viruses = false;
        robot.identity.species_file = "Corpse".into();
        robot.phenotype.color = 0xffff_ffff;
        robot.materials.energy = 0.0;
        robot.materials.chloroplasts = 0.0;
        robot.buoyancy = 0.0;
        robot.senses.occurrence_counts.fill(0);
        robot.senses.vision_reference_occurrences.fill(0);
        for address in 501..=509 {
            robot.program.memory.write(address, 0);
        }
        robot.publish_occurrence_readbacks();
        Ok(())
    }

    pub(super) fn corpse_views(&self) -> Vec<CorpseSnapshot> {
        let mut views = self
            .corpses
            .iter()
            .map(CorpseSnapshot::view)
            .collect::<Vec<_>>();
        for robot in self
            .slots
            .iter()
            .filter_map(|slot| slot.organism.as_ref())
            .filter(|robot| robot.flags.corpse)
        {
            let mut view =
                CorpseSnapshot::projection_from_robot(robot, robot.identity.species.0, false);
            view.energy = resource_readback(robot.materials.energy);
            view.decay_timer = u64::from(robot.counters.decay_timer);
            views.push(view);
        }
        views
    }

    #[cfg(test)]
    pub(super) fn archive_corpse(
        &self,
        id: OrganismId,
        robot: &RobotState,
        species: u32,
        repopulating: bool,
    ) -> CorpseSnapshot {
        let mut corpse = CorpseSnapshot::from_robot(robot, species, repopulating);
        corpse.native_id = Some(id);
        corpse.native_ties = self
            .ties
            .iter()
            .filter(|tie| tie.contains(id))
            .cloned()
            .collect();
        corpse
    }

    pub(super) fn native_world_with_corpses(&self) -> Result<(Self, Vec<OrganismId>), EngineError> {
        self.native_world_with_selected_corpses(&(0..self.corpses.len()).collect::<Vec<_>>())
    }

    fn native_world_with_selected_corpses(
        &self,
        indices: &[usize],
    ) -> Result<(Self, Vec<OrganismId>), EngineError> {
        if self.occupied_robot_records() + indices.len() > MAX_ROBOTS {
            return Err(EngineError::InvalidSave(
                "native world exceeds 32000 records".into(),
            ));
        }
        // Clear archives before restore so automatic migration cannot duplicate
        // the selected corpse records in this export-only copy.
        let mut payload = self.db3s_v3_payload()?;
        payload["corpses"] = serde_json::json!([]);
        let mut world = Self::from_db3s_v3_payload(payload)?;
        let corpses = indices
            .iter()
            .map(|&index| self.corpses[index].clone())
            .collect::<Vec<_>>();
        let members = world.materialize_archived_corpses(&corpses)?;
        Ok((world, members))
    }

    pub(super) fn migrate_complete_archived_corpses(&mut self) -> Result<(), EngineError> {
        let archives = std::mem::take(&mut self.corpses);
        let (complete, partial): (Vec<_>, Vec<_>) = archives
            .into_iter()
            .partition(|corpse| corpse.native_state.is_some());
        self.corpses = partial;
        if !complete.is_empty() {
            self.materialize_archived_corpses(&complete)?;
            self.refresh_tie_projections();
        }
        Ok(())
    }

    fn materialize_archived_corpses(
        &mut self,
        corpses: &[CorpseSnapshot],
    ) -> Result<Vec<OrganismId>, EngineError> {
        if self.occupied_robot_records() + corpses.len() > MAX_ROBOTS {
            return Err(EngineError::InvalidSave(
                "migrated world exceeds 32000 robot records".into(),
            ));
        }
        let mut prior_ids = HashSet::new();
        for corpse in corpses {
            if let Some(id) = corpse.native_id
                && (!prior_ids.insert(id) || self.slots.get(id).is_ok())
            {
                return Err(EngineError::InvalidSave(
                    "archived corpse identity duplicates a retained robot record".into(),
                ));
            }
        }
        let mut remap = HashMap::new();
        let mut members = Vec::with_capacity(corpses.len());
        for corpse in corpses {
            let mut robot = corpse.native_state.as_deref().cloned().ok_or_else(|| {
                EngineError::InvalidSave("older corpse projection lacks the complete robot state required for native export".into())
            })?;
            // Robots.ManageDeath leaves DNA, memory, materials and ties intact.
            // These are the fields it changes when the living robot becomes a corpse.
            robot.flags.corpse = true;
            robot.flags.dead = false;
            robot.flags.exists = true;
            robot.flags.vegetable = false;
            robot.flags.fixed = false;
            robot.kinematics.native_fixed_state = Some(false);
            robot.flags.can_execute_dna = false;
            robot.flags.can_move = false;
            robot.flags.can_see = false;
            robot.flags.virus_immune = true;
            robot.flags.can_receive_viruses = false;
            robot.identity.species_file = "Corpse".into();
            robot.phenotype.color = 0xffff_ffff;
            robot.buoyancy = 0.0;
            robot.materials.chloroplasts = 0.0;
            robot.materials.energy = corpse.energy as f32;
            let body = corpse.body as f32 + corpse.body_fraction;
            if body != robot.materials.body {
                // The old archive decay path updated its projection, while
                // retaining the original complete robot as exchange metadata.
                robot.materials.radius =
                    crate::physics::organism_radius(body, 0.0, self.config.fixed_bot_radii);
            }
            robot.materials.body = body;
            robot.kinematics.position = corpse.position;
            robot.kinematics.previous_position = corpse.position;
            robot.kinematics.velocity = corpse.velocity;
            robot.counters.age = corpse.age;
            robot.counters.decay_timer = corpse.decay_timer.min(u32::MAX as u64) as u32;
            robot.senses.occurrence_counts.fill(0);
            robot.senses.vision_reference_occurrences.fill(0);
            for address in 501..=509 {
                robot.program.memory.write(address, 0);
            }
            let id = self.slots.spawn(robot);
            if let Some(old_id) = corpse.native_id {
                remap.insert(old_id, id);
            }
            members.push(id);
        }
        for slot in self.slots.iter_mut() {
            if let Some(robot) = slot.organism.as_mut() {
                for parent in robot.identity.parents.iter_mut().flatten() {
                    *parent = remap.get(parent).copied().unwrap_or(*parent);
                }
                for opponent in [
                    &mut robot.senses.focused_opponent,
                    &mut robot.senses.console_reference,
                ]
                .into_iter()
                .flatten()
                {
                    *opponent = remap.get(opponent).copied().unwrap_or(*opponent);
                }
                for material in [
                    &mut robot.effects.stored_sperm,
                    &mut robot.effects.stored_virus,
                ]
                .into_iter()
                .flatten()
                {
                    if let Some(source) = material.source.as_mut() {
                        *source = remap.get(source).copied().unwrap_or(*source);
                    }
                }
                for endpoint in robot.ties.iter_mut().flatten() {
                    if let Some(other) = endpoint.other.and_then(|id| remap.get(&id).copied()) {
                        endpoint.other = Some(other);
                    }
                }
            }
        }
        for owner in &mut self.projectiles.owners {
            *owner = remap.get(owner).copied().unwrap_or(*owner);
        }
        for id in &mut self.pending_mutations {
            *id = remap.get(id).copied().unwrap_or(*id);
        }
        if let Some(focus) = self.player_bot.focus.as_mut() {
            *focus = remap.get(focus).copied().unwrap_or(*focus);
        }
        for tie in &mut self.ties {
            tie.first = remap.get(&tie.first).copied().unwrap_or(tie.first);
            tie.second = remap.get(&tie.second).copied().unwrap_or(tie.second);
        }
        for corpse in &mut self.corpses {
            if let Some(parent) = corpse.parent.as_mut() {
                *parent = remap.get(parent).copied().unwrap_or(*parent);
            }
        }
        self.ties
            .retain(|tie| self.slots.get(tie.first).is_ok() && self.slots.get(tie.second).is_ok());
        for tie in corpses.iter().flat_map(|corpse| &corpse.native_ties) {
            let mut tie = tie.clone();
            tie.first = remap.get(&tie.first).copied().unwrap_or(tie.first);
            tie.second = remap.get(&tie.second).copied().unwrap_or(tie.second);
            if self.slots.get(tie.first).is_ok()
                && self.slots.get(tie.second).is_ok()
                && !self
                    .ties
                    .iter()
                    .any(|existing| existing.contains(tie.first) && existing.contains(tie.second))
            {
                self.ties.push(tie);
            }
        }
        self.rebuild_native_tie_index();
        Ok(members)
    }

    pub(super) fn native_corpse_transfer(
        &self,
        index: usize,
    ) -> Result<NativeCorpseTransfer, EngineError> {
        let (mut world, corpse_members) = self.native_world_with_corpses()?;
        world.refresh_tie_projections();
        let group = world.file_transfer_group(corpse_members[index])?;
        let mut living = Vec::new();
        let mut archived = Vec::new();
        for &id in &group {
            if let Some(index) = corpse_members.iter().position(|&corpse_id| corpse_id == id) {
                archived.push(index);
            } else {
                self.valid_slot(id)?;
                living.push(id);
            }
        }
        Ok(NativeCorpseTransfer {
            bytes: world.export_native_organisms(&group)?,
            living,
            archived,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_binary::NativeRecordFile;

    #[test]
    fn native_corpse_decay_shot_retains_owner_radius_range_and_newshot_energy() {
        let mut world = Engine::new(EngineConfig {
            fixed_bot_radii: true,
            metabolism_cost: 0,
            corpse_decay: CorpseDecaySettings {
                enabled: true,
                mode: crate::CorpseDecayMode::EnergyShot,
                packet_size: 3,
                period: 1,
                ..Default::default()
            },
            ..EngineConfig::testing()
        })
        .unwrap();
        let id = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [500.0; 2])
            .unwrap();
        world.set_organism_energy(id, 14).unwrap();
        world.tick().unwrap();
        world.tick().unwrap();
        let slot = world.projectiles.live_slots()[0] as usize;
        assert_eq!(world.projectiles.owners[slot], id);
        assert_eq!(world.projectiles.values[slot], 3);
        assert_eq!(world.projectiles.ranges[slot], 11.0);
        assert_eq!(world.projectiles.energies[slot], 440.0);
        let robot = world.slots.get(id).unwrap();
        let shot = world.projectiles.positions[slot];
        let position = robot.kinematics.position;
        assert!(((shot[0] - position[0]).hypot(shot[1] - position[1]) - 60.0).abs() < 0.0001);
        assert_eq!(robot.materials.body, 999.7);
        world.tick().unwrap();
        assert!(world.projectiles.alive[slot]);
        assert!(
            !world.projectiles.impact_flash[slot],
            "a decay shot must not hit its own corpse"
        );
    }

    #[test]
    fn natural_corpse_retains_its_robot_identity_memory_dna_and_physical_tie() {
        let mut world = Engine::new(EngineConfig {
            fixed_bot_radii: true,
            metabolism_cost: 0,
            corpse_decay: CorpseDecaySettings {
                enabled: true,
                ..Default::default()
            },
            ..EngineConfig::testing()
        })
        .unwrap();
        let plant = world.register_species(SpeciesDefinition {
            vegetable: true,
            minimum_population: 1,
            reseed: false,
            ..Default::default()
        });
        let dna = LegacyDna::parse("start 7 999 store stop").unwrap();
        let dead = world
            .spawn_species_at(dna.clone(), plant, [400.0, 500.0])
            .unwrap();
        let living = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [600.0, 500.0])
            .unwrap();
        let absolute = world.slots.get(dead).unwrap().identity.absolute_id;
        world.set_organism_energy(dead, 14).unwrap();
        world.ties.push(TieState::legacy(dead, living, 200.0, 1, 1));
        world.tick().unwrap();
        let corpse = world.slots.get(dead).unwrap();
        assert!(corpse.flags.corpse);
        assert!(
            !corpse.flags.fixed
                && !corpse.flags.can_execute_dna
                && !corpse.flags.can_move
                && !corpse.flags.can_see
        );
        assert_eq!(corpse.identity.absolute_id, absolute);
        assert_eq!(corpse.identity.species_file, "Corpse");
        assert_eq!(corpse.program.memory.read(999), 7);
        assert_eq!(corpse.program.dna.blocks(), dna.blocks());
        assert!(
            corpse
                .senses
                .occurrence_counts
                .iter()
                .all(|&value| value == 0)
        );
        assert!(
            world
                .ties
                .iter()
                .any(|tie| tie.contains(dead) && tie.contains(living))
        );
        assert_eq!(world.population(), 1);
        assert_eq!(world.snapshot().stats.population, 1);
        assert_eq!(world.snapshot().organisms.len(), 2);
        assert!(
            !world
                .snapshot()
                .organisms
                .iter()
                .find(|r| r.id == dead)
                .unwrap()
                .vegetable
        );
        assert_eq!(world.vegetable_population(), 0);
        assert_eq!(world.snapshot().corpses.len(), 1);
        assert!(
            world.corpses.is_empty(),
            "natural deaths must not move into the legacy archive"
        );
        let resumed = SaveFile::decode(&SaveFile::encode(&world).unwrap()).unwrap();
        assert!(resumed.slots.get(dead).unwrap().flags.corpse);
        assert!(
            resumed
                .ties
                .iter()
                .any(|tie| tie.contains(dead) && tie.contains(living))
        );
        let native = NativeRecordFile::decode(&resumed.export_native_sim().unwrap()).unwrap();
        let record = native
            .robots
            .iter()
            .find(|r| r.long("absolute_id").unwrap() as u64 == absolute)
            .unwrap();
        assert_eq!(record.word("corpse").unwrap(), -1);
        assert_eq!(record.word("vegetable").unwrap(), 0);
        assert_eq!(record.single("energy").unwrap(), 0.0);
        assert_eq!(record.word("fixed").unwrap(), 0);
    }

    #[test]
    fn natural_corpse_keeps_momentum_and_collides_with_living_robots() {
        let mut config = EngineConfig {
            fixed_bot_radii: true,
            drag: 0.0,
            metabolism_cost: 0,
            corpse_decay: CorpseDecaySettings {
                enabled: true,
                ..Default::default()
            },
            ..EngineConfig::testing()
        };
        config.physics.density = 0.0;
        config.physics.surface_gravity = 0.0;
        config.physics.static_friction = 0.0;
        config.physics.kinetic_friction = 0.0;
        let mut world = Engine::new(config).unwrap();
        let id = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [500.0; 2])
            .unwrap();
        world.set_organism_energy(id, 14).unwrap();
        world.slots.get_mut(id).unwrap().kinematics.velocity = [10.0, 0.0];
        world.tick().unwrap();
        let first_position = world.slots.get(id).unwrap().kinematics.position;
        world.tick().unwrap();
        let second_position = world.slots.get(id).unwrap().kinematics.position;
        assert_eq!(
            second_position,
            [first_position[0] + 10.0, first_position[1]]
        );
        let living = world
            .spawn_at(
                LegacyDna::parse("stop").unwrap(),
                [second_position[0] + 50.0, second_position[1]],
            )
            .unwrap();
        world.tick().unwrap();
        let dead = world.slots.get(id).unwrap().kinematics.position;
        let living = world.slots.get(living).unwrap().kinematics.position;
        let distance = (dead[0] - living[0]).hypot(dead[1] - living[1]);
        assert!(
            distance > 50.0 && distance < 120.0,
            "Repel3 separates moving robots gradually: {distance}"
        );
        assert_eq!(world.stats.deaths, 1);
    }

    #[test]
    fn corpse_occupies_capacity_until_exhausted_without_counting_a_second_death() {
        let mut world = Engine::new(EngineConfig {
            organism_capacity: 1,
            metabolism_cost: 0,
            corpse_decay: CorpseDecaySettings {
                enabled: true,
                packet_size: 2000,
                period: 1,
                ..Default::default()
            },
            ..EngineConfig::testing()
        })
        .unwrap();
        let id = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [500.0; 2])
            .unwrap();
        world.set_organism_energy(id, 14).unwrap();
        world.slots.get_mut(id).unwrap().materials.body = 100.0;
        world.tick().unwrap();
        assert_eq!(world.population(), 0);
        assert!(matches!(
            world.spawn_at(LegacyDna::parse("stop").unwrap(), [500.0; 2]),
            Err(EngineError::CapacityReached)
        ));
        world.tick().unwrap();
        assert!(world.slots.get(id).is_err());
        assert!(world.snapshot().corpses.is_empty());
        assert_eq!(world.stats.deaths, 1);
        let replacement = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [500.0; 2])
            .unwrap();
        assert_eq!(id.slot(), replacement.slot());
        assert_ne!(id.generation(), replacement.generation());
    }

    #[test]
    fn archived_native_corpse_keeps_memory_materials_and_tie_through_modern_save_and_native_export()
    {
        let mut world = Engine::from_native_sim(include_bytes!(
            "../../tests/fixtures/db24832-original-user.sim"
        ))
        .unwrap();
        let tie = world.ties[0].clone();
        let original = world.slots.get(tie.first).unwrap().clone();
        let other_absolute = world.slots.get(tie.second).unwrap().identity.absolute_id;
        let mut corpse =
            world.archive_corpse(tie.first, &original, original.identity.species.0, false);
        corpse.body = 153;
        corpse.body_fraction = 0.375;
        corpse.position = [500.0, 600.0];
        corpse.age += 7;
        let other = world.slots.get_mut(tie.second).unwrap();
        other.identity.parents[0] = Some(tie.first);
        other.senses.focused_opponent = Some(tie.first);
        other.effects.stored_sperm = Some(StoredGeneticMaterial {
            dna: original.program.dna.clone(),
            source: Some(tie.first),
        });
        world.projectiles.owners[0] = tie.first;
        world.player_bot.focus = Some(tie.first);
        world.slots.remove(tie.first).unwrap();
        let dead_id = tie.first;
        world.ties.retain(|tie| !tie.contains(dead_id));
        world.corpses.push(corpse);
        let replacement = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [3500.0; 2])
            .unwrap();
        assert_eq!(replacement.slot(), dead_id.slot());
        assert_ne!(replacement.generation(), dead_id.generation());
        world.publish_snapshot();
        assert!(world.snapshot.corpses[0].native_state.is_none());
        let modern = SaveFile::encode(&world).unwrap();
        let resumed = SaveFile::decode(&modern).unwrap();
        assert!(resumed.corpses.is_empty());
        let migrated = resumed
            .slots
            .ids_in_slot_order()
            .into_iter()
            .find(|&id| {
                resumed.slots.get(id).unwrap().identity.absolute_id == original.identity.absolute_id
            })
            .unwrap();
        assert!(resumed.slots.get(migrated).unwrap().flags.corpse);
        assert_ne!(migrated, dead_id);
        assert!(!resumed.slots.get(replacement).unwrap().flags.corpse);
        assert_eq!(resumed.projectiles.owners[0], migrated);
        assert_eq!(resumed.player_bot.focus, Some(migrated));
        let other = resumed.slots.get(tie.second).unwrap();
        assert_eq!(other.identity.parents[0], Some(migrated));
        assert_eq!(other.senses.focused_opponent, Some(migrated));
        assert_eq!(
            other.effects.stored_sperm.as_ref().unwrap().source,
            Some(migrated)
        );
        assert_eq!(resumed.slots.get(migrated).unwrap().materials.body, 153.375);
        assert!(resumed.ties.iter().any(|tie| tie.contains(migrated)));
        let twice = SaveFile::decode(&SaveFile::encode(&resumed).unwrap()).unwrap();
        assert!(twice.corpses.is_empty());
        assert_eq!(
            twice.occupied_robot_records(),
            resumed.occupied_robot_records()
        );
        let before = SaveFile::encode(&resumed).unwrap();
        let bytes = resumed.export_native_sim().unwrap();
        assert_eq!(
            before,
            SaveFile::encode(&resumed).unwrap(),
            "export is read-only"
        );
        let file = NativeRecordFile::decode(&bytes).unwrap();
        let corpse = file
            .robots
            .iter()
            .find(|r| r.long("absolute_id").unwrap() as u64 == original.identity.absolute_id)
            .unwrap();
        assert_eq!(corpse.word("corpse").unwrap(), -1);
        assert_eq!(corpse.single("energy").unwrap(), 0.0);
        assert_eq!(corpse.single("body").unwrap(), 153.375);
        assert_eq!(
            corpse.single("shell").unwrap().to_bits(),
            original.materials.shell.to_bits()
        );
        assert_eq!(
            corpse.single("waste").unwrap().to_bits(),
            original.materials.waste.to_bits()
        );
        assert_eq!(
            corpse.long("age").unwrap() as u64,
            original.counters.age + 7
        );
        let reloaded = Engine::from_native_sim(&bytes).unwrap();
        let dead = reloaded
            .slots
            .ids_in_slot_order()
            .into_iter()
            .find(|&id| reloaded.slots.get(id).unwrap().flags.corpse)
            .unwrap();
        let robot = reloaded.slots.get(dead).unwrap();
        assert_eq!(robot.program.dna.blocks(), original.program.dna.blocks());
        assert_eq!(
            robot.program.memory.read(1000),
            original.program.memory.read(1000)
        );
        assert!(reloaded.ties.iter().any(|tie| {
            tie.contains(dead)
                && reloaded
                    .slots
                    .get(if tie.first == dead {
                        tie.second
                    } else {
                        tie.first
                    })
                    .unwrap()
                    .identity
                    .absolute_id
                    == other_absolute
        }));
        let group =
            NativeRecordFile::decode(&resumed.export_native_organisms(&[migrated]).unwrap())
                .unwrap();
        assert_eq!(group.robots.len(), 1);
        assert!(group.tail.is_empty());
        assert_eq!(group.robots[0].word("corpse").unwrap(), -1);
    }

    #[test]
    fn returned_native_corpse_decays_without_becoming_another_archived_corpse() {
        let mut world = Engine::new(EngineConfig {
            brownian_motion: 0.0,
            corpse_decay: CorpseDecaySettings {
                enabled: true,
                packet_size: 3,
                period: 2,
                ..Default::default()
            },
            ..EngineConfig::testing()
        })
        .unwrap();
        let id = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [500.0; 2])
            .unwrap();
        let original = world.slots.get(id).unwrap().clone();
        let corpse = world.archive_corpse(id, &original, 0, false);
        world.remove(id).unwrap();
        world.corpses.push(corpse);
        let mut returned = Engine::from_native_sim(&world.export_native_sim().unwrap()).unwrap();
        let id = returned.slots.ids_in_slot_order()[0];
        let body = returned.slots.get(id).unwrap().materials.body;
        let age = returned.slots.get(id).unwrap().counters.age;
        // Latched commands from before death must not run corpse metabolism,
        // manufacture materials, shoot or modify its saved genome.
        let robot = returned.slots.get_mut(id).unwrap();
        for (address, value) in [
            (313, 100),
            (822, 100),
            (824, 100),
            (7, -4),
            (335, 1),
            (340, 1),
        ] {
            robot.program.memory.write(address, value);
        }
        let dna = robot.program.dna.blocks().to_vec();
        let shell = robot.materials.shell;
        let venom = robot.materials.venom;
        returned.tick().unwrap();
        assert_eq!(returned.slots.get(id).unwrap().materials.body, body);
        returned.tick().unwrap();
        assert_eq!(returned.slots.get(id).unwrap().materials.body, body - 0.3);
        assert!(returned.corpses.is_empty());
        assert!(returned.slots.get(id).unwrap().flags.corpse);
        assert_eq!(returned.slots.get(id).unwrap().counters.age, age);
        let corpse = returned.slots.get(id).unwrap();
        assert_eq!(corpse.materials.energy, 0.0);
        assert_eq!(corpse.materials.shell, shell);
        assert_eq!(corpse.materials.venom, venom);
        assert_eq!(corpse.program.dna.blocks(), dna);
        assert!(returned.projectiles.snapshots().is_empty());
    }

    #[test]
    fn newborn_native_parent_identity_survives_removal_and_slot_reuse() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let parent = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [500.0; 2])
            .unwrap();
        let parent_absolute = world.slots.get(parent).unwrap().identity.absolute_id;
        assert_ne!(
            parent_absolute, 0,
            "fresh parents require a native identity"
        );
        let child = world.clone_organism(parent, [700.0; 2]).unwrap();
        assert_eq!(parent_absolute, 1);
        assert_eq!(world.slots.get(child).unwrap().identity.absolute_id, 2);
        world.remove(parent).unwrap();
        let replacement = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [1000.0; 2])
            .unwrap();
        assert_eq!(replacement.slot(), parent.slot());
        assert_eq!(
            world.slots.get(replacement).unwrap().identity.absolute_id,
            3
        );
        let resumed = SaveFile::decode(&SaveFile::encode(&world).unwrap()).unwrap();
        assert_eq!(resumed.next_absolute_id, 4);
        let file =
            NativeRecordFile::decode(&resumed.export_native_organisms(&[child]).unwrap()).unwrap();
        assert_eq!(
            file.robots[0].long("parent").unwrap() as u64,
            parent_absolute
        );
    }

    #[test]
    fn complete_archive_loads_once_and_moves_as_a_physical_corpse() {
        let mut world = Engine::new(EngineConfig {
            brownian_motion: 0.0,
            corpse_decay: CorpseDecaySettings {
                enabled: true,
                period: 100,
                ..Default::default()
            },
            ..EngineConfig::testing()
        })
        .unwrap();
        let id = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [500.0; 2])
            .unwrap();
        let robot = world.slots.get_mut(id).unwrap();
        robot.kinematics.velocity = [10.0, 0.0];
        robot.materials.radius = 415.0;
        robot.program.memory.restore_native_cell(999, 32767);
        let corpse = world.archive_corpse(id, world.slots.get(id).unwrap(), 0, false);
        world.slots.remove(id).unwrap();
        world.corpses.push(corpse);
        let mut resumed = SaveFile::decode(&SaveFile::encode(&world).unwrap()).unwrap();
        assert!(resumed.corpses.is_empty());
        assert_eq!(resumed.population(), 0);
        assert_eq!(resumed.occupied_robot_records(), 1);
        let dead = resumed.slots.ids_in_slot_order()[0];
        assert_eq!(resumed.memory_at(dead, 999).unwrap(), 32767);
        assert_eq!(resumed.slots.get(dead).unwrap().materials.radius, 415.0);
        resumed.tick().unwrap();
        assert!(resumed.slots.get(dead).unwrap().kinematics.position[0] > 500.0);
        assert_eq!(
            resumed.stats.deaths, 0,
            "migration does not record a new death"
        );
        assert_eq!(resumed.snapshot().render_instances.len(), 1);
    }

    #[test]
    fn archive_identity_collision_is_rejected_and_partial_archives_are_retained() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let id = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [500.0; 2])
            .unwrap();
        world
            .corpses
            .push(world.archive_corpse(id, world.slots.get(id).unwrap(), 0, false));
        assert!(matches!(
            SaveFile::decode(&SaveFile::encode(&world).unwrap()),
            Err(EngineError::InvalidSave(_))
        ));
        world.corpses.clear();
        world.corpses.push(CorpseSnapshot {
            body: 100,
            dna: "1 2 3 stop".into(),
            ..Default::default()
        });
        let resumed = SaveFile::decode(&SaveFile::encode(&world).unwrap()).unwrap();
        assert_eq!(resumed.corpses.len(), 1);
        assert!(resumed.corpses[0].native_state.is_none());
        assert_eq!(resumed.occupied_robot_records(), 1);
    }
}
