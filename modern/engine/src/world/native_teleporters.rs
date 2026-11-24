use super::{Engine, EngineError, Teleporter, world_random_unit};
use crate::physics::{native_magnitude, native_scalar};
use crate::{NativeTeleporterKind, NativeTeleporterOptions};

impl Engine {
    /// Teleport.bas NewTeleporter, as called by the original New Teleporter form.
    pub fn add_native_teleporter(
        &mut self,
        options: NativeTeleporterOptions,
    ) -> Result<u32, EngineError> {
        if self.teleporters.len() >= 10 {
            return Err(EngineError::Invariant(
                "the original teleporter limit of 10 has been reached".into(),
            ));
        }
        if !(100..=1_000).contains(&options.size) {
            return Err(EngineError::Invariant(
                "teleporter size must be between 100 and 1000".into(),
            ));
        }
        if !(1..32_000).contains(&options.poll_interval) || !(1..32_000).contains(&options.capacity)
        {
            return Err(EngineError::Invariant(
                "teleporter poll interval and capacity must be between 1 and 31999".into(),
            ));
        }

        let height = options.size as f32;
        let width = height * (self.config.world_height / self.config.world_width);
        if !width.is_finite() || width / 2.0 <= 0.0 {
            return Err(EngineError::Invariant(
                "teleporter geometry is invalid for this field".into(),
            ));
        }
        let id = (1..=u32::MAX)
            .find(|id| {
                self.teleporters.iter().all(|value| value.id != *id)
                    && self.file_teleporters.iter().all(|value| value.id != *id)
            })
            .ok_or_else(|| EngineError::Invariant("no teleporter ID is available".into()))?;

        let mut fallback_random = self.config.seed ^ self.tick.rotate_left(23) ^ u64::from(id);
        let mut top_left = |field_extent: f32, dimension: f32| {
            // Common.bas Random(0, field extent - size) uses inclusive Single arithmetic.
            let draw = world_random_unit(&mut self.native_random, &mut fallback_random) as f32;
            ((field_extent - dimension + 1.0) * draw).floor()
        };
        let position = [
            top_left(self.config.world_width, width),
            top_left(self.config.world_height, height),
        ];
        let center = [position[0] + width / 2.0, position[1] + height / 2.0];

        let mut teleporter = Teleporter::new(id, center, width / 2.0, center);
        teleporter.width = width;
        teleporter.height = height;
        teleporter.native_top_left = Some(position);
        // NewTeleporter leaves the virgin slot's center at zero until MoveTeleporter.
        teleporter.native_contact_center = Some([0.0; 2]);
        teleporter.color = 0xffffffff;
        teleporter.local = options.kind == NativeTeleporterKind::Intrasim;
        teleporter.inbound = options.kind == NativeTeleporterKind::Inbound;
        teleporter.outbound = options.kind == NativeTeleporterKind::Outbound;
        teleporter.random_destination = teleporter.local;
        if !teleporter.local {
            teleporter.path = options.path;
        }
        teleporter.fixed_position = options.fixed_position;
        teleporter.drift_horizontal = !options.fixed_position;
        teleporter.drift_vertical = !options.fixed_position;
        teleporter.filters = options.filters;
        teleporter.poll_interval = options.poll_interval;
        teleporter.capacity = options.capacity as usize;
        teleporter.poll_countdown = options.capacity;
        self.teleporters.push(teleporter);
        self.publish_snapshot();
        Ok(id)
    }

    pub(super) fn move_native_teleporters(&mut self) {
        if self.native_cycle.is_some_and(|cycle| cycle < 0) {
            return;
        }
        let field = [self.config.world_width, self.config.world_height];
        let wrap = self.config.axis_topology();
        let max_velocity = self.config.physics.max_velocity.max(0.0);
        let speed_limit = max_velocity / 4.0;
        let rebound = (f64::from(max_velocity) * 0.1) as f32;
        let mut fallback_random = self.config.seed ^ self.tick.rotate_left(23);

        // Teleport.bas UpdateTeleporters calls Drift and Move in slot order.
        for teleporter in &mut self.teleporters {
            let Some(mut position) = teleporter.native_top_left.filter(|_| !teleporter.internet)
            else {
                continue;
            };
            let drift = [teleporter.drift_horizontal, teleporter.drift_vertical];
            for (velocity, enabled) in teleporter.velocity.iter_mut().zip(drift) {
                if enabled {
                    *velocity += world_random_unit(&mut self.native_random, &mut fallback_random)
                        as f32
                        - 0.5;
                }
            }
            let speed = native_magnitude(teleporter.velocity);
            if speed > speed_limit {
                teleporter.velocity = native_scalar(teleporter.velocity, speed_limit / speed);
            }

            let size = [teleporter.width, teleporter.height];
            if drift == [true; 2] {
                position[0] += teleporter.velocity[0];
                position[1] += teleporter.velocity[1];
            }
            // MoveTeleporter stores this before the rectangle wraps or bounces.
            teleporter.native_contact_center = Some(teleporter.native_contact_center_at(position));
            for axis in 0..2 {
                let velocity = &mut teleporter.velocity[axis];
                correct_teleporter_edge(
                    &mut position[axis],
                    velocity,
                    size[axis],
                    field[axis],
                    wrap[axis],
                    rebound,
                );
                teleporter.center[axis] = position[axis] + size[axis] / 2.0;
            }
            teleporter.native_top_left = Some(position);
        }
    }
}

impl Teleporter {
    pub(super) fn retain_native_movement(&mut self, previous: &Self) -> Result<(), EngineError> {
        self.native_top_left = previous.native_top_left;
        self.native_contact_center = previous
            .native_contact_center
            .or_else(|| previous.native_contact().map(|contact| contact.center));
        if self.native_top_left.is_none() {
            return Ok(());
        }
        if !self.width.is_finite()
            || !self.height.is_finite()
            || self.width <= 0.0
            || self.height <= 0.0
            || self.velocity.iter().any(|value| !value.is_finite())
        {
            return Err(EngineError::Invariant(
                "native teleporter geometry or velocity is invalid".into(),
            ));
        }
        if self.center != previous.center
            || self.width != previous.width
            || self.height != previous.height
        {
            self.native_top_left = Some([
                self.center[0] - self.width / 2.0,
                self.center[1] - self.height / 2.0,
            ]);
        }
        if self.fixed_position != previous.fixed_position {
            self.drift_horizontal = !self.fixed_position;
            self.drift_vertical = !self.fixed_position;
            if self.fixed_position {
                self.velocity = [0.0; 2];
            }
        }
        Ok(())
    }
}

fn correct_teleporter_edge(
    position: &mut f32,
    velocity: &mut f32,
    size: f32,
    field: f32,
    wrap: bool,
    rebound: f32,
) {
    let span = f64::from(field) - f64::from(size);
    if *position < 0.0 {
        if f64::from(*position) + f64::from(size) < 0.0 {
            *position = 0.0;
        }
        if wrap {
            *position = (f64::from(*position) + f64::from(field) - f64::from(size)) as f32;
        } else {
            *velocity = rebound;
        }
    }
    if f64::from(*position) + f64::from(size) > f64::from(field) {
        if *position > field {
            *position = span as f32;
        }
        if wrap {
            *position = (f64::from(*position) - span) as f32;
        } else {
            *velocity = -rebound;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::inside_teleporter;
    use crate::{BackendPreference, EngineConfig, SaveFile, TeleporterFilters};

    #[test]
    fn original_teleporter_creation_uses_native_rng_and_legacy_fields() {
        let mut engine = Engine::new_native(EngineConfig {
            backend: BackendPreference::Cpu,
            world_width: 2000.0,
            world_height: 1000.0,
            ..EngineConfig::default()
        })
        .unwrap();
        let options = NativeTeleporterOptions {
            kind: NativeTeleporterKind::Intrasim,
            size: 300,
            path: "ignored".into(),
            fixed_position: false,
            filters: TeleporterFilters {
                teleport_vegetables: true,
                teleport_corpses: false,
                teleport_heterotrophs: true,
            },
            poll_interval: 3,
            capacity: 12,
        };
        let mut expected_rng = engine.native_random.clone().unwrap();
        let x = expected_rng.integer(0, 1850);
        let y = expected_rng.integer(0, 700);
        assert_eq!(engine.add_native_teleporter(options.clone()).unwrap(), 1);
        assert_eq!(engine.native_random.as_ref(), Some(&expected_rng));
        let teleporter = &engine.teleporters[0];
        assert_eq!(teleporter.native_top_left, Some([x as f32, y as f32]));
        assert_eq!(teleporter.native_contact_center, Some([0.0; 2]));
        assert_eq!(teleporter.center, [x as f32 + 75.0, y as f32 + 150.0]);
        assert_eq!([teleporter.width, teleporter.height], [150.0, 300.0]);
        assert_eq!(teleporter.radius, 75.0);
        assert_eq!(teleporter.color, 0xffffffff);
        assert!(teleporter.local && teleporter.random_destination);
        assert!(!teleporter.inbound && !teleporter.outbound);
        assert!(teleporter.path.as_os_str().is_empty());
        assert!(
            !teleporter.fixed_position && teleporter.drift_horizontal && teleporter.drift_vertical
        );
        assert_eq!(teleporter.filters, options.filters);
        assert_eq!(
            (
                teleporter.poll_interval,
                teleporter.capacity,
                teleporter.poll_countdown
            ),
            (3, 12, 12)
        );
        assert_eq!((teleporter.sent, teleporter.received), (0, 0));

        for id in 2..=10 {
            assert_eq!(engine.add_native_teleporter(options.clone()).unwrap(), id);
        }
        let before_rejection = engine.native_random.clone();
        assert!(engine.add_native_teleporter(options).is_err());
        assert_eq!(engine.native_random, before_rejection);
        assert_eq!(engine.teleporters.len(), 10);
    }

    fn native_portal(
        id: u32,
        position: [f32; 2],
        size: [f32; 2],
        velocity: [f32; 2],
        drift: [bool; 2],
    ) -> Teleporter {
        let center = [position[0] + size[0] / 2.0, position[1] + size[1] / 2.0];
        Teleporter {
            width: size[0],
            height: size[1],
            native_top_left: Some(position),
            velocity,
            drift_horizontal: drift[0],
            drift_vertical: drift[1],
            fixed_position: drift == [false; 2],
            ..Teleporter::new(id, center, size[0] / 2.0, center)
        }
    }

    #[test]
    fn native_teleporter_contact_uses_strict_robot_radius_and_old_checkpoint_fallback() {
        let mut portal = native_portal(1, [90.0, 60.0], [20.0, 100.0], [0.0; 2], [false; 2]);
        // #83 checkpoints only contained a corrected top-left and renderer center.
        assert_eq!(portal.native_contact_center, None);
        assert_eq!(portal.center, [100.0, 110.0]);
        assert_eq!(portal.native_contact().unwrap().center, [100.0, 90.0]);
        assert!(!inside_teleporter(&portal, [114.0, 90.0], 0.0));
        assert!(inside_teleporter(&portal, [114.0, 90.0], 5.0));
        assert!(!inside_teleporter(&portal, [115.0, 90.0], 5.0));

        portal.native_top_left = None;
        assert!(inside_teleporter(&portal, [110.0, 110.0], 0.0));
        assert!(!inside_teleporter(&portal, [111.0, 110.0], 5.0));
    }

    #[test]
    fn native_teleporter_contact_keeps_pre_wrap_center_through_checkpoint_and_edit() {
        let mut engine = Engine::new_native(EngineConfig {
            world_width: 400.0,
            world_height: 200.0,
            axis_topology: Some([true, false]),
            ..EngineConfig::testing()
        })
        .unwrap();
        let mut portal = native_portal(1, [352.0, 96.0], [50.0, 80.0], [0.0; 2], [false; 2]);
        portal.native_contact_center = Some([0.0; 2]);
        assert!(inside_teleporter(&portal, [14.0, 16.0], 0.0));
        assert!(!inside_teleporter(&portal, [377.0, 120.0], 0.0));
        engine.teleporters.push(portal);

        engine.native_cycle = Some(0);
        engine.move_native_teleporters();
        let moved = &engine.teleporters[0];
        assert_eq!(moved.native_top_left, Some([2.0, 96.0]));
        assert_eq!(moved.center, [27.0, 136.0]);
        assert_eq!(moved.native_contact_center, Some([377.0, 120.0]));
        let mut resumed = SaveFile::decode(&SaveFile::encode(&engine).unwrap()).unwrap();
        assert_eq!(resumed.teleporters[0], *moved);
        let edit = Teleporter {
            center: [27.0, 138.0],
            native_contact_center: None, // The desktop sends only authored geometry.
            ..moved.clone()
        };
        resumed.update_teleporter(edit).unwrap();
        let cached = &resumed.teleporters[0];
        assert!(inside_teleporter(cached, [377.0, 120.0], 0.0));
        assert!(!inside_teleporter(cached, [27.0, 122.0], 0.0));

        resumed.native_cycle = Some(1);
        resumed.move_native_teleporters();
        assert!(inside_teleporter(
            &resumed.teleporters[0],
            [27.0, 122.0],
            0.0
        ));
        assert!(!inside_teleporter(
            &resumed.teleporters[0],
            [377.0, 120.0],
            0.0
        ));
    }

    #[test]
    fn native_teleporter_drift_matches_slot_draws_rectangular_wrap_and_bounce() {
        let mut engine = Engine::new_native(EngineConfig {
            world_width: 400.0,
            world_height: 200.0,
            axis_topology: Some([true, false]),
            ..EngineConfig::testing()
        })
        .unwrap();
        engine.config.physics.max_velocity = 20.0;
        engine.native_random = Some(crate::native_random::NativeRandom::default());
        let mut portable = native_portal(2, [100.0; 2], [20.0; 2], [3.0; 2], [true; 2]);
        portable.native_top_left = None;
        let mut internet = native_portal(1, [100.0; 2], [20.0; 2], [3.0; 2], [true; 2]);
        internet.internet = true;
        engine.teleporters = vec![
            native_portal(30, [349.5, 180.0], [50.0, 30.0], [12.0, 11.0], [true; 2]),
            portable.clone(),
            native_portal(20, [10.0, -31.0], [90.0, 30.0], [0.5, -6.0], [true, false]),
            internet.clone(),
            native_portal(10, [-2.0, 10.0], [20.0, 35.0], [-0.1, 0.35], [false, true]),
            native_portal(9, [15.0, 206.0], [20.0, 17.0], [0.0; 2], [false; 2]),
        ];

        let before = engine.teleporters.clone();
        engine.move_native_teleporters(); // Native startup still has cycle -1.
        assert_eq!(engine.teleporters, before);
        assert_eq!(engine.native_random.as_ref().unwrap().state(), 0x50000);

        engine.native_cycle = Some(0);
        engine.move_native_teleporters();
        let portals = &engine.teleporters;
        // Single values from Common.bas Rnd/VectorMagnitude and Teleport.bas.
        assert_eq!(
            portals[0].native_top_left.unwrap().map(f32::to_bits),
            [0x404d6280, 0x43375a5b]
        );
        assert_eq!(
            portals[0].velocity.map(f32::to_bits),
            [0x406d629b, 0xc0000000]
        );
        assert_eq!(portals[1], portable);
        assert_eq!(portals[2].native_top_left, Some([10.0, 0.0]));
        assert_eq!(
            portals[2].velocity.map(f32::to_bits),
            [0x3ef61dae, 0x40000000]
        );
        assert_eq!(portals[3], internet);
        assert_eq!(portals[4].native_top_left, Some([378.0, 10.0]));
        assert_eq!(
            portals[4].velocity.map(f32::to_bits),
            [0xbdcccccd, 0x3e0ee976]
        );
        assert_eq!(portals[5].native_top_left, Some([15.0, 183.0]));
        assert_eq!(portals[5].velocity, [0.0, -2.0]);
        assert_eq!(portals[5].center, [25.0, 191.5]);
        assert_eq!(engine.native_random.as_ref().unwrap().state(), 0x4a20c4);
    }

    #[test]
    fn native_teleporter_movement_roundtrips_exact_origin_and_honors_fixed_edits() {
        let mut engine = Engine::new_native(EngineConfig::testing()).unwrap();
        let position = [0.000001, 10.000001];
        let portal = native_portal(1, position, [50.0, 30.0], [0.0; 2], [false; 2]);
        engine.teleporters.push(portal.clone());

        let legacy = Engine::from_native_sim(&engine.export_native_sim().unwrap()).unwrap();
        assert_eq!(legacy.teleporters[0].native_top_left, Some(position));
        assert_ne!(legacy.teleporters[0].center[0] - 25.0, position[0]);

        let edit = Teleporter {
            fixed_position: false,
            native_top_left: None, // The desktop only sends center and rectangle dimensions.
            velocity: [3.0, 4.0],
            ..portal
        };
        engine.update_teleporter(edit).unwrap();
        assert_eq!(engine.teleporters[0].native_top_left, Some(position));
        assert!(engine.teleporters[0].drift_horizontal && engine.teleporters[0].drift_vertical);

        let mut resumed = SaveFile::decode(&SaveFile::encode(&engine).unwrap()).unwrap();
        engine.tick().unwrap();
        resumed.tick().unwrap();
        let moved = &engine.snapshot().teleporters[0];
        assert_ne!(moved.native_top_left, Some(position));
        assert_eq!(moved, &resumed.snapshot().teleporters[0]);
        assert_eq!(
            SaveFile::encode(&engine).unwrap(),
            SaveFile::encode(&resumed).unwrap()
        );

        let fixed_position = moved.native_top_left;
        let edit = Teleporter {
            fixed_position: true,
            ..moved.clone()
        };
        engine.update_teleporter(edit).unwrap();
        engine.tick().unwrap();
        let stopped = &engine.snapshot().teleporters[0];
        assert_eq!(stopped.native_top_left, fixed_position);
        assert_eq!(stopped.velocity, [0.0; 2]);
        assert!(!stopped.drift_horizontal && !stopped.drift_vertical);
    }
}
