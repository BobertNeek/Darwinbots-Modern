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
        if let Some(mut random) = self.native_random.clone() {
            let position = [
                random.integer(
                    60,
                    (self.config.world_width - 60.0).round_ties_even() as i32,
                ) as f32,
                random.integer(
                    60,
                    (self.config.world_height - 60.0).round_ties_even() as i32,
                ) as f32,
            ];
            // The modern command is atomic. Failed imports retain the RNG stream,
            // just as they retain every robot and file transfer.
            let count = self.import_native_organisms(bytes, Some(position))?;
            self.native_random = Some(random);
            self.native_insertion_sequence = self.native_insertion_sequence.saturating_add(1);
            return Ok(count);
        }
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
        let file = Self::decode_native_organisms(bytes, position)?;
        // The public method is atomic too, not only its command-batch caller.
        let mut candidate = SaveFile::decode(&SaveFile::encode(self)?)?;
        let count = candidate.apply_native_organisms(file, position, true)?;
        std::mem::swap(&mut candidate.player_bot_input, &mut self.player_bot_input);
        candidate.publish_snapshot();
        *self = candidate;
        Ok(count)
    }

    pub(super) fn decode_native_organisms(
        bytes: &[u8],
        position: Option<[f32; 2]>,
    ) -> Result<NativeRecordFile, EngineError> {
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
        Ok(file)
    }

    pub(super) fn apply_native_organisms(
        &mut self,
        file: NativeRecordFile,
        position: Option<[f32; 2]>,
        publish: bool,
    ) -> Result<usize, EngineError> {
        if file.robots.len() > self.available_robot_capacity() {
            return Err(EngineError::CapacityReached);
        }
        let native = self.native_cycle.is_some();
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
                let species = SpeciesDefinition {
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
                };
                if publish {
                    self.register_species(species);
                } else {
                    self.register_species_unpublished(species);
                }
            }
            let mut robot = self.native_robot(record)?;
            // posto clears the runtime tie count. LoadRobot restores the saved
            // memory cells, and the first Update_Ties counts the live endpoints.
            robot.native_tie_count_pending = native;
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
            if native {
                // posto clears the slot and allocates before LoadRobotBody
                // overwrites AbsNum. LoadRobot allocates again for a live zero.
                self.allocate_absolute_identity();
                if robot.flags.exists && robot.identity.absolute_id == 0 {
                    robot.identity.absolute_id = self.allocate_absolute_identity();
                }
            } else if robot.identity.absolute_id == 0 {
                robot.identity.absolute_id = self.next_absolute_id;
                self.next_absolute_id = self.next_absolute_id.saturating_add(1);
            }
            let absolute_id = robot.identity.absolute_id;
            let id = self.insert_live_robot(robot);
            old_slots.insert(old_slot, id);
            parents.entry(absolute_id).or_insert(id);
        }
        for record in &file.robots {
            let id = old_slots[&record.word("old_slot")?];
            self.slots.get_mut(id)?.identity.parents[0] = parents
                .get(&(record.long("parent")? as u32 as u64))
                .copied();
        }
        if native {
            self.load_native_tie_rows(&file, &old_slots, true)?;
        } else {
            self.native_ties(&file, &old_slots)?;
        }
        // LoadOrganism remaps links for display and traversal without writing
        // tie readbacks into either imported or existing robots' memory.
        self.refresh_tie_projections_impl(false, &HashMap::new(), publish && !native);
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
    use crate::SpeciesSettings;
    use crate::native_binary::NativeSimulationTail;

    #[test]
    fn native_dbo_founder_keeps_original_roster_quantity_through_untouched_apply() {
        let bytes = tied_file().encode().unwrap();
        let mut world = Engine::new_native(EngineConfig::testing()).unwrap();
        world.insert_native_organisms(&bytes).unwrap();
        let ids = world.slots.ids_in_slot_order();
        let founder = ids
            .iter()
            .map(|&id| world.slots.get(id).unwrap())
            .find(|robot| !robot.flags.corpse)
            .unwrap();
        let name = founder.identity.species_file.clone();
        let index = founder.identity.species.0 as usize;
        assert_eq!(world.species[index].minimum_population, 5);
        assert!(world.species_templates[index].is_none());
        assert_native_quantity(&world, &name);

        let initial_configuration = world.configuration_view().unwrap();
        let initial_row = founder_row(&initial_configuration, &name);
        assert_eq!(initial_row["initial_count"], 5);
        assert_eq!(initial_row["initial_energy"], 3000);
        let mut resumed = SaveFile::decode(&SaveFile::encode(&world).unwrap()).unwrap();
        let configuration = resumed.configuration_view().unwrap();
        assert_eq!(founder_row(&configuration, &name), initial_row);

        // Match the species editor's untouched Apply on a paused world.
        let roster: Vec<SpeciesSettings> = configuration["species"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["dna"].is_string())
            .map(|row| serde_json::from_value(row.clone()).unwrap())
            .collect();
        resumed.update_species_settings(roster).unwrap();
        assert_eq!(resumed.snapshot().tick, world.snapshot().tick);
        assert_eq!(resumed.slots.ids_in_slot_order(), ids);
        assert_native_quantity(&resumed, &name);

        let saved = SaveFile::decode(&SaveFile::encode(&resumed).unwrap()).unwrap();
        let template = saved.species_templates[index].as_ref().unwrap();
        assert_eq!(
            (template.initial_population, template.initial_energy),
            (5, 3000)
        );

        let mut portable = Engine::new(EngineConfig::testing()).unwrap();
        portable.native_random = None;
        portable.insert_native_organisms(&bytes).unwrap();
        assert_eq!(
            founder_row(&portable.configuration_view().unwrap(), &name)["initial_count"],
            1
        );

        fn founder_row<'a>(
            configuration: &'a serde_json::Value,
            name: &str,
        ) -> &'a serde_json::Value {
            configuration["species"]
                .as_array()
                .unwrap()
                .iter()
                .find(|row| row["name"].as_str() == Some(name))
                .unwrap()
        }

        fn assert_native_quantity(world: &Engine, name: &str) {
            let file = NativeRecordFile::decode(&world.export_native_sim().unwrap()).unwrap();
            let tail = NativeSimulationTail::decode(&file.tail).unwrap();
            let species = (0..tail.integer("species_count").unwrap())
                .map(|index| format!("species[{index}]"))
                .find(|row| {
                    crate::world::native_io::text(tail.bytes(&format!("{row}.name")).unwrap())
                        == name
                })
                .unwrap();
            assert_eq!(tail.integer(&format!("{species}.quantity")).unwrap(), 5);
            assert_eq!(
                tail.integer(&format!("{species}.start_energy")).unwrap(),
                3000
            );
        }
    }

    #[test]
    fn native_menu_insertion_uses_whole_twips_translates_all_cells_and_continues_after_save() {
        let file = tied_file();
        let bytes = file.encode().unwrap();
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let mut expected_random = world.native_random.clone().unwrap();
        let expected_position = [
            expected_random.integer(60, (world.config.world_width - 60.0) as i32) as f32,
            expected_random.integer(60, (world.config.world_height - 60.0) as i32) as f32,
        ];
        world.insert_native_organisms(&bytes).unwrap();
        let ids = world.slots.ids_in_slot_order();
        let anchor = world.slots.get(ids[0]).unwrap().kinematics.position;
        assert_eq!(anchor, expected_position);
        assert_eq!(world.native_random.as_ref().unwrap(), &expected_random);
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

    const SAVED_TIE_READBACKS: [i32; 3] = [431, -593, 7];
    const TIE_READBACKS: [i32; 3] = [MEM_TIE_ANGLE, MEM_TIE_LENGTH, MEM_NUMBER_OF_TIES];

    fn tie_readbacks(world: &Engine, id: OrganismId) -> [i32; 3] {
        TIE_READBACKS.map(|address| world.memory_at(id, address).unwrap())
    }

    fn set_tie_readbacks(world: &mut Engine, id: OrganismId, values: [i32; 3]) {
        let memory = &mut world.slots.get_mut(id).unwrap().program.memory;
        for (address, value) in TIE_READBACKS.into_iter().zip(values) {
            memory.write(address, value);
        }
    }

    fn tied_dbo_with_saved_readbacks() -> Vec<u8> {
        let mut donor = Engine::new_native(EngineConfig::testing()).unwrap();
        let dna = LegacyDna::parse("start stop").unwrap();
        let first = donor.spawn_at(dna.clone(), [300.0, 500.0]).unwrap();
        let second = donor.spawn_at(dna, [650.0, 500.0]).unwrap();
        assert!(donor.native_make_tie(first, second, 500, 65, 11));
        for end in donor.ties[0].native_ends.as_mut().unwrap() {
            end.rest_length = 200.0;
        }
        for id in [first, second] {
            set_tie_readbacks(&mut donor, id, SAVED_TIE_READBACKS);
        }
        donor.export_native_organisms(&[first, second]).unwrap()
    }

    #[test]
    fn native_menu_insert_exposes_the_tie_but_waits_for_the_first_recount() {
        let mut world = Engine::new_native(EngineConfig {
            world_width: 16_000.0,
            world_height: 12_000.0,
            ..EngineConfig::testing()
        })
        .unwrap();
        let dna = LegacyDna::parse("start stop").unwrap();
        let incumbent = world.spawn_at(dna.clone(), [750.0, 500.0]).unwrap();
        world.spawn_at(dna, [400.0, 500.0]).unwrap();
        set_tie_readbacks(&mut world, incumbent, [123, -456, 9]);

        assert_eq!(
            world
                .insert_native_organisms(&tied_dbo_with_saved_readbacks())
                .unwrap(),
            2
        );
        let [tie] = world.snapshot().ties.as_slice() else {
            panic!("the reciprocal tie should already be visible");
        };
        let pair = [tie.first, tie.second];
        assert_eq!(pair.map(|id| id.slot()), [2, 3]);
        assert_eq!(tie_readbacks(&world, incumbent), [123, -456, 9]);
        assert!(!world.slots.get(incumbent).unwrap().native_tie_count_pending);
        for id in pair {
            assert_eq!(tie_readbacks(&world, id), SAVED_TIE_READBACKS);
            assert!(world.slots.get(id).unwrap().native_tie_count_pending);
        }

        let mut resumed = SaveFile::decode(&SaveFile::encode(&world).unwrap()).unwrap();
        for world in [&mut world, &mut resumed] {
            world.tick().unwrap();
            assert_eq!(
                world.ties[0].native_end_states().map(|end| end.last),
                [65; 2]
            );
            for id in pair {
                let [angle, length, count] = tie_readbacks(world, id);
                assert_ne!(angle, SAVED_TIE_READBACKS[0]);
                assert_ne!(length, SAVED_TIE_READBACKS[1]);
                assert_eq!(count, 1);
                assert!(!world.slots.get(id).unwrap().native_tie_count_pending);
            }
            world.tick().unwrap();
            assert_eq!(
                world.ties[0].native_end_states().map(|end| end.last),
                [64; 2]
            );
        }
    }

    #[test]
    fn legacy_inbound_poll_preserves_readbacks_at_the_import_boundary() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("incoming.dbo");
        std::fs::write(&path, tied_dbo_with_saved_readbacks()).unwrap();
        let mut world = Engine::new_native(EngineConfig::testing()).unwrap();
        let mut inbound = Teleporter::new(1, [350.0, 500.0], 100.0, [0.0; 2]);
        inbound.local = false;
        inbound.path = folder.path().to_owned();
        world.add_teleporter(inbound).unwrap();

        // Stop at the end of the shared file poll, before running a robot pass.
        world.apply_file_teleporters().unwrap();
        assert!(!path.exists());
        assert_eq!(world.teleporters[0].received, 1);
        assert_eq!(world.ties.len(), 1);
        let ids = world.slots.ids_in_slot_order();
        for &id in &ids {
            assert_eq!(tie_readbacks(&world, id), SAVED_TIE_READBACKS);
            assert!(world.slots.get(id).unwrap().native_tie_count_pending);
        }

        world.tick().unwrap();
        assert_eq!(world.snapshot().ties.len(), 1);
        for &id in &ids {
            assert_eq!(world.slots.get(id).unwrap().kinematics.velocity, [0.0; 2]);
        }
        world.tick().unwrap();
        assert!(world.slots.get(ids[0]).unwrap().kinematics.velocity[0] > 0.0);
        assert!(world.slots.get(ids[1]).unwrap().kinematics.velocity[0] < 0.0);
    }

    #[test]
    fn portable_dbo_import_still_publishes_the_tie_readbacks_immediately() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        world.native_random = None;
        assert_eq!(
            world
                .import_native_organisms(&tied_dbo_with_saved_readbacks(), None)
                .unwrap(),
            2
        );
        assert_eq!(world.snapshot().ties.len(), 1);
        for id in world.slots.ids_in_slot_order() {
            let [angle, length, count] = tie_readbacks(&world, id);
            assert_ne!(angle, SAVED_TIE_READBACKS[0]);
            assert_ne!(length, SAVED_TIE_READBACKS[1]);
            assert_eq!(count, 1);
            assert!(!world.slots.get(id).unwrap().native_tie_count_pending);
        }
    }

    #[test]
    fn native_group_import_preserves_memory_singles_and_ties_in_reused_slots() {
        assert_native_group_in_reused_slots(Engine::new(EngineConfig::testing()).unwrap(), 2, 0);
        assert_native_group_in_reused_slots(
            Engine::new_native(EngineConfig::testing()).unwrap(),
            0,
            2,
        );
    }

    fn assert_native_group_in_reused_slots(mut target: Engine, first_slot: u32, second_slot: u32) {
        let file = tied_file();
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
        let first = OrganismId::new(first_slot, 1);
        let second = OrganismId::new(second_slot, 1);
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
    fn original_dbo_directed_and_inactive_rows_keep_the_original_insert_pointers() {
        let original_world =
            include_bytes!("../../tests/fixtures/db24832-movement-start-48032.sim");
        let one_way =
            include_bytes!("../../../docs/fixtures/native-directed-ties/back-pointer-zeroed.dbo");
        let inactive =
            include_bytes!("../../../docs/fixtures/native-directed-ties/inactive-tail.dbo");
        let mut original = NativeRecordFile::decode(one_way).unwrap();
        let old_first = original.robots[0].word("old_slot").unwrap();
        let mut back = original.robots[1].bytes("ties_core").unwrap().to_vec();
        back[40..42].copy_from_slice(&old_first.to_le_bytes());
        original.robots[1]
            .set_value("ties_core", NativeValue::Bytes(back))
            .unwrap();
        let reciprocal = original.encode().unwrap();

        fn pointers(world: &Engine, ids: &[OrganismId]) -> Vec<[i16; 2]> {
            let output =
                NativeRecordFile::decode(&world.export_original_organism_file(ids).unwrap())
                    .unwrap();
            output
                .robots
                .iter()
                .map(|record| {
                    let rows = record.bytes("ties_core").unwrap();
                    [
                        i16::from_le_bytes(rows[40..42].try_into().unwrap()),
                        i16::from_le_bytes(rows[78..80].try_into().unwrap()),
                    ]
                })
                .collect()
        }

        for (bytes, expected, counts, replaced) in [
            (reciprocal.as_slice(), [[3, 0], [3, 0]], [1, 1], None),
            (
                one_way.as_slice(),
                [[3, 0], [0, 0]],
                [1, 0],
                Some([[3, 3], [2, 0]]),
            ),
            (
                inactive.as_slice(),
                [[0, 2], [0, 1]],
                [0, 0],
                Some([[3, 2], [2, 1]]),
            ),
        ] {
            let mut world = Engine::from_native_sim(original_world).unwrap();
            assert_eq!(world.insert_native_organisms(bytes).unwrap(), 2);
            let ids = world.slots.ids_in_slot_order();
            assert_eq!(ids.len(), 3);
            assert_eq!(
                ids.iter()
                    .map(|id| world.slots.get(*id).unwrap().identity.absolute_id)
                    .collect::<Vec<_>>(),
                [1, 250, 294]
            );
            let resume = SaveFile::decode(&SaveFile::encode(&world).unwrap()).unwrap();
            let saved_sim = world.export_native_sim().unwrap();
            let sim_resume = Engine::from_native_sim(&saved_sim).unwrap();
            for unchanged in [&world, &resume, &sim_resume] {
                let actual = pointers(unchanged, &ids[1..]);
                assert_eq!(actual, expected);
                assert_eq!(
                    [
                        unchanged.tie_indices_for(ids[1]).len(),
                        unchanged.tie_indices_for(ids[2]).len()
                    ],
                    counts
                );
            }
            if let Some(expected_after_creation) = replaced {
                world.slots.get_mut(ids[2]).unwrap().materials.slime = 0.0;
                assert!(world.native_make_tie(ids[1], ids[2], 32_000, 50, 7));
                // The dormant row can activate against the incumbent.
                // Include all three robots, as the original SIM capture did.
                assert_eq!(pointers(&world, &ids)[1..], expected_after_creation);
                let resumed = SaveFile::decode(&SaveFile::encode(&world).unwrap()).unwrap();
                assert_eq!(pointers(&resumed, &ids)[1..], expected_after_creation);
            }
        }
    }

    #[test]
    fn native_dbo_allocates_provisional_ids_and_resumes_the_saved_cursor() {
        use crate::native_binary::NativeSimulationTail;

        let mut file = tied_file();
        file.robots[1]
            .set_value("absolute_id", NativeValue::Long(294))
            .unwrap();
        let dna = LegacyDna::parse("stop").unwrap();

        for (wire_id, imported_id, following_id) in [(250, 250, 4), (0, 3, 5)] {
            file.robots[0]
                .set_value("absolute_id", NativeValue::Long(wire_id))
                .unwrap();
            let bytes = file.encode().unwrap();
            let mut world = Engine::new_native(EngineConfig::testing()).unwrap();
            let incumbent = world.spawn(dna.clone()).unwrap();
            assert_eq!(world.slots.get(incumbent).unwrap().identity.absolute_id, 1);
            assert_eq!(world.next_absolute_id, 2);

            world.import_native_organisms(&bytes, None).unwrap();
            let ids = world.slots.ids_in_slot_order();
            let alive_ids: Vec<_> = ids
                .into_iter()
                .map(|id| world.slots.get(id).unwrap().identity.absolute_id)
                .collect();
            assert_eq!(alive_ids, [1, imported_id, 294]);
            assert_eq!(world.next_absolute_id, following_id);

            let modern = SaveFile::encode(&world).unwrap();
            let mut modern_resume = SaveFile::decode(&modern).unwrap();
            let native = world.export_native_sim().unwrap();
            let file = NativeRecordFile::decode(&native).unwrap();
            let tail = NativeSimulationTail::decode(&file.tail).unwrap();
            assert_eq!(
                tail.integer("max_absolute_id").unwrap(),
                following_id as i32 - 1
            );
            let mut native_resume = Engine::from_native_sim(&native).unwrap();
            for resumed in [&mut world, &mut modern_resume, &mut native_resume] {
                assert_eq!(resumed.next_absolute_id, following_id);
                let born = resumed.spawn(dna.clone()).unwrap();
                assert_eq!(
                    resumed.slots.get(born).unwrap().identity.absolute_id,
                    following_id
                );
            }
        }
    }

    #[test]
    fn older_native_missing_cursor_and_portable_restore_infer_from_observed_ids() {
        let dna = LegacyDna::parse("stop").unwrap();
        for native in [false, true] {
            let mut world = if native {
                Engine::new_native(EngineConfig::testing()).unwrap()
            } else {
                Engine::new(EngineConfig::testing()).unwrap()
            };
            let actor = world.spawn(dna.clone()).unwrap();
            world.slots.get_mut(actor).unwrap().identity.absolute_id = 294;
            let mut payload = world.db3s_v3_payload().unwrap();
            if native {
                payload.as_object_mut().unwrap().remove("next_absolute_id");
            }
            let restored = Engine::restore(Engine::from_db3s_v3_payload(payload).unwrap()).unwrap();
            assert_eq!(restored.next_absolute_id, 295);
        }
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
