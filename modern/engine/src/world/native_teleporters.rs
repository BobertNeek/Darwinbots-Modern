use super::{Engine, EngineError, Teleporter, world_random_unit};
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
        let center = [
            top_left(self.config.world_width, width) + width / 2.0,
            top_left(self.config.world_height, height) + height / 2.0,
        ];

        let mut teleporter = Teleporter::new(id, center, width / 2.0, center);
        teleporter.width = width;
        teleporter.height = height;
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BackendPreference, EngineConfig, TeleporterFilters};

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
}
