//! Console execution samples from a single native advance request.
use super::*;

#[derive(Debug, PartialEq, Eq, Serialize)]
pub(crate) struct NativeConsoleGeneReport {
    focus: OrganismId,
    unique_id: u64,
    tick: u64,
    activations: Vec<bool>,
}

pub(super) struct NativeConsoleGeneCapture {
    focuses: HashMap<OrganismId, u64>,
    executed: Vec<NativeConsoleGeneReport>,
    terminal: Vec<NativeConsoleGeneReport>,
}

impl NativeConsoleGeneCapture {
    pub(super) fn record(
        &mut self,
        focus: OrganismId,
        unique_id: u64,
        tick: u64,
        activations: &[bool],
    ) {
        if self.focuses.get(&focus) == Some(&unique_id) {
            self.executed.push(NativeConsoleGeneReport {
                focus,
                unique_id,
                tick,
                activations: activations.to_vec(),
            });
        }
    }
}

impl Engine {
    pub(crate) fn with_native_console_gene_capture<T>(
        &mut self,
        focuses: Vec<OrganismId>,
        advance: impl FnOnce(&mut Self) -> Result<T, EngineError>,
    ) -> Result<(T, Option<Vec<NativeConsoleGeneReport>>), EngineError> {
        if focuses.is_empty() || self.native_cycle.is_none() || self.native_random.is_none() {
            return advance(self).map(|value| (value, None));
        }
        self.native_console_gene_capture = Some(NativeConsoleGeneCapture {
            focuses: focuses
                .into_iter()
                .filter_map(|focus| {
                    self.slots
                        .get(focus)
                        .ok()
                        .map(|actor| (focus, actor.identity.absolute_id))
                })
                .collect(),
            executed: Vec::new(),
            terminal: Vec::new(),
        });
        let result = advance(self);
        let capture = self.native_console_gene_capture.take();
        result.map(|value| (value, capture.map(|capture| capture.terminal)))
    }

    pub(super) fn finish_native_console_gene_cycle(&mut self) {
        let Some(capture) = self.native_console_gene_capture.as_mut() else {
            return;
        };
        let slots = &self.slots;
        capture.focuses.retain(|focus, unique_id| {
            slots
                .get(*focus)
                .is_ok_and(|actor| actor.identity.absolute_id == *unique_id)
        });
        // Live actors already carry their own activation history. Only the
        // frames that disappear with their actors need to leave this cycle.
        let remaining = &capture.focuses;
        capture.terminal.extend(
            capture
                .executed
                .drain(..)
                .filter(|report| !remaining.contains_key(&report.focus)),
        );
    }
}
