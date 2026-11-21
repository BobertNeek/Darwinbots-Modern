use super::*;
use crate::native_binary::{MAX_TEXT_BYTES, NativeSimulationTail};

pub(super) const GRAPH_COUNT: u8 = 18;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct NativeGraphRecordingState {
    pub(crate) run_stamp: String,
    pub(crate) graphs: Vec<NativeGraphRecordingRow>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct NativeGraphRecordingRow {
    pub(crate) graph_index: u8,
    pub(crate) enabled: bool,
    pub(crate) counter: i32,
}

fn validate_rows(graphs: &[NativeGraphRecordingRow]) -> Result<(), EngineError> {
    let mut seen = [false; GRAPH_COUNT as usize];
    for graph in graphs {
        let Some(present) = seen.get_mut(graph.graph_index as usize) else {
            return Err(EngineError::Invariant(
                "original graph index must be 0 through 17".to_owned(),
            ));
        };
        if std::mem::replace(present, true) || graph.counter < 0 {
            return Err(EngineError::Invariant(
                "original graph updates require unique indices and nonnegative counters".to_owned(),
            ));
        }
    }
    Ok(())
}

fn validate_stamp(run_stamp: &str) -> Result<(), EngineError> {
    if run_stamp.is_empty() || run_stamp.len() > MAX_TEXT_BYTES {
        return Err(EngineError::Invariant(
            "original graph recording requires a nonempty native run stamp".to_owned(),
        ));
    }
    super::native_export::ansi(run_stamp)?;
    Ok(())
}

impl NativeGraphRecordingState {
    pub(super) fn validate(&self) -> Result<(), EngineError> {
        if !self.run_stamp.is_empty() {
            validate_stamp(&self.run_stamp)?;
        }
        validate_rows(&self.graphs)
    }
}

impl Engine {
    pub(crate) fn read_native_graph_recording(
        &self,
    ) -> Result<NativeGraphRecordingState, EngineError> {
        let mut state = self.native_graph_recording.clone().unwrap_or_default();
        if let Some(origin) = self.native_origin.as_ref().filter(|f| !f.tail.is_empty()) {
            let tail = NativeSimulationTail::decode(&origin.tail)?;
            if state.run_stamp.is_empty() {
                state.run_stamp = super::native_io::text(tail.bytes("sim_start")?);
            }
            for graph_index in 0..GRAPH_COUNT {
                if state
                    .graphs
                    .iter()
                    .any(|row| row.graph_index == graph_index)
                {
                    continue;
                }
                let enabled = tail.integer(&format!("graph[{graph_index}].save"))? != 0;
                let counter = tail.integer(&format!("graph[{graph_index}].counter"))?;
                if enabled || counter != 0 {
                    state.graphs.push(NativeGraphRecordingRow {
                        graph_index,
                        enabled,
                        counter,
                    });
                }
            }
        }
        state.graphs.sort_unstable_by_key(|row| row.graph_index);
        Ok(state)
    }

    pub(crate) fn update_native_graph_recording(
        &mut self,
        run_stamp: Option<String>,
        graphs: Vec<NativeGraphRecordingRow>,
    ) -> Result<(), EngineError> {
        if let Some(stamp) = run_stamp.as_deref() {
            validate_stamp(stamp)?;
        }
        validate_rows(&graphs)?;
        if run_stamp.is_none() && graphs.is_empty() {
            return Ok(());
        }
        let state = self
            .native_graph_recording
            .get_or_insert_with(Default::default);
        if let Some(stamp) = run_stamp {
            state.run_stamp = stamp;
        }
        state.graphs.retain(|current| {
            !graphs
                .iter()
                .any(|row| row.graph_index == current.graph_index)
        });
        state.graphs.extend(graphs);
        state.graphs.sort_unstable_by_key(|row| row.graph_index);
        Ok(())
    }
}
