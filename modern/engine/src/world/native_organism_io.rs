//! Native organism groups retain robot state and commit as one operation.

use super::native_io::{color, skin, text};
use super::*;
use crate::SaveFile;
use crate::native_binary::{NativeRecordFile, NativeValue};

fn invalid(message: impl Into<String>) -> EngineError {
    EngineError::InvalidSave(format!("native DB2 organism import {}", message.into()))
}

impl Engine {
    /// HDRoutines.InsertOrganism chooses integer coordinates from the inclusive
    /// 60..FieldSize-60 interval, then translates the entire incoming group.
    pub fn insert_native_organisms(&mut self, bytes: &[u8]) -> Result<usize, EngineError> {
        let first = advance_random(
            self.config.seed
                ^ self.native_insertion_sequence.rotate_left(29)
                ^ self.tick.rotate_left(17),
        );
        let position = [
            insertion_coordinate(first, self.config.world_width),
            insertion_coordinate(advance_random(first), self.config.world_height),
        ];
        let count = self.import_native_organisms(bytes, Some(position))?;
        self.native_insertion_sequence = self.native_insertion_sequence.saturating_add(1);
        Ok(count)
    }

    /// Import a native `.dbo`, translating every cell by the first cell's
    /// displacement when a destination is supplied (HDRoutines.PlaceOrganism).
    pub fn import_native_organisms(
        &mut self,
        bytes: &[u8],
        position: Option<[f32; 2]>,
    ) -> Result<usize, EngineError> {
        let file = NativeRecordFile::decode(bytes)?;
        if !file.tail.is_empty() || file.robots.is_empty() || file.robots.len() > 50 {
            return Err(invalid(
                "a .dbo must contain 1..50 robot records and no simulation tail",
            ));
        }
        if position.is_some_and(|p| p.iter().any(|v| !v.is_finite())) {
            return Err(invalid("destination is not finite"));
        }
        for record in &file.robots {
            for field in record.fields() {
                let finite = match field.value {
                    NativeValue::Single(bits) => f32::from_bits(bits).is_finite(),
                    NativeValue::Double(bits) => f64::from_bits(bits).is_finite(),
                    _ => true,
                };
                if !finite {
                    return Err(invalid(format!("{} is not finite", field.name)));
                }
            }
        }
        // The public method is atomic too, not only its command-batch caller.
        let mut candidate = SaveFile::decode(&SaveFile::encode(self)?)?;
        let count = candidate.apply_native_organisms(file, position)?;
        *self = candidate;
        Ok(count)
    }

    fn apply_native_organisms(
        &mut self,
        file: NativeRecordFile,
        position: Option<[f32; 2]>,
    ) -> Result<usize, EngineError> {
        if file.robots.len() > self.available_robot_capacity() {
            return Err(EngineError::CapacityReached);
        }
        let mut old_slots = HashMap::new();
        let mut parents: HashMap<_, _> = self
            .slots
            .ids_in_slot_order()
            .into_iter()
            .map(|id| (self.slots.get(id).unwrap().identity.absolute_id, id))
            .collect();
        let anchor = [
            file.robots[0].single("position_x")?,
            file.robots[0].single("position_y")?,
        ];
        let displacement = position.map_or([0.0; 2], |p| [p[0] - anchor[0], p[1] - anchor[1]]);
        for record in &file.robots {
            let old_slot = record.word("old_slot")?;
            if old_slot <= 0 || old_slots.contains_key(&old_slot) {
                return Err(invalid("duplicate or invalid old robot slot"));
            }
            let name = text(record.bytes("species_name")?);
            if record.word("corpse")? == 0
                && name != "Corpse"
                && !self.species.iter().any(|s| s.name == name)
            {
                // AddSpecie uses default species mutation rates; the robot
                // retains its own complete incoming mutation profile.
                self.register_species(SpeciesDefinition {
                    name,
                    vegetable: record.word("vegetable")? != 0,
                    color: color(record.long("color")?),
                    skin: skin(record.bytes("skin")?),
                    custom_skin: true,
                    minimum_population: 5,
                    reseed: false,
                    mutation_profile: Some(self.default_mutation_profile.clone()),
                    restrictions: Some(RestrictionColumn {
                        disable_vision: record.word("cant_see")? != 0,
                        disable_dna: record.word("disable_dna")? != 0,
                        disable_movement: record.word("disable_movement")? != 0,
                        disable_reproduction: record.word("cant_reproduce")? != 0,
                        virus_immune: record.word("virus_immune")? != 0,
                        disable_mutations: record.word("mutations_enabled")? == 0,
                        ..RestrictionColumn::default()
                    }),
                    ..SpeciesDefinition::default()
                });
            }
            let mut robot = self.native_robot(record)?;
            // Incoming metadata is owned by the live robot. Removal releases
            // it; repeated teleporter imports cannot grow a global archive.
            robot.native_record_override = Some(Box::new(record.clone()));
            for (axis, delta) in displacement.into_iter().enumerate() {
                robot.kinematics.position[axis] += delta;
                robot.kinematics.previous_position[axis] = robot.kinematics.position[axis];
                if !robot.kinematics.position[axis].is_finite() {
                    return Err(invalid("translated position is not finite"));
                }
            }
            if robot.identity.absolute_id == 0 {
                robot.identity.absolute_id = self.next_absolute_id;
                self.next_absolute_id = self.next_absolute_id.saturating_add(1);
            }
            let absolute_id = robot.identity.absolute_id;
            let id = self.slots.spawn(robot);
            old_slots.insert(old_slot, id);
            parents.entry(absolute_id).or_insert(id);
        }
        for record in &file.robots {
            let id = old_slots[&record.word("old_slot")?];
            self.slots.get_mut(id)?.identity.parents[0] = parents
                .get(&(record.long("parent")? as u32 as u64))
                .copied();
        }
        self.native_ties(&file, &old_slots)?;
        self.refresh_tie_projections();
        self.publish_snapshot();
        Ok(file.robots.len())
    }
}

fn insertion_coordinate(random: u64, extent: f32) -> f32 {
    let high = (f64::from(extent) - 60.0).floor();
    if high < 60.0 {
        return (extent / 2.0).round_ties_even();
    }
    // Source Random uses Int((hi-low+1)*rndy+low), yielding whole twips.
    (60.0 + ((high - 59.0) * (random >> 40) as f64 / (1_u64 << 24) as f64).floor()) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_menu_insertion_uses_whole_twips_translates_all_cells_and_continues_after_save() {
        let file = tied_file();
        let bytes = file.encode().unwrap();
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        world.insert_native_organisms(&bytes).unwrap();
        let ids = world.slots.ids_in_slot_order();
        let anchor = world.slots.get(ids[0]).unwrap().kinematics.position;
        for (axis, extent) in [world.config.world_width, world.config.world_height]
            .into_iter()
            .enumerate()
        {
            assert_eq!(anchor[axis].fract(), 0.0);
            assert!((60.0..=extent - 60.0).contains(&anchor[axis]));
        }
        let delta = [
            anchor[0] - file.robots[0].single("position_x").unwrap(),
            anchor[1] - file.robots[0].single("position_y").unwrap(),
        ];
        assert_eq!(
            world.slots.get(ids[1]).unwrap().kinematics.position,
            [
                file.robots[1].single("position_x").unwrap() + delta[0],
                file.robots[1].single("position_y").unwrap() + delta[1]
            ]
        );
        let mut resumed = SaveFile::decode(&SaveFile::encode(&world).unwrap()).unwrap();
        world.insert_native_organisms(&bytes).unwrap();
        resumed.insert_native_organisms(&bytes).unwrap();
        assert_eq!(
            SaveFile::encode(&world).unwrap(),
            SaveFile::encode(&resumed).unwrap()
        );
        let before = SaveFile::encode(&resumed).unwrap();
        assert!(
            resumed
                .insert_native_organisms(&bytes[..bytes.len() - 1])
                .is_err()
        );
        assert_eq!(
            before,
            SaveFile::encode(&resumed).unwrap(),
            "failure preserves placement sequence too"
        );
    }

    fn tied_file() -> NativeRecordFile {
        let mut source = Engine::from_native_sim(include_bytes!(
            "../../tests/fixtures/db24832-original-user.sim"
        ))
        .unwrap();
        let tie = source.ties[0].clone();
        source
            .slots
            .get_mut(tie.first)
            .unwrap()
            .program
            .memory
            .write(1000, -712);
        NativeRecordFile::decode(
            &source
                .export_native_organisms(&[tie.first, tie.second])
                .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn native_group_import_preserves_memory_singles_and_ties_in_reused_slots() {
        let file = tied_file();
        let mut target = Engine::new(EngineConfig::testing()).unwrap();
        let mut ids = vec![];
        for _ in 0..4 {
            ids.push(
                target
                    .spawn_at(LegacyDna::parse("stop").unwrap(), [750.0, 500.0])
                    .unwrap(),
            );
        }
        target.remove(ids[0]).unwrap();
        target.remove(ids[2]).unwrap();
        assert_eq!(
            target
                .import_native_organisms(&file.encode().unwrap(), Some([500.0, 500.0]))
                .unwrap(),
            2
        );
        let first = OrganismId::new(2, 1);
        let second = OrganismId::new(0, 1);
        let robot = target.slots.get(first).unwrap();
        assert_eq!(robot.program.memory.read(1000), -712);
        assert_eq!(
            robot.materials.energy.to_bits(),
            file.robots[0].single("energy").unwrap().to_bits()
        );
        assert_eq!(
            robot.heading().to_bits(),
            file.robots[0].single("aim").unwrap().to_bits()
        );
        assert_eq!(robot.kinematics.position, [500.0, 500.0]);
        let offset = [
            file.robots[1].single("position_x").unwrap()
                - file.robots[0].single("position_x").unwrap(),
            file.robots[1].single("position_y").unwrap()
                - file.robots[0].single("position_y").unwrap(),
        ];
        assert_eq!(
            target.slots.get(second).unwrap().kinematics.position,
            [500.0 + offset[0], 500.0 + offset[1]]
        );
        assert!(
            target
                .ties
                .iter()
                .any(|tie| tie.first == first && tie.second == second)
        );
        assert_eq!(
            target
                .slots
                .get(second)
                .unwrap()
                .ties
                .iter()
                .flatten()
                .next()
                .unwrap()
                .other,
            Some(first)
        );
    }

    #[test]
    fn repeated_native_imports_keep_duplicate_absolute_ids_and_separate_metadata() {
        let mut file = tied_file();
        let mark = |file: &mut NativeRecordFile, value: i16| {
            let mut core = file.robots[0].bytes("ties_core").unwrap().to_vec();
            core[38 + 24..38 + 26].copy_from_slice(&value.to_le_bytes());
            file.robots[0]
                .set_value("ties_core", NativeValue::Bytes(core))
                .unwrap();
        };
        let mut target = Engine::new(EngineConfig::testing()).unwrap();
        mark(&mut file, -712);
        target
            .import_native_organisms(&file.encode().unwrap(), None)
            .unwrap();
        mark(&mut file, -713);
        target
            .import_native_organisms(&file.encode().unwrap(), None)
            .unwrap();
        let ids = target.slots.ids_in_slot_order();
        assert!(target.native_origin.is_none());
        assert_eq!(
            target.slots.get(ids[0]).unwrap().identity.absolute_id,
            target.slots.get(ids[2]).unwrap().identity.absolute_id
        );
        for (members, expected) in [(&ids[..2], -712_i16), (&ids[2..], -713_i16)] {
            let exported =
                NativeRecordFile::decode(&target.export_native_organisms(members).unwrap())
                    .unwrap();
            assert_eq!(
                &exported.robots[0].bytes("ties_core").unwrap()[38 + 24..38 + 26],
                &expected.to_le_bytes()
            );
        }
        let sim = target.export_native_sim().unwrap();
        let loaded = Engine::from_native_sim(&sim).unwrap();
        assert_eq!(loaded.population(), 4);
        assert_eq!(loaded.ties.len(), 2);
    }

    #[test]
    fn invalid_later_native_member_rolls_back_all_imported_state() {
        let mut file = tied_file();
        let mut dna = file.robots[1].bytes("dna").unwrap().to_vec();
        dna[..2].copy_from_slice(&99_i16.to_le_bytes());
        file.robots[1]
            .set_value("dna", NativeValue::Bytes(dna))
            .unwrap();
        let mut target = Engine::new(EngineConfig::testing()).unwrap();
        target.spawn(LegacyDna::parse("stop").unwrap()).unwrap();
        let before = SaveFile::encode(&target).unwrap();
        assert!(
            target
                .import_native_organisms(&file.encode().unwrap(), None)
                .is_err()
        );
        assert_eq!(SaveFile::encode(&target).unwrap(), before);
    }

    #[test]
    fn malformed_native_tie_is_rejected_without_partial_import() {
        let mut file = tied_file();
        let mut core = file.robots[0].bytes("ties_core").unwrap().to_vec();
        core[38 + 6..38 + 10].copy_from_slice(&f32::NAN.to_le_bytes());
        file.robots[0]
            .set_value("ties_core", NativeValue::Bytes(core))
            .unwrap();
        let mut target = Engine::new(EngineConfig::testing()).unwrap();
        let before = SaveFile::encode(&target).unwrap();
        assert!(
            target
                .import_native_organisms(&file.encode().unwrap(), None)
                .is_err()
        );
        assert_eq!(SaveFile::encode(&target).unwrap(), before);
    }

    #[test]
    fn native_integer_memory_preserves_full_word_range_until_a_dna_store() {
        let mut file = tied_file();
        let mut memory = file.robots[0].bytes("memory").unwrap().to_vec();
        memory[1998..2000].copy_from_slice(&i16::MIN.to_le_bytes());
        memory[2000..2002].copy_from_slice(&i16::MAX.to_le_bytes());
        file.robots[0]
            .set_value("memory", NativeValue::Bytes(memory))
            .unwrap();
        let mut target = Engine::new(EngineConfig::testing()).unwrap();
        target
            .import_native_organisms(&file.encode().unwrap(), None)
            .unwrap();
        let id = target.slots.ids_in_slot_order()[0];
        assert_eq!(
            target.slots.get(id).unwrap().program.memory.read(999),
            -32768
        );
        assert_eq!(
            target.slots.get(id).unwrap().program.memory.read(1000),
            32767
        );
        let bytes = target.export_native_organisms(&[id]).unwrap();
        let exported = NativeRecordFile::decode(&bytes).unwrap();
        assert_eq!(
            &exported.robots[0].bytes("memory").unwrap()[1998..2002],
            &[0, 128, 255, 127]
        );
        let mut resumed = SaveFile::decode(&SaveFile::encode(&target).unwrap()).unwrap();
        assert_eq!(
            resumed.slots.get(id).unwrap().program.memory.read(1000),
            32767
        );
        resumed
            .slots
            .get_mut(id)
            .unwrap()
            .program
            .memory
            .write(1000, 32767);
        assert_eq!(
            resumed.slots.get(id).unwrap().program.memory.read(1000),
            767
        );
    }
}
