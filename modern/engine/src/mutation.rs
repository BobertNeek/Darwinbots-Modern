use crate::dna::normalize_master_end;
use crate::{DnaBlock, LegacyDna, MAX_DNA_BLOCKS, TokenClass};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MutationKind {
    Point,
    Insertion,
    Deletion,
    Duplication,
    Replacement,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MutationReport {
    pub changes: u32,
    pub kind: Option<MutationKind>,
}

pub struct PointMutator {
    random_state: u64,
}

impl PointMutator {
    pub fn new(seed: u64) -> Self {
        Self {
            random_state: seed.max(1),
        }
    }

    pub fn random_state(&self) -> u64 {
        self.random_state
    }

    pub fn mutate(&mut self, dna: &mut LegacyDna) -> MutationReport {
        let numeric_positions = dna
            .blocks()
            .iter()
            .enumerate()
            .filter_map(|(index, block)| (block.class == TokenClass::Number).then_some(index))
            .collect::<Vec<_>>();
        if numeric_positions.is_empty() {
            return MutationReport::default();
        }
        let position = numeric_positions[(self.next() as usize) % numeric_positions.len()];
        let mut delta = (self.next() % 21) as i32 - 10;
        if delta == 0 {
            delta = 1;
        }
        dna.blocks_mut()[position].value = dna.blocks()[position].value.saturating_add(delta);
        MutationReport {
            changes: 1,
            kind: Some(MutationKind::Point),
        }
    }

    fn next(&mut self) -> u64 {
        let mut value = self.random_state;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.random_state = value.max(1);
        value
    }
}

pub struct GenomeMutator {
    random_state: u64,
}

impl GenomeMutator {
    pub fn new(seed: u64) -> Self {
        Self {
            random_state: seed.max(1),
        }
    }

    pub fn random_state(&self) -> u64 {
        self.random_state
    }

    pub fn mutate(&mut self, dna: &mut LegacyDna) -> MutationReport {
        let kind = match self.next() % 5 {
            0 => MutationKind::Point,
            1 => MutationKind::Insertion,
            2 => MutationKind::Deletion,
            3 => MutationKind::Duplication,
            _ => MutationKind::Replacement,
        };
        self.mutate_kind(dna, kind)
    }

    pub fn mutate_kind(&mut self, dna: &mut LegacyDna, kind: MutationKind) -> MutationReport {
        if kind == MutationKind::Point {
            let mut point = PointMutator::new(self.random_state);
            let report = point.mutate(dna);
            self.random_state = point.random_state();
            return report;
        }
        let blocks = dna.blocks_mut();
        normalize_master_end(blocks);
        let master_end = blocks.pop().expect("normalized DNA has a master End");
        let changed = match kind {
            MutationKind::Insertion if blocks.len() < MAX_DNA_BLOCKS - 1 => {
                let index = (self.next() as usize) % (blocks.len() + 1);
                let block = self.random_block();
                blocks.insert(index, block);
                true
            }
            MutationKind::Deletion if !blocks.is_empty() => {
                let index = (self.next() as usize) % blocks.len();
                blocks.remove(index);
                true
            }
            MutationKind::Duplication
                if !blocks.is_empty() && blocks.len() < MAX_DNA_BLOCKS - 1 =>
            {
                let start = (self.next() as usize) % blocks.len();
                let available = blocks.len() - start;
                let capacity = MAX_DNA_BLOCKS - 1 - blocks.len();
                let length = (1 + (self.next() as usize) % available.min(4)).min(capacity);
                let copy = blocks[start..start + length].to_vec();
                let insertion = (self.next() as usize) % (blocks.len() + 1);
                blocks.splice(insertion..insertion, copy);
                true
            }
            MutationKind::Replacement if !blocks.is_empty() => {
                let index = (self.next() as usize) % blocks.len();
                let mut replacement = self.random_block();
                if replacement == blocks[index] {
                    replacement = DnaBlock {
                        class: TokenClass::Basic,
                        value: 5,
                    };
                }
                blocks[index] = replacement;
                true
            }
            _ => false,
        };
        blocks.push(master_end);
        MutationReport {
            changes: u32::from(changed),
            kind: changed.then_some(kind),
        }
    }

    fn random_block(&mut self) -> DnaBlock {
        const ADDRESSES: &[i32] = &[1, 2, 3, 4, 7, 8, 300, 301, 310, 330, 830];
        const COMMANDS: &[(TokenClass, i32)] = &[
            (TokenClass::Basic, 1),
            (TokenClass::Basic, 2),
            (TokenClass::Basic, 3),
            (TokenClass::Basic, 4),
            (TokenClass::Basic, 5),
            (TokenClass::Basic, 10),
            (TokenClass::Basic, 11),
            (TokenClass::Logic, 1),
            (TokenClass::Logic, 2),
            (TokenClass::Logic, 4),
        ];
        match self.next() % 7 {
            0 => DnaBlock {
                class: TokenClass::Number,
                value: (self.next() % 64_001) as i32 - 32_000,
            },
            1 => DnaBlock {
                class: TokenClass::Number,
                value: ADDRESSES[(self.next() as usize) % ADDRESSES.len()],
            },
            2 => DnaBlock {
                class: TokenClass::Dereference,
                value: ADDRESSES[(self.next() as usize) % ADDRESSES.len()],
            },
            3 => DnaBlock {
                class: TokenClass::Store,
                value: 1,
            },
            4 => DnaBlock {
                class: TokenClass::Flow,
                value: 2,
            },
            5 => DnaBlock {
                class: TokenClass::Flow,
                value: 4,
            },
            _ => {
                let (class, value) = COMMANDS[(self.next() as usize) % COMMANDS.len()];
                DnaBlock { class, value }
            }
        }
    }

    fn next(&mut self) -> u64 {
        let mut value = self.random_state;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.random_state = value.max(1);
        value
    }
}
