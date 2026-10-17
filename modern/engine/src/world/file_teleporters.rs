//! Offline file exchange uses complete native organism groups.

use super::*;
use crate::SaveFile;

impl Engine {
    pub(super) fn has_file_transfers(&self) -> bool {
        !self.file_teleporters.is_empty()
            || self.teleporters.iter().any(|tele| {
                !tele.internet
                    && ((tele.inbound
                        && (!tele.inbound_path.as_os_str().is_empty()
                            || !tele.path.as_os_str().is_empty()))
                        || (tele.outbound
                            && (!tele.outbound_path.as_os_str().is_empty()
                                || !tele.path.as_os_str().is_empty())))
            })
    }

    #[cfg(test)]
    pub(super) fn apply_file_teleporters(&mut self) -> Result<(), EngineError> {
        let checkpoint = self.cycle_checkpoint();
        let mut transfers = FileTransferTransaction::default();
        let result = self
            .apply_file_teleporters_staged(&mut transfers)
            .and_then(|()| transfers.commit());
        if result.is_err() {
            self.restore_cycle_checkpoint(checkpoint);
        }
        result
    }

    pub(super) fn apply_file_teleporters_staged(
        &mut self,
        transfers: &mut FileTransferTransaction,
    ) -> Result<(), EngineError> {
        let configured_count = self.file_teleporters.len();
        let mut gateways = self
            .file_teleporters
            .iter()
            .cloned()
            .map(|gateway| (gateway, None))
            .collect::<Vec<_>>();
        for (index, tele) in self.teleporters.iter().enumerate() {
            if tele.internet || (!tele.inbound && !tele.outbound) {
                continue;
            }
            let inbound = if tele.inbound {
                if tele.inbound_path.as_os_str().is_empty() {
                    tele.path.clone()
                } else {
                    tele.inbound_path.clone()
                }
            } else {
                Default::default()
            };
            let outbound = if tele.outbound {
                if tele.outbound_path.as_os_str().is_empty() {
                    tele.path.clone()
                } else {
                    tele.outbound_path.clone()
                }
            } else {
                Default::default()
            };
            let mut gateway = FileTeleporter::new(tele.id, tele.center, tele.radius)
                .with_paths(inbound, outbound)
                .with_polling(tele.poll_interval, tele.capacity)
                .with_filters(tele.filters.clone());
            gateway.poll_countdown = tele.poll_countdown;
            gateway.sent = tele.sent;
            gateway.received = tele.received;
            gateway.backflow_guard = tele.file_backflow_guard.clone();
            gateways.push((gateway, Some(index)));
        }
        for (index, (mut gateway, native_index)) in gateways.into_iter().enumerate() {
            let settings = gateway.clone();
            let previous_guard = settings.backflow_guard.clone();
            let mut sent = 0_u64;
            let mut emitted_paths = Vec::new();

            if !settings.outbound_path.as_os_str().is_empty() {
                for id in self.slots.ids_in_slot_order() {
                    if native_index.is_none() && sent >= settings.capacity as u64 {
                        break;
                    }
                    let Ok(organism) = self.organism(id) else {
                        continue;
                    };
                    let robot = self.slots.get(id)?;
                    if !touches_gateway(&settings, organism.position, organism.radius)
                        || !settings.accepts(organism.vegetable, robot.flags.corpse)
                    {
                        continue;
                    }
                    let Ok(members) = self.file_transfer_group(id) else {
                        continue;
                    };
                    let filename = format!(
                        "db2-{tick:016x}-{slot:08x}-{generation:08x}.dbo",
                        tick = self.tick,
                        slot = id.slot(),
                        generation = id.generation(),
                    );
                    let path = settings.outbound_path.join(filename);
                    let Ok(bytes) = self.export_native_organisms(&members) else {
                        continue;
                    };
                    if transfers.stage_export(&path, &bytes).is_err() {
                        continue;
                    }
                    if self.remove_many_with_death_poffs(&members, true).is_ok() {
                        sent = sent.saturating_add(1);
                        emitted_paths.push(path);
                    } else {
                        transfers.cancel_export(&path);
                    }
                }

                let mut corpse_index = 0;
                while corpse_index < self.corpses.len()
                    && (native_index.is_some() || sent < settings.capacity as u64)
                {
                    let corpse = self.corpses[corpse_index].clone();
                    let radius = crate::physics::organism_radius(
                        corpse.body as f32 + corpse.body_fraction,
                        0.0,
                        self.config.fixed_bot_radii,
                    );
                    if !touches_gateway(&settings, corpse.position, radius)
                        || !settings.accepts(false, true)
                    {
                        corpse_index += 1;
                        continue;
                    }
                    let path = settings.outbound_path.join(format!(
                        "db2-{tick:016x}-corpse-{index:08x}-{sequence:016x}.dbo",
                        tick = self.tick,
                        index = corpse_index,
                        sequence = settings.sent.saturating_add(sent),
                    ));
                    let Ok(transfer) = self.native_corpse_transfer(corpse_index) else {
                        corpse_index += 1;
                        continue;
                    };
                    if transfers.stage_export(&path, &transfer.bytes).is_err() {
                        corpse_index += 1;
                        continue;
                    }
                    if !transfer.living.is_empty() {
                        self.remove_many_with_death_poffs(&transfer.living, true)?;
                    }
                    let mut archived = transfer.archived;
                    archived.sort_unstable();
                    for archived_index in archived.into_iter().rev() {
                        self.corpses.remove(archived_index);
                    }
                    corpse_index = 0;
                    sent = sent.saturating_add(1);
                    emitted_paths.push(path);
                }
            }

            let poll_due = if settings.inbound_path.as_os_str().is_empty() {
                false
            } else {
                let teleporter = &mut gateway;
                if teleporter.poll_countdown > 0 {
                    teleporter.poll_countdown -= 1;
                    native_index.is_none() && teleporter.poll_countdown == 0
                } else {
                    teleporter.poll_countdown = teleporter.poll_interval;
                    true
                }
            };
            let mut received = 0_u64;
            if poll_due {
                let mut paths = fs::read_dir(&settings.inbound_path)
                    .ok()
                    .into_iter()
                    .flatten()
                    .filter_map(Result::ok)
                    .map(|entry| entry.path())
                    .filter(|path| {
                        path.is_file()
                            && path
                                .extension()
                                .and_then(|extension| extension.to_str())
                                .is_some_and(|extension| extension.eq_ignore_ascii_case("dbo"))
                    })
                    .collect::<Vec<_>>();
                paths.sort_by(|left, right| left.to_string_lossy().cmp(&right.to_string_lossy()));

                for path in paths {
                    if received >= settings.capacity as u64 {
                        break;
                    }
                    if previous_guard.as_ref().is_some_and(|guard| guard == &path)
                        || emitted_paths.iter().any(|emitted| emitted == &path)
                    {
                        continue;
                    }
                    let Ok(bytes) = fs::read(&path) else {
                        continue;
                    };
                    // TeleportInBots counts files, not cells. Its type filters
                    // apply to departures. Claim the input under a recovery name
                    // before accepting it; finish only after the cycle succeeds.
                    let mut candidate = SaveFile::decode(&SaveFile::encode(self)?)?;
                    let imported = if bytes.starts_with(b"DB2S") {
                        LegacySimulationFile::decode(&bytes).and_then(|transfer| {
                            for organism in &transfer.organisms {
                                candidate.import_legacy_organism(organism)?;
                            }
                            Ok(transfer.organisms.len())
                        })
                    } else if bytes.starts_with(b"DB2O") {
                        LegacyOrganismFile::decode(&bytes)
                            .and_then(|file| candidate.import_legacy_organism_file(&file))
                    } else {
                        candidate.import_native_organisms(
                            &bytes,
                            Some(if let Some(native_index) = native_index {
                                [
                                    settings.center[0],
                                    settings.center[1]
                                        - self.teleporters[native_index].height / 6.0,
                                ]
                            } else {
                                settings.center
                            }),
                        )
                    };
                    if imported.is_ok_and(|count| count > 0)
                        && transfers.claim_input(&path, &bytes).is_ok()
                    {
                        // Importing into a candidate must preserve the active
                        // backend, including its runtime failure state.
                        std::mem::swap(&mut candidate.physics, &mut self.physics);
                        *self = candidate;
                        received = received.saturating_add(1);
                    }
                }
            }

            let teleporter = &mut gateway;
            teleporter.sent = teleporter.sent.saturating_add(sent);
            teleporter.received = teleporter.received.saturating_add(received);
            if poll_due {
                teleporter.backflow_guard = None;
            }
            if settings.inbound_path == settings.outbound_path && !emitted_paths.is_empty() {
                teleporter.backflow_guard = emitted_paths.last().cloned();
            }
            if index < configured_count {
                self.file_teleporters[index] = teleporter.clone();
            } else if let Some(native_index) = native_index {
                let native = &mut self.teleporters[native_index];
                native.sent = teleporter.sent;
                native.received = teleporter.received;
                native.poll_countdown = teleporter.poll_countdown;
                native.file_backflow_guard = teleporter.backflow_guard.clone();
            }
        }
        Ok(())
    }
    // Multibots.ListCells follows ties from cells marked Multibot;
    // a birth tie alone does not turn its parent into a group.
    pub(super) fn file_transfer_group(
        &self,
        seed: OrganismId,
    ) -> Result<Vec<OrganismId>, EngineError> {
        let mut members = vec![seed];
        let mut cursor = 0;
        while cursor < members.len() {
            let robot = self.slots.get(members[cursor])?;
            if robot.counters.multibot {
                for other in robot.ties.iter().flatten().filter_map(|tie| tie.other) {
                    if self.slots.get(other).is_ok() && !members.contains(&other) {
                        if members.len() == 50 {
                            return Err(EngineError::InvalidSave(
                                "native organism transfer exceeds 50 cells".into(),
                            ));
                        }
                        members.push(other);
                    }
                }
            }
            cursor += 1;
        }
        Ok(members)
    }
}

fn touches_gateway(gateway: &FileTeleporter, position: [f32; 2], radius: f32) -> bool {
    let dx = position[0] - gateway.center[0];
    let dy = position[1] - gateway.center[1];
    let reach = gateway.radius + radius;
    dx * dx + dy * dy < reach * reach
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_binary::NativeRecordFile;
    use tempfile::tempdir;

    #[test]
    fn output_collision_preserves_existing_file_and_departing_robot() {
        let root = tempdir().unwrap();
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let id = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [500.0; 2])
            .unwrap();
        world
            .add_file_teleporter(
                FileTeleporter::new(1, [500.0; 2], 100.0)
                    .with_paths("", root.path())
                    .with_polling(1, 1),
            )
            .unwrap();
        let path = root.path().join(format!(
            "db2-{:016x}-{:08x}-{:08x}.dbo",
            world.tick,
            id.slot(),
            id.generation()
        ));
        fs::write(&path, b"existing irreplaceable transfer").unwrap();
        let before = SaveFile::encode(&world).unwrap();
        world.apply_file_teleporters().unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"existing irreplaceable transfer");
        assert_eq!(SaveFile::encode(&world).unwrap(), before);
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    }

    #[test]
    fn file_exchange_moves_a_native_multibot_as_one_file_with_memory_stores_and_tie_intact() {
        let root = tempdir().unwrap();
        let mut source = Engine::from_native_sim(include_bytes!(
            "../../tests/fixtures/db24832-original-user.sim"
        ))
        .unwrap();
        let tie = source.ties[0].clone();
        let group = source
            .export_native_organisms(&[tie.first, tie.second])
            .unwrap();
        source = Engine::new(EngineConfig::testing()).unwrap();
        source
            .import_native_organisms(&group, Some([500.0; 2]))
            .unwrap();
        let members = source.slots.ids_in_slot_order();
        source.ties[0].hardened = true;
        if let Some(ends) = source.ties[0].native_ends.as_mut() {
            for end in ends {
                end.hardened = true;
                end.kind = 3;
            }
        }
        source.refresh_tie_projections();
        for &id in &members {
            let robot = source.slots.get_mut(id).unwrap();
            // regang publishes the independent Multibot latch as it hardens
            // the endpoint. A projection refresh must not synthesize it.
            robot.counters.multibot = true;
            robot.program.memory.write(MEM_MULTI, 1);
            robot.program.memory.restore_native_cell(1000, 32767);
        }
        assert_eq!(source.file_transfer_group(members[0]).unwrap(), members);
        let energy = source.slots.get(members[0]).unwrap().materials.energy;
        source
            .add_file_teleporter(
                FileTeleporter::new(1, [500.0; 2], 100.0)
                    .with_paths("", root.path())
                    .with_polling(1, 1),
            )
            .unwrap();
        source.apply_file_teleporters().unwrap();
        assert_eq!(source.population(), 0);
        assert_eq!(source.file_teleporters[0].sent, 1);
        let files = fs::read_dir(root.path())
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect::<Vec<_>>();
        assert_eq!(files.len(), 1);
        let native = NativeRecordFile::decode(&fs::read(&files[0]).unwrap()).unwrap();
        assert_eq!(native.robots.len(), 2);
        assert!(native.tail.is_empty());
        assert_eq!(
            native.robots[0].single("energy").unwrap().to_bits(),
            energy.to_bits()
        );
        let mut destination = Engine::new(EngineConfig::testing()).unwrap();
        destination
            .add_file_teleporter(
                FileTeleporter::new(2, [750.0, 600.0], 100.0)
                    .with_paths(root.path(), "")
                    .with_polling(1, 1),
            )
            .unwrap();
        destination.apply_file_teleporters().unwrap();
        assert_eq!(
            destination.population(),
            2,
            "one-file poll admits both cells"
        );
        assert_eq!(destination.file_teleporters[0].received, 1);
        assert!(!files[0].exists());
        assert_eq!(destination.ties.len(), 1);
        for id in destination.slots.ids_in_slot_order() {
            assert_eq!(destination.memory_at(id, 1000).unwrap(), 32767);
        }
    }

    #[test]
    fn native_sim_offline_gateway_uses_its_saved_path_geometry_and_poll_state() {
        let root = tempdir().unwrap();
        let mut source = Engine::new(EngineConfig {
            brownian_motion: 0.0,
            ..EngineConfig::testing()
        })
        .unwrap();
        let id = source
            .spawn_at(LegacyDna::parse("stop").unwrap(), [500.0; 2])
            .unwrap();
        let mut tele = Teleporter::new(4, [500.0; 2], 100.0, [1000.0; 2]);
        tele.local = false;
        tele.inbound = false;
        tele.outbound = true;
        tele.path = root.path().to_path_buf();
        source.add_teleporter(tele).unwrap();
        let native = source.export_native_sim().unwrap();
        let mut resumed = Engine::from_native_sim(&native).unwrap();
        assert_eq!(resumed.teleporters[0].center, [500.0; 2]);
        resumed.apply_file_teleporters().unwrap();
        assert_eq!(resumed.population(), 0);
        assert_eq!(resumed.teleporters[0].sent, 1);
        let path = fs::read_dir(root.path())
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let file = NativeRecordFile::decode(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(
            file.robots[0].long("absolute_id").unwrap() as u64,
            source.slots.get(id).unwrap().identity.absolute_id
        );
        resumed.teleporters[0].outbound = false;
        resumed.teleporters[0].inbound = true;
        resumed.teleporters[0].poll_countdown = 0;
        resumed.teleporters[0].file_backflow_guard = None;
        resumed.apply_file_teleporters().unwrap();
        assert_eq!(resumed.population(), 1);
        assert_eq!(resumed.teleporters[0].received, 1);
        let robot = resumed
            .slots
            .get(resumed.slots.ids_in_slot_order()[0])
            .unwrap();
        assert_eq!(robot.kinematics.position, [500.0, 500.0 - 200.0 / 6.0]);
    }

    #[test]
    fn native_group_capacity_failure_keeps_file_and_existing_world() {
        let root = tempdir().unwrap();
        let source = Engine::from_native_sim(include_bytes!(
            "../../tests/fixtures/db24832-original-user.sim"
        ))
        .unwrap();
        let tie = &source.ties[0];
        let file = root.path().join("group.dbo");
        fs::write(
            &file,
            source
                .export_native_organisms(&[tie.first, tie.second])
                .unwrap(),
        )
        .unwrap();
        let mut destination = Engine::new(EngineConfig {
            organism_capacity: 1,
            ..EngineConfig::testing()
        })
        .unwrap();
        destination
            .add_file_teleporter(
                FileTeleporter::new(2, [500.0; 2], 100.0)
                    .with_paths(root.path(), "")
                    .with_polling(1, 1),
            )
            .unwrap();
        destination.apply_file_teleporters().unwrap();
        assert_eq!(destination.population(), 0);
        assert_eq!(destination.file_teleporters[0].received, 0);
        assert!(file.exists());
    }
}
