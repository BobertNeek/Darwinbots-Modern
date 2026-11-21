use super::*;

/// The open Gene Activations tool's display state. It is not written to a save.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneActivationMeterSnapshot {
    pub watch_id: u64,
    pub focus: Option<OrganismId>,
    pub current: Vec<bool>,
    pub weighted: Vec<u8>,
    pub executions: u64,
}

impl GeneActivationMeterSnapshot {
    pub(super) fn record(&mut self, activations: &[bool]) {
        // The current VM numbers genes from one and reserves index zero.
        let genes = activations.get(1..).unwrap_or_default();
        if genes.len() != self.weighted.len() {
            self.weighted = vec![0; genes.len()];
        }
        self.current.clear();
        self.current.extend_from_slice(genes);
        for (weight, &active) in self.weighted.iter_mut().zip(genes) {
            // ActivForm.DrawDyn assigns back to a VB6 Integer. The only
            // fractions are fifths, so adding two before dividing rounds them.
            let target = if active { 100 } else { 0 };
            *weight = ((4 * u16::from(*weight) + target + 2) / 5) as u8;
        }
        self.executions = self.executions.saturating_add(1);
    }
}

impl Engine {
    pub(crate) fn configure_gene_activation_watch(
        &mut self,
        watch_id: u64,
        enabled: bool,
        focus: Option<OrganismId>,
    ) {
        let meter = &mut self.snapshot.gene_activation_meter;
        if !enabled {
            if meter
                .as_ref()
                .is_some_and(|meter| meter.watch_id == watch_id)
            {
                *meter = None;
            }
            return;
        }
        let meter = meter.get_or_insert_with(GeneActivationMeterSnapshot::default);
        if meter.watch_id != watch_id {
            *meter = GeneActivationMeterSnapshot {
                watch_id,
                ..Default::default()
            };
        }
        if meter.focus != focus {
            meter.focus = focus;
            meter.current.clear();
        }
    }

    pub(crate) fn inherit_gene_activation_watch(&mut self, previous: &Self) {
        self.snapshot
            .gene_activation_meter
            .clone_from(&previous.snapshot.gene_activation_meter);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SaveFile;

    fn setup() -> (Engine, OrganismId) {
        let mut world = Engine::new_native(EngineConfig::testing()).unwrap();
        let focus = spawn(&mut world, "start 1 100 store stop", [500.0, 500.0]);
        (world, focus)
    }

    fn spawn(world: &mut Engine, source: &str, position: [f32; 2]) -> OrganismId {
        world
            .spawn_at(LegacyDna::parse(source).unwrap(), position)
            .unwrap()
    }

    fn meter(world: &Engine) -> &GeneActivationMeterSnapshot {
        world.snapshot().gene_activation_meter.as_ref().unwrap()
    }

    #[test]
    fn focused_meter_applies_every_execution_in_batches_longer_than_history() {
        let (mut batch, focus) = setup();
        let mut serial = batch.fork_cpu_command_candidate().unwrap().unwrap();
        batch.configure_gene_activation_watch(1, true, Some(focus));
        serial.configure_gene_activation_watch(1, true, Some(focus));
        batch.tick_many(70).unwrap();
        for _ in 0..70 {
            serial.tick().unwrap();
        }
        assert_eq!(batch.organism(focus).unwrap().activation_history.len(), 32);
        assert_eq!(meter(&batch), meter(&serial));
        assert_eq!(meter(&batch).executions, 70);
        assert_eq!(meter(&batch).weighted, [98]);
    }

    #[test]
    fn focused_meter_follows_executions_and_focus_without_observing_saves_or_refreshes() {
        let (mut world, focus) = setup();
        let idle = spawn(
            &mut world,
            "cond 0 1 > start 1 100 store stop",
            [800.0, 500.0],
        );
        let two = spawn(
            &mut world,
            "start 1 100 store stop start 1 101 store stop",
            [750.0, 800.0],
        );
        let save = SaveFile::encode(&world).unwrap();
        world.configure_gene_activation_watch(3, true, Some(focus));
        assert_eq!(SaveFile::encode(&world).unwrap(), save);
        assert!(
            SaveFile::decode(&save)
                .unwrap()
                .snapshot()
                .gene_activation_meter
                .is_none()
        );

        world.execute_robots().unwrap();
        assert_eq!(meter(&world).weighted, [20]);
        world.publish_snapshot();
        assert_eq!(meter(&world).executions, 1);
        world.execute_robots().unwrap();
        assert_eq!(world.snapshot().tick, 0);
        assert_eq!(meter(&world).weighted, [36]);
        assert_eq!(meter(&world).executions, 2);

        world.configure_gene_activation_watch(3, true, None);
        world.execute_robots().unwrap();
        assert!(meter(&world).current.is_empty());
        assert_eq!(meter(&world).weighted, [36]);
        world.configure_gene_activation_watch(3, true, Some(idle));
        world.execute_robots().unwrap();
        assert_eq!(meter(&world).current, [false]);
        assert_eq!(meter(&world).weighted, [29]);

        world.configure_gene_activation_watch(3, true, Some(two));
        world.slots.get_mut(two).unwrap().flags.can_execute_dna = false;
        world.execute_robots().unwrap();
        assert_eq!(meter(&world).executions, 3);
        world.slots.get_mut(two).unwrap().flags.can_execute_dna = true;
        world.execute_robots().unwrap();
        assert_eq!(meter(&world).weighted, [20, 20]);

        let checkpoint = world.cycle_checkpoint();
        world.execute_robots().unwrap();
        world.restore_cycle_checkpoint(checkpoint);
        assert_eq!(meter(&world).weighted, [20, 20]);
        assert_eq!(meter(&world).executions, 4);

        world.configure_gene_activation_watch(4, true, Some(two));
        world.configure_gene_activation_watch(3, false, None);
        assert_eq!(meter(&world).watch_id, 4);
        assert!(meter(&world).weighted.is_empty());
        world.configure_gene_activation_watch(4, false, None);
        assert!(world.snapshot().gene_activation_meter.is_none());
    }
}
