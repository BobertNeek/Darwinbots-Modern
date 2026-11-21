//! Offline file exchange uses complete native organism groups.

use super::*;
use crate::SaveFile;
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum FileTransferPhase {
    BeforeRobots,
    AfterPortalMovement,
}

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
        let mut shared_path_exports = HashMap::new();
        let result = self
            .apply_file_teleporters_staged(
                &mut transfers,
                FileTransferPhase::BeforeRobots,
                &mut shared_path_exports,
            )
            .and_then(|()| {
                self.apply_file_teleporters_staged(
                    &mut transfers,
                    FileTransferPhase::AfterPortalMovement,
                    &mut shared_path_exports,
                )
            })
            .and_then(|()| transfers.commit());
        if result.is_err() {
            self.restore_cycle_checkpoint(checkpoint);
        }
        result
    }

    pub(super) fn apply_file_teleporters_staged(
        &mut self,
        transfers: &mut FileTransferTransaction,
        phase: FileTransferPhase,
        shared_path_exports: &mut HashMap<u32, Vec<PathBuf>>,
    ) -> Result<(), EngineError> {
        let late = phase == FileTransferPhase::AfterPortalMovement;
        let native_world = self.native_cycle.is_some();
        if late && !native_world {
            return Ok(());
        }
        let mut gateways = if late {
            Vec::new()
        } else {
            self.file_teleporters
                .iter()
                .cloned()
                .map(|gateway| (gateway, None))
                .collect::<Vec<_>>()
        };
        for (index, tele) in self.teleporters.iter().enumerate() {
            if tele.internet
                || !(late && tele.inbound
                    || !late && (tele.outbound || (!native_world && tele.inbound)))
            {
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
            let native_contact =
                native_index.and_then(|index| self.teleporters[index].native_contact());
            let touches_outbound = |position, radius| {
                native_contact.map_or_else(
                    || touches_gateway(&settings, position, radius),
                    |contact| contact.touches(position, radius),
                )
            };
            let previous_guard = settings.backflow_guard.clone();
            let mut sent = 0_u64;
            let mut emitted_paths = if late && settings.inbound_path == settings.outbound_path {
                let mut paths = shared_path_exports.remove(&settings.id).unwrap_or_default();
                paths.retain(|path| path.parent() == Some(settings.inbound_path.as_path()));
                paths
            } else {
                Vec::new()
            };

            if !late && !settings.outbound_path.as_os_str().is_empty() {
                for id in self.slots.ids_in_slot_order() {
                    if native_index.is_none() && sent >= settings.capacity as u64 {
                        break;
                    }
                    let Ok(organism) = self.organism(id) else {
                        continue;
                    };
                    let robot = self.slots.get(id)?;
                    if !touches_outbound(organism.position, organism.radius)
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
                    if !touches_outbound(corpse.position, radius) || !settings.accepts(false, true)
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

            let separate_native_departures = native_world && native_index.is_some() && !late;
            let poll_due =
                if separate_native_departures || settings.inbound_path.as_os_str().is_empty() {
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
                    .filter(|path| path.is_file())
                    .filter(|path| {
                        if late {
                            // TeleportInBots compares the final three characters with "dbo".
                            path.file_name()
                                .is_some_and(|name| name.as_encoded_bytes().ends_with(b"dbo"))
                        } else {
                            path.extension()
                                .and_then(|extension| extension.to_str())
                                .is_some_and(|extension| extension.eq_ignore_ascii_case("dbo"))
                        }
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
                    let position = native_index.map_or(settings.center, |native_index| {
                        let portal = &self.teleporters[native_index];
                        if late && let Some(position) = portal.native_top_left {
                            // TeleportInBots reads the corrected top-left after MoveTeleporter.
                            [
                                (f64::from(position[0]) + f64::from(portal.width) / 2.0) as f32,
                                (f64::from(position[1]) + f64::from(portal.height) / 3.0) as f32,
                            ]
                        } else {
                            [settings.center[0], settings.center[1] - portal.height / 6.0]
                        }
                    });
                    if late {
                        // Loading a save would discard the current cycle's runtime state.
                        let checkpoint = self.cycle_checkpoint();
                        let tie_index = self.native_tie_index.clone();
                        if self
                            .apply_file_teleporter_payload(&bytes, position, false)
                            .is_ok_and(|count| count > 0)
                            && transfers.claim_input(&path, &bytes).is_ok()
                        {
                            self.native_buckets.synchronize(
                                [self.config.world_width, self.config.world_height],
                                &self.slots,
                            );
                            received = received.saturating_add(1);
                        } else {
                            self.restore_cycle_checkpoint(checkpoint);
                            self.native_tie_index = tie_index;
                        }
                        continue;
                    }

                    // Keep the existing transaction path for configured file exchange.
                    let mut candidate = SaveFile::decode(&SaveFile::encode(self)?)?;
                    let imported = candidate.apply_file_teleporter_payload(&bytes, position, true);
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
                if separate_native_departures {
                    // The early outputs become visible only when both passes commit.
                    shared_path_exports.insert(settings.id, emitted_paths);
                } else {
                    teleporter.backflow_guard = emitted_paths.pop();
                }
            }
            if let Some(native_index) = native_index {
                let native = &mut self.teleporters[native_index];
                native.sent = teleporter.sent;
                native.received = teleporter.received;
                native.poll_countdown = teleporter.poll_countdown;
                native.file_backflow_guard = teleporter.backflow_guard.clone();
            } else {
                self.file_teleporters[index] = teleporter.clone();
            }
        }
        Ok(())
    }

    fn apply_file_teleporter_payload(
        &mut self,
        bytes: &[u8],
        position: [f32; 2],
        publish: bool,
    ) -> Result<usize, EngineError> {
        if bytes.starts_with(b"DB2S") {
            let file = LegacySimulationFile::decode(bytes)?;
            for organism in &file.organisms {
                self.import_legacy_organism(organism, publish)?;
            }
            Ok(file.organisms.len())
        } else if bytes.starts_with(b"DB2O") {
            let file = LegacyOrganismFile::decode(bytes)?;
            self.apply_legacy_organism_file(&file, publish)
        } else if publish {
            self.import_native_organisms(bytes, Some(position))
        } else {
            let file = Self::decode_native_organisms(bytes, Some(position))?;
            self.apply_native_organisms(file, Some(position), false)
        }
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
    fn native_inbound_filename_matches_the_original_case_sensitive_suffix() {
        let source = Engine::from_native_sim(include_bytes!(
            "../../tests/fixtures/db24832-genuine-reproduction-cycle1-20261017.sim"
        ))
        .unwrap();
        let payload = source
            .export_native_organisms(&[source.snapshot().organisms[0].id])
            .unwrap();

        for (native, legacy, accepted, ignored) in [
            (true, true, "incomingdbo", "incoming.DBO"),
            (true, false, "incoming.DBO", "incomingdbo"),
            (false, true, "incoming.DBO", "incomingdbo"),
        ] {
            let root = tempdir().unwrap();
            let accepted = root.path().join(accepted);
            let ignored = root.path().join(ignored);
            let corrupt = root.path().join("broken.dbo");
            let foreign = root.path().join("foreign.txt");
            for path in [&accepted, &ignored, &foreign] {
                fs::write(path, &payload).unwrap();
            }
            fs::write(&corrupt, b"not an organism").unwrap();

            let config = EngineConfig::testing();
            let mut world = if native {
                Engine::new_native(config).unwrap()
            } else {
                Engine::new(config).unwrap()
            };
            if legacy {
                let mut portal = Teleporter::new(1, [500.0; 2], 100.0, [500.0; 2]);
                portal.local = false;
                portal.inbound = true;
                portal.path = root.path().to_path_buf();
                portal.capacity = 3;
                world.add_teleporter(portal).unwrap();
            } else {
                world
                    .add_file_teleporter(
                        FileTeleporter::new(1, [500.0; 2], 100.0)
                            .with_paths(root.path(), "")
                            .with_polling(1, 3),
                    )
                    .unwrap();
            }

            world.apply_file_teleporters().unwrap();
            assert!(
                !accepted.exists(),
                "{} should be consumed",
                accepted.display()
            );
            assert_eq!(fs::read(ignored).unwrap(), payload);
            assert_eq!(fs::read(foreign).unwrap(), payload);
            assert_eq!(fs::read(corrupt).unwrap(), b"not an organism");
            assert_eq!(world.population(), 1);
            let received = if legacy {
                world.teleporters[0].received
            } else {
                world.file_teleporters[0].received
            };
            assert_eq!(received, 1);
        }
    }

    #[test]
    fn legacy_internet_metadata_roundtrips_without_local_relocation_or_file_exchange() {
        let root = tempdir().unwrap();
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let id = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [500.0; 2])
            .unwrap();
        // Old saves can retain local/inbound/outbound defaults alongside Internet.
        let mut gateway = Teleporter::new(7, [500.0; 2], 100.0, [900.0; 2]);
        gateway.internet = true;
        gateway.local = true;
        gateway.inbound = true;
        gateway.outbound = true;
        gateway.inbound_path = root.path().to_path_buf();
        gateway.outbound_path = root.path().to_path_buf();
        world.add_teleporter(gateway.clone()).unwrap();
        // Mixed historical flags must remain inert even with supplied cache fields.
        gateway.native_top_left = Some([400.0; 2]);
        gateway.native_contact_center = Some([500.0; 2]);
        gateway.drift_horizontal = true;
        gateway.drift_vertical = true;
        world.teleporters[0] = gateway.clone();
        let saved = SaveFile::encode(&world).unwrap();
        let mut resumed = SaveFile::decode(&saved).unwrap();
        assert_eq!(SaveFile::encode(&resumed).unwrap(), saved);
        resumed.apply_teleporter_relocation();
        resumed.apply_file_teleporters().unwrap();
        resumed.move_native_teleporters();
        assert_eq!(resumed.organism(id).unwrap().position, [500.0; 2]);
        assert_eq!(resumed.teleporters[0], gateway);
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
        assert_eq!(SaveFile::encode(&resumed).unwrap(), saved);
    }

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
            .spawn_at(LegacyDna::parse("stop").unwrap(), [500.0, 260.0])
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
        assert_eq!(
            resumed.population(),
            1,
            "the virgin contact cache is still at the origin"
        );
        let radius = resumed.snapshot().organisms[0].radius;
        assert!(100.0 + radius < 240.0 && 100.0 + radius > 200.0);
        resumed.move_native_teleporters();
        assert_eq!(
            resumed.teleporters[0].native_contact_center,
            Some([500.0, 460.0])
        );
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
        assert_eq!(
            robot.kinematics.position,
            [500.0, (400.0 + 200.0 / 3.0) as f32]
        );
    }

    #[test]
    fn native_file_admission_follows_movement_and_uses_one_transfer_journal() {
        use crate::native_binary::NativeValue;

        let inbound = tempdir().unwrap();
        let outbound = tempdir().unwrap();
        let mut incoming = NativeRecordFile::decode(include_bytes!(
            "../../tests/fixtures/db24832-genuine-reproduction-cycle1-20261017.sim"
        ))
        .unwrap();
        incoming.tail.clear();
        let robot = &mut incoming.robots[0];
        robot
            .set_value("absolute_id", NativeValue::Long(888))
            .unwrap();
        robot.set_value("age", NativeValue::Long(73)).unwrap();
        robot.set_single("position_x", 5000.0).unwrap();
        robot.set_single("position_y", 5000.0).unwrap();
        robot.set_single("energy", 1500.25).unwrap();
        robot.set_single("actual_velocity_x", 17.125).unwrap();
        let payload = incoming.encode().unwrap();
        let input = inbound.path().join("arriving.dbo");
        fs::write(&input, &payload).unwrap();

        let mut world = Engine::new_native(EngineConfig {
            world_width: 10_000.0,
            world_height: 10_000.0,
            metabolism_cost: 7,
            sunlight_energy: 0,
            allow_cpu_fallback: false,
            ..EngineConfig::testing()
        })
        .unwrap();
        let departing = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [1000.0, 1000.0])
            .unwrap();
        let portal = |id, center: [f32; 2], receives: bool, path: &std::path::Path| {
            let mut tele = Teleporter::new(id, center, 100.0, center);
            tele.local = false;
            tele.inbound = receives;
            tele.outbound = !receives;
            tele.path = path.to_owned();
            let anchor = [center[0] - 100.0, center[1] - 100.0];
            tele.native_top_left = Some(anchor);
            tele.native_contact_center = Some(tele.native_contact_center_at(anchor));
            tele
        };
        let mut receiver = portal(1, [5000.0, 5000.0], true, inbound.path());
        receiver.drift_horizontal = true;
        receiver.drift_vertical = true;
        receiver.fixed_position = false;
        receiver.velocity = [8.0, 3.0];
        let old_anchor = receiver.native_top_left.unwrap();
        // The earlier combined pass forwarded arrivals through the next sender.
        world.teleporters = vec![
            receiver,
            portal(2, [5000.0, 5000.0], false, outbound.path()),
            portal(3, [1000.0, 1000.0], false, outbound.path()),
        ];

        let before = SaveFile::encode(&world).unwrap();
        // The publication check runs after both transfer stages, even on CPU.
        world.forced_sensing_error_for_tests = Some("late transfer rollback".into());
        assert!(matches!(world.tick(), Err(EngineError::Gpu(_))));
        assert_eq!(SaveFile::encode(&world).unwrap(), before);
        assert_eq!(fs::read(&input).unwrap(), payload);
        assert_eq!(fs::read_dir(inbound.path()).unwrap().count(), 1);
        assert_eq!(fs::read_dir(outbound.path()).unwrap().count(), 0);

        world.tick().unwrap();
        assert!(world.slots.get(departing).is_err());
        assert!(!input.exists());
        assert_eq!(world.teleporters[0].received, 1);
        assert_eq!(world.teleporters[0].poll_countdown, 1);
        assert_eq!(world.teleporters[1].sent, 0);
        assert_eq!(world.teleporters[2].sent, 1);
        assert_eq!(fs::read_dir(outbound.path()).unwrap().count(), 1);
        let anchor = world.teleporters[0].native_top_left.unwrap();
        assert_ne!(anchor, old_anchor);
        let arrival = world.slots.get(world.slots.ids_in_slot_order()[0]).unwrap();
        assert_eq!(arrival.identity.absolute_id, 888);
        assert_eq!(arrival.counters.age, 73);
        assert_eq!(arrival.materials.energy, 1500.25);
        assert_eq!(arrival.kinematics.actual_velocity[0], 17.125);
        assert_eq!(
            arrival.kinematics.position,
            [
                (f64::from(anchor[0]) + 100.0) as f32,
                (f64::from(anchor[1]) + 200.0 / 3.0) as f32,
            ]
        );

        world.tick().unwrap();
        assert_eq!(world.population(), 0);
        assert_eq!(world.teleporters[1].sent, 1);
        assert_eq!(world.teleporters[2].sent, 1);
        assert_eq!(fs::read_dir(outbound.path()).unwrap().count(), 2);
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
