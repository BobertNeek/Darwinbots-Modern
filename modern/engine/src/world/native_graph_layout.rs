use super::*;
use crate::native_binary::{MAX_TEXT_BYTES, NativeSimulationTail};

const QUERY_COUNT: u8 = 3;

#[derive(Debug, Serialize)]
pub(crate) struct NativeGraphLayoutState {
    graphs: Vec<NativeGraphLayoutRow>,
    custom_queries: [String; QUERY_COUNT as usize],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct NativeGraphLayoutRow {
    pub(crate) graph_index: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) visible: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) left: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) top: Option<i32>,
}

impl NativeGraphLayoutRow {
    fn merge(&mut self, update: Self) {
        self.visible = update.visible.or(self.visible);
        self.left = update.left.or(self.left);
        self.top = update.top.or(self.top);
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct NativeGraphCustomQuery {
    pub(crate) slot: u8,
    pub(crate) expression: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct NativeGraphLayoutOverlay {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) graphs: Vec<NativeGraphLayoutRow>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) custom_queries: Vec<NativeGraphCustomQuery>,
}

impl NativeGraphLayoutOverlay {
    pub(super) fn validate(&self) -> Result<(), EngineError> {
        let mut graphs = [false; super::native_graph_recording::GRAPH_COUNT as usize];
        for row in &self.graphs {
            let Some(seen) = graphs.get_mut(row.graph_index as usize) else {
                return Err(EngineError::Invariant(
                    "original graph index must be 0 through 17".to_owned(),
                ));
            };
            if std::mem::replace(seen, true) {
                return Err(EngineError::Invariant(
                    "original graph layout updates require unique graph indices".to_owned(),
                ));
            }
        }

        let mut queries = [false; QUERY_COUNT as usize];
        for query in &self.custom_queries {
            if !(1..=QUERY_COUNT).contains(&query.slot)
                || std::mem::replace(&mut queries[query.slot as usize - 1], true)
            {
                return Err(EngineError::Invariant(
                    "original custom graph queries require unique slots from 1 through 3"
                        .to_owned(),
                ));
            }
            if super::native_export::ansi(&query.expression)?.len() > MAX_TEXT_BYTES {
                return Err(EngineError::Invariant(
                    "original custom graph query exceeds native text size".to_owned(),
                ));
            }
        }
        Ok(())
    }
}

impl Engine {
    pub(crate) fn read_native_graph_layout(&self) -> Result<NativeGraphLayoutState, EngineError> {
        let mut state = NativeGraphLayoutState {
            graphs: (0..super::native_graph_recording::GRAPH_COUNT)
                .map(|graph_index| NativeGraphLayoutRow {
                    graph_index,
                    visible: Some(false),
                    left: None,
                    top: None,
                })
                .collect(),
            custom_queries: Default::default(),
        };
        if let Some(origin) = self
            .native_origin
            .as_ref()
            .filter(|file| !file.tail.is_empty())
        {
            let tail = NativeSimulationTail::decode(&origin.tail)?;
            for graph in &mut state.graphs {
                let index = graph.graph_index;
                graph.visible = Some(tail.integer(&format!("graph[{index}].visible"))? != 0);
                graph.left = Some(tail.integer(&format!("graph[{index}].left"))?);
                graph.top = Some(tail.integer(&format!("graph[{index}].top"))?);
            }
            for (index, query) in state.custom_queries.iter_mut().enumerate() {
                *query = super::native_io::text(tail.bytes(&format!("graph_query{}", index + 1))?);
            }
        }
        if let Some(layout) = &self.native_graph_layout {
            for row in &layout.graphs {
                state.graphs[row.graph_index as usize].merge(*row);
            }
            for query in &layout.custom_queries {
                state.custom_queries[query.slot as usize - 1].clone_from(&query.expression);
            }
        }
        Ok(state)
    }

    pub(crate) fn update_native_graph_layout(
        &mut self,
        graphs: Vec<NativeGraphLayoutRow>,
        custom_queries: Vec<NativeGraphCustomQuery>,
    ) -> Result<(), EngineError> {
        let mut patch = NativeGraphLayoutOverlay {
            graphs,
            custom_queries,
        };
        patch.validate()?;
        patch
            .graphs
            .retain(|row| row.visible.is_some() || row.left.is_some() || row.top.is_some());
        if patch.graphs.is_empty() && patch.custom_queries.is_empty() {
            return Ok(());
        }
        let layout = self
            .native_graph_layout
            .get_or_insert_with(Default::default);
        for row in patch.graphs {
            if let Some(existing) = layout
                .graphs
                .iter_mut()
                .find(|existing| existing.graph_index == row.graph_index)
            {
                existing.merge(row);
            } else {
                layout.graphs.push(row);
            }
        }
        for query in patch.custom_queries {
            if let Some(existing) = layout
                .custom_queries
                .iter_mut()
                .find(|existing| existing.slot == query.slot)
            {
                *existing = query;
            } else {
                layout.custom_queries.push(query);
            }
        }
        layout.graphs.sort_unstable_by_key(|row| row.graph_index);
        layout
            .custom_queries
            .sort_unstable_by_key(|query| query.slot);
        Ok(())
    }
}
