use crate::dna_tokens::{block_token, command_string, token_block};
use crate::{EngineError, sysvar_address};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{BTreeMap, BTreeSet};

pub use crate::dna_tokens::{DnaBlock, MAX_DNA_BLOCKS, TokenClass};

#[path = "dna/native_crossover.rs"]
mod native_crossover;

pub(crate) const LEGACY_OCCURRENCE_CENSUS_SLOTS: usize = 11;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Instruction {
    Number(i32),
    ReadAddress(i32),
    Read(String),
    Address(String),
    ReadResolved(i32),
    AddressResolved(i32),
    Flow(FlowInstruction),
    Store(StoreInstruction),
    Command(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum FlowInstruction {
    Cond,
    Start,
    Else,
    Stop,
    End,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum StoreInstruction {
    Store,
    Inc,
    Dec,
    Add,
    Subtract,
    Multiply,
    Divide,
    Ceil,
    Floor,
    Random,
    Sign,
    Absolute,
    SquareRoot,
    Negate,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DnaMetadata {
    pub generation: u32,
    pub mutations: u64,
    pub tag: String,
    pub legacy_hash: Option<String>,
    pub hash_valid: bool,
}

impl Default for DnaMetadata {
    fn default() -> Self {
        Self {
            generation: 0,
            mutations: 0,
            tag: String::new(),
            legacy_hash: None,
            hash_valid: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegacyDna {
    blocks: Vec<DnaBlock>,
    user_variables: BTreeMap<String, i32>,
    ordered_user_variables: Vec<(String, i32)>,
    new_move: bool,
    compatibility_warnings: Vec<String>,
    metadata: DnaMetadata,
}

#[derive(Serialize)]
struct LegacyDnaSerialize<'a> {
    instructions: Vec<Instruction>,
    user_variables: &'a BTreeMap<String, i32>,
    ordered_user_variables: &'a [(String, i32)],
    new_move: bool,
    compatibility_warnings: &'a [String],
    #[serde(skip_serializing_if = "metadata_is_default")]
    metadata: &'a DnaMetadata,
}

#[derive(Deserialize)]
struct LegacyDnaDeserialize {
    instructions: Vec<Instruction>,
    #[serde(default)]
    user_variables: BTreeMap<String, i32>,
    #[serde(default)]
    ordered_user_variables: Option<Vec<(String, i32)>>,
    #[serde(default)]
    new_move: bool,
    #[serde(default)]
    compatibility_warnings: Vec<String>,
    #[serde(default)]
    metadata: DnaMetadata,
}

impl Serialize for LegacyDna {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        LegacyDnaSerialize {
            instructions: self.instructions(),
            user_variables: &self.user_variables,
            ordered_user_variables: &self.ordered_user_variables,
            new_move: self.new_move,
            compatibility_warnings: &self.compatibility_warnings,
            metadata: &self.metadata,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for LegacyDna {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let decoded = LegacyDnaDeserialize::deserialize(deserializer)?;
        let mut warnings = decoded
            .compatibility_warnings
            .into_iter()
            .collect::<BTreeSet<_>>();
        let mut blocks = decoded
            .instructions
            .into_iter()
            .map(|instruction| {
                instruction_block(instruction, &decoded.user_variables, &mut warnings)
            })
            .collect::<Vec<_>>();
        normalize_master_end(&mut blocks);
        Ok(Self {
            blocks,
            ordered_user_variables: decoded.ordered_user_variables.unwrap_or_else(|| decoded.user_variables.iter().map(|(name, value)| (name.clone(), *value)).collect()),
            user_variables: decoded.user_variables,
            new_move: decoded.new_move,
            compatibility_warnings: warnings.into_iter().collect(),
            metadata: decoded.metadata,
        })
    }
}

impl LegacyDna {
    pub(crate) fn from_legacy_blocks(blocks: Vec<DnaBlock>) -> Self {
        let mut blocks = blocks;
        normalize_master_end(&mut blocks);
        Self {
            blocks,
            user_variables: BTreeMap::new(),
            ordered_user_variables: Vec::new(),
            new_move: false,
            compatibility_warnings: Vec::new(),
            metadata: DnaMetadata::default(),
        }
    }

    pub(crate) fn set_native_context(
        &mut self,
        ordered_user_variables: Vec<(String, i32)>,
        new_move: bool,
        metadata: DnaMetadata,
    ) {
        self.user_variables = ordered_user_variables.iter().filter(|(name, _)| !name.is_empty())
            .map(|(name, value)| (name.to_ascii_lowercase(), *value)).collect();
        self.ordered_user_variables = ordered_user_variables;
        self.new_move = new_move;
        self.metadata = metadata;
    }

    pub(crate) fn from_legacy_wire_parts(
        instructions: Vec<Instruction>,
        user_variables: BTreeMap<String, i32>,
        new_move: bool,
        compatibility_warnings: Vec<String>,
        metadata: DnaMetadata,
    ) -> Self {
        let mut warnings = compatibility_warnings.into_iter().collect::<BTreeSet<_>>();
        let mut blocks = instructions
            .into_iter()
            .map(|instruction| instruction_block(instruction, &user_variables, &mut warnings))
            .collect::<Vec<_>>();
        normalize_master_end(&mut blocks);
        Self {
            blocks,
            ordered_user_variables: user_variables.iter().map(|(name, value)| (name.clone(), *value)).collect(),
            user_variables,
            new_move,
            compatibility_warnings: warnings.into_iter().collect(),
            metadata,
        }
    }

    pub fn parse(source: &str) -> Result<Self, EngineError> {
        let mut user_variables = BTreeMap::new();
        let mut ordered_user_variables = Vec::new();
        let mut metadata = DnaMetadata::default();
        let mut saw_metadata = false;
        let mut hash_validation = None;
        let mut new_move = false;

        let mut hash_prefix = String::new();
        for (line_index, raw_segment) in source.split_inclusive('\n').enumerate() {
            let line_number = line_index + 1;
            let logical_line = raw_segment
                .strip_suffix('\n')
                .unwrap_or(raw_segment)
                .strip_suffix('\r')
                .unwrap_or_else(|| raw_segment.strip_suffix('\n').unwrap_or(raw_segment));
            let raw_line = logical_line.trim_start_matches('\u{feff}');
            let (is_metadata, validation) =
                parse_metadata(raw_line, line_number, &hash_prefix, &mut metadata)?;
            if is_metadata {
                saw_metadata = true;
                if validation.is_some() {
                    hash_validation = validation;
                }
                hash_prefix.push_str(logical_line);
                hash_prefix.push_str("\r\n");
                continue;
            }
            if raw_line.starts_with('/') {
                hash_prefix.push_str(logical_line);
                hash_prefix.push_str("\r\n");
                continue;
            }
            let code = strip_comment(raw_line).trim();
            if code.is_empty() {
                hash_prefix.push_str(logical_line);
                hash_prefix.push_str("\r\n");
                continue;
            }
            if code.eq_ignore_ascii_case("newmove") {
                new_move = true;
                hash_prefix.push_str(logical_line);
                hash_prefix.push_str("\r\n");
                continue;
            }
            let tokens = code.split_whitespace().collect::<Vec<_>>();
            if tokens
                .first()
                .is_some_and(|token| token.eq_ignore_ascii_case("def"))
            {
                parse_definition(&tokens, line_number, &mut user_variables)?;
                ordered_user_variables.push((tokens[1].to_owned(), user_variables[&tokens[1].to_ascii_lowercase()]));
            }
            hash_prefix.push_str(logical_line);
            hash_prefix.push_str("\r\n");
        }

        validate_metadata(&mut metadata, saw_metadata, hash_validation);

        let mut blocks = Vec::new();
        let mut compatibility_warnings = BTreeSet::new();
        'lines: for (line_index, raw_line) in source.lines().enumerate() {
            let line_number = line_index + 1;
            let raw_line = raw_line.trim_start_matches('\u{feff}');
            if raw_line.starts_with('/') || is_metadata_line(raw_line) {
                continue;
            }
            let code = strip_comment(raw_line).trim();
            if code.is_empty() || code.eq_ignore_ascii_case("newmove") {
                continue;
            }
            let tokens = code.split_whitespace().collect::<Vec<_>>();
            if tokens
                .first()
                .is_some_and(|token| token.eq_ignore_ascii_case("def"))
            {
                continue;
            }

            for (token_index, token) in tokens.into_iter().enumerate() {
                let parsed = parse_source_token(
                    token,
                    line_number,
                    token_index + 1,
                    &user_variables,
                    &mut compatibility_warnings,
                )?;
                for block in parsed {
                    if block.class == TokenClass::End {
                        blocks.push(block);
                        break 'lines;
                    }
                    if blocks.len() < MAX_DNA_BLOCKS - 1 {
                        blocks.push(block);
                    }
                }
            }
        }
        normalize_master_end(&mut blocks);

        Ok(Self {
            blocks,
            user_variables,
            ordered_user_variables,
            new_move,
            compatibility_warnings: compatibility_warnings.into_iter().collect(),
            metadata,
        })
    }

    pub fn blocks(&self) -> &[DnaBlock] {
        &self.blocks
    }

    pub fn metadata(&self) -> &DnaMetadata {
        &self.metadata
    }

    pub fn user_variable(&self, name: &str) -> Option<i32> {
        self.user_variables.get(&name.to_ascii_lowercase()).copied()
    }

    pub(crate) fn native_user_variables(&self) -> &[(String, i32)] {
        &self.ordered_user_variables
    }

    pub(crate) fn clear_native_user_variables(&mut self) {
        self.user_variables.clear();
        self.ordered_user_variables.clear();
    }

    pub fn compatibility_warnings(&self) -> &[String] {
        &self.compatibility_warnings
    }

    pub fn uses_new_move(&self) -> bool {
        self.new_move
    }

    pub fn address_reference_count(&self, first: i32, last: i32) -> i32 {
        self.blocks
            .iter()
            .filter(|block| {
                matches!(block.class, TokenClass::Number | TokenClass::Dereference)
                    && (first..=last).contains(&block.value)
            })
            .count()
            .min(i32::MAX as usize) as i32
    }

    pub(crate) fn occurrence_census_248(&self) -> [u16; LEGACY_OCCURRENCE_CENSUS_SLOTS] {
        let coding_end = self
            .blocks
            .iter()
            .take(MAX_DNA_BLOCKS)
            .position(|block| block.class == TokenClass::End && block.value == 1)
            .unwrap_or_else(|| self.blocks.len().min(MAX_DNA_BLOCKS));
        let blocks = &self.blocks[..coding_end];
        let mut occurrences = [0_u16; LEGACY_OCCURRENCE_CENSUS_SLOTS];

        for (index, block) in blocks.iter().enumerate() {
            match block.class {
                TokenClass::Number => {
                    if block.value == 330 {
                        occurrences[8] = occurrences[8].saturating_add(1);
                    }
                    if blocks
                        .get(index + 1)
                        .is_some_and(|next| next.class == TokenClass::Store)
                    {
                        match block.value {
                            1..=7 => {
                                let slot = usize::try_from(block.value - 1).unwrap();
                                occurrences[slot] = occurrences[slot].saturating_add(1);
                            }
                            826 => occurrences[9] = occurrences[9].saturating_add(1),
                            824 => occurrences[10] = occurrences[10].saturating_add(1),
                            _ => {}
                        }
                    }
                }
                TokenClass::Dereference if 500 < block.value && block.value < 510 => {
                    occurrences[7] = occurrences[7].saturating_add(1);
                }
                _ => {}
            }
        }

        occurrences
    }

    pub(crate) fn blocks_mut(&mut self) -> &mut Vec<DnaBlock> {
        &mut self.blocks
    }

    pub(crate) fn instructions(&self) -> Vec<Instruction> {
        self.blocks.iter().copied().map(block_instruction).collect()
    }

    pub(crate) fn gene_count(&self) -> i32 {
        self.gene_ranges().len().min(i32::MAX as usize) as i32
    }

    pub(crate) fn gene(&self, requested: i32) -> Option<Self> {
        let index = usize::try_from(requested.saturating_sub(1)).ok()?;
        let (start, end) = *self.gene_ranges().get(index)?;
        Some(self.with_blocks(self.blocks[start..end].to_vec()))
    }

    pub(crate) fn delete_gene(&self, requested: i32) -> Option<Self> {
        let index = usize::try_from(requested.saturating_sub(1)).ok()?;
        let ranges = self.gene_ranges();
        if ranges.len() <= 1 {
            return None;
        }
        let (start, end) = *ranges.get(index)?;
        let mut blocks = self.blocks[..start].to_vec();
        blocks.extend_from_slice(&self.blocks[end..]);
        Some(self.with_blocks(blocks))
    }

    pub(crate) fn insert_gene(&self, gene: &Self, boundary: usize) -> Self {
        let ranges = self.gene_ranges();
        let insert_at = ranges.get(boundary).map_or_else(
            || {
                self.blocks
                    .iter()
                    .position(|block| block.class == TokenClass::End)
                    .unwrap_or(self.blocks.len())
            },
            |(start, _)| *start,
        );
        let mut blocks = self.blocks[..insert_at].to_vec();
        blocks.extend(
            gene.blocks
                .iter()
                .copied()
                .filter(|block| block.class != TokenClass::End),
        );
        blocks.extend(self.blocks[insert_at..].iter().copied());
        self.with_blocks(blocks)
    }

    /// DB2 `DNAtoInt` and `simplematch`: align insertions before counting unmatched bases.
    pub fn genetic_distance(&self, other: &Self) -> f32 {
        if self.blocks == other.blocks { return 0.0; }
        let (left, right) = native_crossover::match_labels(&self.blocks, &other.blocks);
        let unmatched = left.iter().chain(&right).filter(|label| **label == 0).count();
        unmatched as f32 / (left.len() + right.len()) as f32
    }

    fn gene_ranges(&self) -> Vec<(usize, usize)> {
        let mut ranges = Vec::new();
        let mut start = None;
        let mut in_gene = false;
        for (index, block) in self.blocks.iter().enumerate() {
            if block.class == TokenClass::End {
                break;
            }
            if block.class != TokenClass::Flow {
                continue;
            }
            match block.value {
                1..=3 if !in_gene => {
                    start = Some(index);
                    in_gene = true;
                }
                1..=3 => {}
                4 => {
                    if let Some(start) = start.take() {
                        ranges.push((start, index + 1));
                    }
                    in_gene = false;
                }
                _ => {}
            }
        }
        if let Some(start) = start {
            let end = self
                .blocks
                .iter()
                .position(|block| block.class == TokenClass::End)
                .unwrap_or(self.blocks.len());
            ranges.push((start, end));
        }
        if ranges.is_empty() {
            let end = self
                .blocks
                .iter()
                .position(|block| block.class == TokenClass::End)
                .unwrap_or(self.blocks.len());
            if end > 0 {
                ranges.push((0, end));
            }
        }
        ranges
    }

    fn with_blocks(&self, mut blocks: Vec<DnaBlock>) -> Self {
        normalize_master_end(&mut blocks);
        Self {
            blocks,
            user_variables: self.user_variables.clone(),
            ordered_user_variables: self.ordered_user_variables.clone(),
            new_move: self.new_move,
            compatibility_warnings: self.compatibility_warnings.clone(),
            metadata: self.metadata.clone(),
        }
    }

    pub fn crossover(&self, other: &Self) -> Self {
        let left_end = self.blocks.len().div_ceil(2);
        let right_start = other.blocks.len() / 2;
        let mut blocks = self.blocks[..left_end].to_vec();
        blocks.extend_from_slice(&other.blocks[right_start..]);
        normalize_master_end(&mut blocks);
        let mut user_variables = self.user_variables.clone();
        for (name, address) in &other.user_variables {
            user_variables.entry(name.clone()).or_insert(*address);
        }
        let mut compatibility_warnings = self.compatibility_warnings.clone();
        compatibility_warnings.extend(other.compatibility_warnings.iter().cloned());
        compatibility_warnings.sort();
        compatibility_warnings.dedup();
        Self {
            blocks,
            ordered_user_variables: self.ordered_user_variables.iter()
                .chain(other.ordered_user_variables.iter().filter(|(name, _)| !self.user_variables.contains_key(&name.to_ascii_lowercase())))
                .cloned().collect(),
            user_variables,
            new_move: self.new_move || other.new_move,
            compatibility_warnings,
            metadata: self.metadata.clone(),
        }
    }

    pub fn to_source(&self) -> String {
        let mut lines = vec![
            format!("'#generation: {}", self.metadata.generation),
            format!("'#mutations: {}", self.metadata.mutations),
        ];
        if self.new_move {
            lines.push("NewMove".to_owned());
        }
        lines.extend(self.ordered_user_variables.iter().filter(|(name, _)| !name.is_empty())
            .map(|(name, value)| format!("def {name} {value}")));
        lines.push(
            self.blocks
                .iter()
                .map(|block| detokenize_block(*block))
                .collect::<Vec<_>>()
                .join(" "),
        );
        let prefix = format!("{}\r\n\r\n", lines.join("\r\n"));
        let hash = legacy_hash(&prefix);
        let mut source = format!("{prefix}'#hash: {hash}");
        if !self.metadata.tag.is_empty() {
            source.push_str("\r\n'#tag: ");
            source.push_str(&self.metadata.tag);
        }
        source
    }

    pub fn to_source_with_runtime_metadata(
        &self,
        generation: u32,
        mutations: u64,
        tag: Option<&str>,
    ) -> String {
        let mut dna = self.clone();
        dna.metadata.generation = generation;
        dna.metadata.mutations = mutations;
        if let Some(tag) = tag {
            dna.metadata.tag = tag.to_owned();
        }
        dna.to_source()
    }
}

fn parse_source_token(
    source_token: &str,
    line: usize,
    token_index: usize,
    user_variables: &BTreeMap<String, i32>,
    warnings: &mut BTreeSet<String>,
) -> Result<Vec<DnaBlock>, EngineError> {
    let token = source_token.trim_matches(|character| matches!(character, '(' | ')' | '`'));
    if let Some((name, operation, value)) = split_compact_comparison(token) {
        return Ok(vec![
            DnaBlock {
                class: TokenClass::Dereference,
                value: resolve_address(name, user_variables, warnings),
            },
            DnaBlock {
                class: TokenClass::Number,
                value,
            },
            token_block(operation).expect("comparison table is complete"),
        ]);
    }
    if let Some((number, address)) = split_number_address(token) {
        return Ok(vec![
            DnaBlock {
                class: TokenClass::Number,
                value: number,
            },
            DnaBlock {
                class: TokenClass::Number,
                value: resolve_address(address, user_variables, warnings),
            },
        ]);
    }
    if let Ok(number) = token.parse::<i32>() {
        return Ok(vec![DnaBlock {
            class: TokenClass::Number,
            value: number,
        }]);
    }
    if let Some(block) = token_block(token) {
        return Ok(vec![block]);
    }
    if let Some(address) = token
        .strip_prefix('*')
        .and_then(|value| value.parse::<i32>().ok())
    {
        return Ok(vec![DnaBlock {
            class: TokenClass::Dereference,
            value: address,
        }]);
    }
    if let Some(name) = token.strip_prefix("*.").or_else(|| token.strip_prefix('*')) {
        validate_name(name, line)?;
        return Ok(vec![DnaBlock {
            class: TokenClass::Dereference,
            value: resolve_address(name, user_variables, warnings),
        }]);
    }
    if let Some(name) = token.strip_prefix('.') {
        validate_name(name, line)?;
        return Ok(vec![DnaBlock {
            class: TokenClass::Number,
            value: resolve_address(name, user_variables, warnings),
        }]);
    }

    warnings.insert(format!(
        "line {line}, token {token_index}: unknown token {source_token} compiles to 0"
    ));
    Ok(vec![DnaBlock {
        class: TokenClass::Number,
        value: 0,
    }])
}

fn resolve_address(
    name: &str,
    user_variables: &BTreeMap<String, i32>,
    warnings: &mut BTreeSet<String>,
) -> i32 {
    if let Some(address) = user_variables.get(&name.to_ascii_lowercase()).copied() {
        return address;
    }
    match sysvar_address(name) {
        Some(address) => address,
        None => {
            warnings.insert(format!(
                "unknown sysvar .{name} resolves to memory address 0"
            ));
            0
        }
    }
}

fn block_instruction(block: DnaBlock) -> Instruction {
    match block.class {
        TokenClass::Number => Instruction::Number(block.value),
        TokenClass::Dereference => Instruction::ReadResolved(block.value),
        TokenClass::Store => store_instruction(block.value)
            .map(Instruction::Store)
            .unwrap_or(Instruction::Number(0)),
        TokenClass::Flow => flow_instruction(block.value)
            .map(Instruction::Flow)
            .unwrap_or(Instruction::Number(0)),
        TokenClass::End if block.value == 1 => Instruction::Flow(FlowInstruction::End),
        TokenClass::Basic
        | TokenClass::Advanced
        | TokenClass::Bitwise
        | TokenClass::Condition
        | TokenClass::Logic => command_string(block)
            .cloned()
            .map(Instruction::Command)
            .unwrap_or(Instruction::Number(0)),
        TokenClass::Reserved | TokenClass::End => Instruction::Number(0),
    }
}

fn instruction_block(
    instruction: Instruction,
    user_variables: &BTreeMap<String, i32>,
    warnings: &mut BTreeSet<String>,
) -> DnaBlock {
    match instruction {
        Instruction::Number(value) | Instruction::AddressResolved(value) => DnaBlock {
            class: TokenClass::Number,
            value,
        },
        Instruction::ReadAddress(value) | Instruction::ReadResolved(value) => DnaBlock {
            class: TokenClass::Dereference,
            value,
        },
        Instruction::Read(name) => DnaBlock {
            class: TokenClass::Dereference,
            value: resolve_address(&name, user_variables, warnings),
        },
        Instruction::Address(name) => DnaBlock {
            class: TokenClass::Number,
            value: resolve_address(&name, user_variables, warnings),
        },
        Instruction::Flow(FlowInstruction::End) => DnaBlock {
            class: TokenClass::End,
            value: 1,
        },
        Instruction::Flow(command) => DnaBlock {
            class: TokenClass::Flow,
            value: match command {
                FlowInstruction::Cond => 1,
                FlowInstruction::Start => 2,
                FlowInstruction::Else => 3,
                FlowInstruction::Stop => 4,
                FlowInstruction::End => unreachable!(),
            },
        },
        Instruction::Store(command) => DnaBlock {
            class: TokenClass::Store,
            value: match command {
                StoreInstruction::Store => 1,
                StoreInstruction::Inc => 2,
                StoreInstruction::Dec => 3,
                StoreInstruction::Add => 4,
                StoreInstruction::Subtract => 5,
                StoreInstruction::Multiply => 6,
                StoreInstruction::Divide => 7,
                StoreInstruction::Ceil => 8,
                StoreInstruction::Floor => 9,
                StoreInstruction::Random => 10,
                StoreInstruction::Sign => 11,
                StoreInstruction::Absolute => 12,
                StoreInstruction::SquareRoot => 13,
                StoreInstruction::Negate => 14,
            },
        },
        Instruction::Command(command) => token_block(&command).unwrap_or(DnaBlock {
            class: TokenClass::Number,
            value: 0,
        }),
    }
}

fn flow_instruction(value: i32) -> Option<FlowInstruction> {
    match value {
        1 => Some(FlowInstruction::Cond),
        2 => Some(FlowInstruction::Start),
        3 => Some(FlowInstruction::Else),
        4 => Some(FlowInstruction::Stop),
        _ => None,
    }
}

fn store_instruction(value: i32) -> Option<StoreInstruction> {
    match value {
        1 => Some(StoreInstruction::Store),
        2 => Some(StoreInstruction::Inc),
        3 => Some(StoreInstruction::Dec),
        4 => Some(StoreInstruction::Add),
        5 => Some(StoreInstruction::Subtract),
        6 => Some(StoreInstruction::Multiply),
        7 => Some(StoreInstruction::Divide),
        8 => Some(StoreInstruction::Ceil),
        9 => Some(StoreInstruction::Floor),
        10 => Some(StoreInstruction::Random),
        11 => Some(StoreInstruction::Sign),
        12 => Some(StoreInstruction::Absolute),
        13 => Some(StoreInstruction::SquareRoot),
        14 => Some(StoreInstruction::Negate),
        _ => None,
    }
}

fn detokenize_block(block: DnaBlock) -> String {
    match block.class {
        TokenClass::Number => crate::sysvars::sysvar_name(block.value)
            .map_or_else(|| block.value.to_string(), |name| format!(".{name}")),
        TokenClass::Dereference => format!("*{}", block.value),
        _ => block_token(block).unwrap_or("0").to_owned(),
    }
}

fn parse_definition(
    tokens: &[&str],
    line: usize,
    user_variables: &mut BTreeMap<String, i32>,
) -> Result<(), EngineError> {
    if tokens.len() != 3 {
        return Err(parse_error(line, "expected: def <name> <value>"));
    }
    validate_name(tokens[1], line)?;
    let value = tokens[2]
        .parse::<i32>()
        .map_err(|_| parse_error(line, format!("invalid variable value {}", tokens[2])))?;
    user_variables.insert(tokens[1].to_ascii_lowercase(), value);
    Ok(())
}

fn parse_metadata(
    raw_line: &str,
    line: usize,
    prefix: &str,
    metadata: &mut DnaMetadata,
) -> Result<(bool, Option<bool>), EngineError> {
    let Some(header) = raw_line
        .strip_prefix("'#")
        .or_else(|| raw_line.strip_prefix("/#"))
    else {
        return Ok((false, None));
    };
    let Some((field, value)) = header.split_once(':') else {
        return Ok((true, None));
    };
    let value = value.trim();
    let validation = match field.trim().to_ascii_lowercase().as_str() {
        "generation" => {
            metadata.generation = value
                .parse::<u32>()
                .map_err(|_| parse_error(line, "invalid generation metadata"))?;
            None
        }
        "mutations" => {
            metadata.mutations = value
                .parse::<u64>()
                .map_err(|_| parse_error(line, "invalid mutations metadata"))?;
            None
        }
        "tag" => {
            metadata.tag = value.to_owned();
            None
        }
        "hash" => {
            metadata.legacy_hash = Some(value.to_owned());
            Some(value == legacy_hash(prefix))
        }
        _ => None,
    };
    Ok((true, validation))
}

fn validate_metadata(
    metadata: &mut DnaMetadata,
    saw_metadata: bool,
    hash_validation: Option<bool>,
) {
    if !saw_metadata {
        *metadata = DnaMetadata::default();
        return;
    }
    metadata.hash_valid = hash_validation.unwrap_or(false);
    if hash_validation == Some(false) {
        metadata.generation = 0;
        metadata.mutations = 0;
    }
}

fn legacy_hash(source_prefix: &str) -> String {
    let mut trimmed = source_prefix.trim_matches(' ');
    while let Some(without_crlf) = trimmed.strip_suffix("\r\n") {
        trimmed = without_crlf;
    }

    let mut buffer = [0u16; 21];
    for (index, character) in trimmed.chars().enumerate() {
        let position = index + 1;
        let slot = position % 20;
        let previous = (position - 1) % 20;
        buffer[slot] = (buffer[slot] + ansi_code(character) + buffer[previous]) % 100;
    }
    (0..20)
        .map(|index| char::from_u32(u32::from(buffer[index] % 93 + 33)).unwrap())
        .collect()
}

fn ansi_code(character: char) -> u16 {
    let value = character as u32;
    if value <= 0xff {
        value as u16
    } else {
        match character {
            '€' => 128,
            '‚' => 130,
            'ƒ' => 131,
            '„' => 132,
            '…' => 133,
            '†' => 134,
            '‡' => 135,
            'ˆ' => 136,
            '‰' => 137,
            'Š' => 138,
            '‹' => 139,
            'Œ' => 140,
            'Ž' => 142,
            '‘' => 145,
            '’' => 146,
            '“' => 147,
            '”' => 148,
            '•' => 149,
            '–' => 150,
            '—' => 151,
            '˜' => 152,
            '™' => 153,
            'š' => 154,
            '›' => 155,
            'œ' => 156,
            'ž' => 158,
            'Ÿ' => 159,
            _ => 63,
        }
    }
}

fn metadata_is_default(metadata: &DnaMetadata) -> bool {
    metadata.generation == 0
        && metadata.mutations == 0
        && metadata.tag.is_empty()
        && metadata.legacy_hash.is_none()
        && metadata.hash_valid
}

fn is_metadata_line(raw_line: &str) -> bool {
    raw_line.starts_with("'#") || raw_line.starts_with("/#")
}

fn strip_comment(line: &str) -> &str {
    line.split_once('\'').map_or(line, |(code, _)| code)
}

pub(crate) fn normalize_master_end(blocks: &mut Vec<DnaBlock>) {
    let coding_end = blocks
        .iter()
        .position(|block| block.class == TokenClass::End)
        .unwrap_or(blocks.len())
        .min(MAX_DNA_BLOCKS - 1);
    blocks.truncate(coding_end);
    blocks.push(DnaBlock {
        class: TokenClass::End,
        value: 1,
    });
}

fn split_compact_comparison(token: &str) -> Option<(&str, &str, i32)> {
    let expression = token.strip_prefix("*.")?;
    for operation in [">=", "<=", "!=", "=", ">", "<"] {
        let Some((name, value)) = expression.split_once(operation) else {
            continue;
        };
        if name.is_empty()
            || !name
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '_')
        {
            return None;
        }
        return Some((name, operation, value.parse().ok()?));
    }
    None
}

fn split_number_address(token: &str) -> Option<(i32, &str)> {
    let dot = token.find('.')?;
    if dot == 0 {
        return None;
    }
    let number = token[..dot].parse::<i32>().ok()?;
    let address = &token[dot + 1..];
    if address.is_empty()
        || !address
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        return None;
    }
    Some((number, address))
}

fn validate_name(name: &str, line: usize) -> Result<&str, EngineError> {
    if !name.is_empty()
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        Ok(name)
    } else {
        Err(parse_error(line, format!("invalid system variable {name}")))
    }
}

fn legacy_nucleotide(block: DnaBlock) -> i32 {
    let mut value = block.value.clamp(-32000, 32000);
    if matches!(block.class, TokenClass::Number | TokenClass::Dereference) {
        if value.abs() > 999 {
            value = (512.0 * f64::from(value.signum()) + f64::from(value) / 2.05).round_ties_even()
                as i32;
        }
        return -16646
            + value
            + if block.class == TokenClass::Dereference {
                32729
            } else {
                0
            };
    }
    // calc_dnamatrix enumerates valid commands by type, then by value.
    let offset = match block.class {
        TokenClass::Basic => 0,
        TokenClass::Advanced => 14,
        TokenClass::Bitwise => 28,
        TokenClass::Condition => 37,
        TokenClass::Logic => 47,
        TokenClass::Store => 58,
        TokenClass::Flow => 72,
        TokenClass::End => 76,
        _ => 0,
    };
    32691
        + if block_token(block).is_some() {
            offset + value - 1
        } else {
            0
        }
}

fn parse_error(line: usize, message: impl Into<String>) -> EngineError {
    EngineError::DnaParse {
        line,
        message: message.into(),
    }
}
