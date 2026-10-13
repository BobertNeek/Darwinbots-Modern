//! Retain the complete robot at death and materialize it for native exchange.

use super::*;
use crate::SaveFile;

pub(super) struct NativeCorpseTransfer {
    pub bytes: Vec<u8>,
    pub living: Vec<OrganismId>,
    pub archived: Vec<usize>,
}

impl Engine {
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
        if self.population() + indices.len() > MAX_ROBOTS {
            return Err(EngineError::InvalidSave(
                "native world exceeds 32000 records".into(),
            ));
        }
        let mut world = SaveFile::decode(&SaveFile::encode(self)?)?;
        world.corpses.clear();
        let corpses = indices
            .iter()
            .map(|&index| &self.corpses[index])
            .collect::<Vec<_>>();
        let mut remap = HashMap::new();
        let mut members = Vec::with_capacity(corpses.len());
        for corpse in &corpses {
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
            robot.materials.body = corpse.body as f32 + corpse.body_fraction;
            robot.kinematics.position = corpse.position;
            robot.kinematics.previous_position = corpse.position;
            robot.kinematics.velocity = corpse.velocity;
            robot.counters.age = corpse.age;
            robot.counters.decay_timer = corpse.decay_timer.min(u32::MAX as u64) as u32;
            for address in 501..=509 {
                robot.program.memory.write(address, 0);
            }
            let id = world.slots.spawn(robot);
            if let Some(old_id) = corpse.native_id {
                remap.insert(old_id, id);
            }
            members.push(id);
        }
        for slot in world.slots.iter_mut() {
            if let Some(robot) = slot.organism.as_mut() {
                for endpoint in robot.ties.iter_mut().flatten() {
                    if let Some(other) = endpoint.other.and_then(|id| remap.get(&id).copied()) {
                        endpoint.other = Some(other);
                    }
                }
            }
        }
        world.ties.retain(|tie| {
            world.slots.get(tie.first).is_ok() && world.slots.get(tie.second).is_ok()
        });
        for tie in corpses.iter().flat_map(|corpse| &corpse.native_ties) {
            let mut tie = tie.clone();
            tie.first = remap.get(&tie.first).copied().unwrap_or(tie.first);
            tie.second = remap.get(&tie.second).copied().unwrap_or(tie.second);
            if world.slots.get(tie.first).is_ok()
                && world.slots.get(tie.second).is_ok()
                && !world
                    .ties
                    .iter()
                    .any(|existing| existing.contains(tie.first) && existing.contains(tie.second))
            {
                world.ties.push(tie);
            }
        }
        Ok((world, members))
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
        world.slots.remove(tie.first).unwrap();
        let dead_id = tie.first;
        world.ties.retain(|tie| !tie.contains(dead_id));
        world.corpses.push(corpse);
        world.publish_snapshot();
        assert!(world.snapshot.corpses[0].native_state.is_none());
        let modern = SaveFile::encode(&world).unwrap();
        let resumed = SaveFile::decode(&modern).unwrap();
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
            NativeRecordFile::decode(&resumed.native_corpse_transfer(0).unwrap().bytes).unwrap();
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
}
