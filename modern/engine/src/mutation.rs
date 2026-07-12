use crate::{Instruction, LegacyDna};
use crate::dna::{FlowInstruction, StoreInstruction};

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
        Self { random_state: seed.max(1) }
    }

    pub fn random_state(&self) -> u64 {
        self.random_state
    }

    pub fn mutate(&mut self, dna: &mut LegacyDna) -> MutationReport {
        let numeric_positions: Vec<_> = dna.instructions().iter().enumerate()
            .filter_map(|(index, instruction)| matches!(instruction, Instruction::Number(_)).then_some(index))
            .collect();
        if numeric_positions.is_empty() {
            return MutationReport::default();
        }
        let position = numeric_positions[(self.next() as usize) % numeric_positions.len()];
        let mut delta = (self.next() % 21) as i32 - 10;
        if delta == 0 { delta = 1; }
        if let Instruction::Number(value) = &mut dna.instructions_mut()[position] {
            *value = value.saturating_add(delta);
        }
        MutationReport { changes: 1, kind: Some(MutationKind::Point) }
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
        Self { random_state: seed.max(1) }
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
        let instructions = dna.instructions_mut();
        let changed = match kind {
            MutationKind::Insertion => {
                let index = (self.next() as usize) % (instructions.len() + 1);
                let instruction = self.random_instruction();
                instructions.insert(index, instruction);
                true
            }
            MutationKind::Deletion if !instructions.is_empty() => {
                let index = (self.next() as usize) % instructions.len();
                instructions.remove(index);
                true
            }
            MutationKind::Duplication if !instructions.is_empty() => {
                let start = (self.next() as usize) % instructions.len();
                let available = instructions.len() - start;
                let length = 1 + (self.next() as usize) % available.min(4);
                let copy = instructions[start..start + length].to_vec();
                let insertion = (self.next() as usize) % (instructions.len() + 1);
                instructions.splice(insertion..insertion, copy);
                true
            }
            MutationKind::Replacement if !instructions.is_empty() => {
                let index = (self.next() as usize) % instructions.len();
                let mut replacement = self.random_instruction();
                if replacement == instructions[index] {
                    replacement = Instruction::Command("rnd".to_owned());
                }
                instructions[index] = replacement;
                true
            }
            _ => false,
        };
        MutationReport { changes: u32::from(changed), kind: changed.then_some(kind) }
    }

    fn random_instruction(&mut self) -> Instruction {
        const ADDRESSES: &[i32] = &[1, 2, 3, 4, 7, 8, 300, 301, 310, 330, 830];
        const COMMANDS: &[&str] = &["add", "sub", "mult", "div", "rnd", "dup", "drop", "and", "or", "not"];
        match self.next() % 7 {
            0 => Instruction::Number((self.next() % 64_001) as i32 - 32_000),
            1 => Instruction::AddressResolved(ADDRESSES[(self.next() as usize) % ADDRESSES.len()]),
            2 => Instruction::ReadResolved(ADDRESSES[(self.next() as usize) % ADDRESSES.len()]),
            3 => Instruction::Store(StoreInstruction::Store),
            4 => Instruction::Flow(FlowInstruction::Start),
            5 => Instruction::Flow(FlowInstruction::Stop),
            _ => Instruction::Command(COMMANDS[(self.next() as usize) % COMMANDS.len()].to_owned()),
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
