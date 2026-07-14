use crate::{sysvar_address, EngineError};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Instruction {
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
pub enum FlowInstruction { Cond, Start, Else, Stop, End }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StoreInstruction {
    Store, Inc, Dec, Add, Subtract, Multiply, Divide, Ceil, Floor, Random,
    Sign, Absolute, SquareRoot, Negate,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacyDna {
    instructions: Vec<Instruction>,
    user_variables: BTreeMap<String, i32>,
    #[serde(default)]
    new_move: bool,
    #[serde(default)]
    compatibility_warnings: Vec<String>,
}

impl LegacyDna {
    pub fn parse(source: &str) -> Result<Self, EngineError> {
        let mut instructions = Vec::new();
        let mut user_variables = BTreeMap::new();
        let mut compatibility_warnings = BTreeSet::new();
        let mut new_move = false;

        for (line_index, raw_line) in source.lines().enumerate() {
            let line_number = line_index + 1;
            let code = raw_line.split_once('\'').map_or(raw_line, |(code, _)| code)
                .trim_start_matches('\u{feff}')
                .trim();
            if code.is_empty() {
                continue;
            }
            if code.eq_ignore_ascii_case("newmove") {
                new_move = true;
                continue;
            }

            let tokens: Vec<_> = code.split_whitespace().collect();
            if tokens.len() > 1 && tokens.iter().all(|token| token.chars().all(|character| character.is_ascii_alphabetic())) {
                continue;
            }
            if tokens.first().is_some_and(|token| token.eq_ignore_ascii_case("def")) {
                if tokens.len() != 3 {
                    return Err(parse_error(line_number, "expected: def <name> <value>"));
                }
                let value = tokens[2].parse::<i32>().map_err(|_| {
                    parse_error(line_number, format!("invalid variable value {}", tokens[2]))
                })?;
                user_variables.insert(tokens[1].to_ascii_lowercase(), value);
                continue;
            }

            for token in tokens {
                if let Some((name, operation, value)) = split_compact_comparison(token) {
                    instructions.push(Instruction::Read(name.to_owned()));
                    instructions.push(Instruction::Number(value));
                    instructions.push(Instruction::Command(operation.to_owned()));
                } else if let Some((number, address)) = split_number_address(token) {
                    instructions.push(Instruction::Number(number));
                    instructions.push(Instruction::Address(address.to_owned()));
                } else {
                    instructions.push(parse_token(token, line_number)?);
                }
            }
        }

        for instruction in &mut instructions {
            match instruction {
                Instruction::Read(name) => {
                    let address = resolve_address(name, &user_variables, &mut compatibility_warnings);
                    *instruction = Instruction::ReadResolved(address);
                }
                Instruction::Address(name) => {
                    let address = resolve_address(name, &user_variables, &mut compatibility_warnings);
                    *instruction = Instruction::AddressResolved(address);
                }
                _ => {}
            }
        }

        Ok(Self {
            instructions,
            user_variables,
            new_move,
            compatibility_warnings: compatibility_warnings.into_iter().collect(),
        })
    }

    pub fn instructions(&self) -> &[Instruction] {
        &self.instructions
    }

    pub fn user_variable(&self, name: &str) -> Option<i32> {
        self.user_variables.get(&name.to_ascii_lowercase()).copied()
    }

    pub fn compatibility_warnings(&self) -> &[String] {
        &self.compatibility_warnings
    }

    pub fn uses_new_move(&self) -> bool {
        self.new_move
    }

    pub fn address_reference_count(&self, first: i32, last: i32) -> i32 {
        self.instructions.iter().filter(|instruction| {
            let address = match instruction {
                Instruction::ReadAddress(address)
                | Instruction::ReadResolved(address)
                | Instruction::AddressResolved(address) => *address,
                _ => return false,
            };
            (first..=last).contains(&address)
        }).count().min(i32::MAX as usize) as i32
    }

    pub(crate) fn instructions_mut(&mut self) -> &mut Vec<Instruction> {
        &mut self.instructions
    }

    pub fn crossover(&self, other: &Self) -> Self {
        let left_end = self.instructions.len().div_ceil(2);
        let right_start = other.instructions.len() / 2;
        let mut instructions = self.instructions[..left_end].to_vec();
        instructions.extend_from_slice(&other.instructions[right_start..]);
        let mut user_variables = self.user_variables.clone();
        for (name, address) in &other.user_variables {
            user_variables.entry(name.clone()).or_insert(*address);
        }
        let mut compatibility_warnings = self.compatibility_warnings.clone();
        compatibility_warnings.extend(other.compatibility_warnings.iter().cloned());
        compatibility_warnings.sort();
        compatibility_warnings.dedup();
        Self {
            instructions,
            user_variables,
            new_move: self.new_move || other.new_move,
            compatibility_warnings,
        }
    }

    pub fn to_source(&self) -> String {
        let instructions = self.instructions.iter().map(instruction_token).collect::<Vec<_>>().join(" ");
        if self.new_move {
            format!("NewMove\n{instructions}")
        } else {
            instructions
        }
    }
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
        Some(address) => {
            if !crate::sysvars::sysvar_is_active(address) {
                warnings.insert(format!("legacy sysvar .{name} is accepted but currently inert"));
            }
            address
        }
        None => {
            warnings.insert(format!("unknown sysvar .{name} resolves to memory address 0"));
            0
        }
    }
}

fn instruction_token(instruction: &Instruction) -> String {
    match instruction {
        Instruction::Number(value) => value.to_string(),
        Instruction::ReadAddress(address) => format!("*{address}"),
        Instruction::Read(name) => format!("*.{name}"),
        Instruction::Address(name) => format!(".{name}"),
        Instruction::ReadResolved(address) => crate::sysvars::sysvar_name(*address)
            .map_or_else(|| format!("*{address}"), |name| format!("*.{name}")),
        Instruction::AddressResolved(address) => crate::sysvars::sysvar_name(*address)
            .map_or_else(|| address.to_string(), |name| format!(".{name}")),
        Instruction::Flow(value) => match value {
            FlowInstruction::Cond => "cond",
            FlowInstruction::Start => "start",
            FlowInstruction::Else => "else",
            FlowInstruction::Stop => "stop",
            FlowInstruction::End => "end",
        }.to_owned(),
        Instruction::Store(value) => match value {
            StoreInstruction::Store => "store",
            StoreInstruction::Inc => "inc",
            StoreInstruction::Dec => "dec",
            StoreInstruction::Add => "+=",
            StoreInstruction::Subtract => "-=",
            StoreInstruction::Multiply => "*=",
            StoreInstruction::Divide => "/=",
            StoreInstruction::Ceil => "ceilstore",
            StoreInstruction::Floor => "floorstore",
            StoreInstruction::Random => "rndstore",
            StoreInstruction::Sign => "sgnstore",
            StoreInstruction::Absolute => "absstore",
            StoreInstruction::SquareRoot => "sqrstore",
            StoreInstruction::Negate => "negstore",
        }.to_owned(),
        Instruction::Command(command) => command.clone(),
    }
}


fn split_compact_comparison(token: &str) -> Option<(&str, &str, i32)> {
    let expression = token.strip_prefix("*.")?;
    for operation in [">=", "<=", "!=", "=", ">", "<"] {
        let Some((name, value)) = expression.split_once(operation) else { continue };
        if name.is_empty() || !name.chars().all(|character| character.is_ascii_alphanumeric() || character == '_') {
            return None;
        }
        return Some((name, operation, value.parse().ok()?));
    }
    None
}

fn parse_token(token: &str, line: usize) -> Result<Instruction, EngineError> {
    let token = token.trim_matches(|character| matches!(character, '(' | ')' | '`'));
    if let Ok(number) = token.parse::<i32>() {
        return Ok(Instruction::Number(number));
    }
    if let Some(address) = token.strip_prefix('*').and_then(|value| value.parse::<i32>().ok()) {
        return Ok(Instruction::ReadAddress(address));
    }
    if let Some(name) = token.strip_prefix("*.") {
        return validate_name(name, line).map(|name| Instruction::Read(name.to_owned()));
    }
    if let Some(name) = token.strip_prefix('*') {
        if !name.is_empty() && name.chars().all(|character| character.is_ascii_alphanumeric() || character == '_') {
            return Ok(Instruction::Read(name.to_owned()));
        }
    }
    if let Some(name) = token.strip_prefix('.') {
        return validate_name(name, line).map(|name| Instruction::Address(name.to_owned()));
    }

    let command = token.to_ascii_lowercase();
    let compiled = match command.as_str() {
        "cond" => Some(Instruction::Flow(FlowInstruction::Cond)),
        "start" => Some(Instruction::Flow(FlowInstruction::Start)),
        "else" => Some(Instruction::Flow(FlowInstruction::Else)),
        "stop" | "stopd" => Some(Instruction::Flow(FlowInstruction::Stop)),
        "end" => Some(Instruction::Flow(FlowInstruction::End)),
        "store" => Some(Instruction::Store(StoreInstruction::Store)),
        "inc" => Some(Instruction::Store(StoreInstruction::Inc)),
        "dec" => Some(Instruction::Store(StoreInstruction::Dec)),
        "+=" => Some(Instruction::Store(StoreInstruction::Add)),
        "-=" => Some(Instruction::Store(StoreInstruction::Subtract)),
        "*=" => Some(Instruction::Store(StoreInstruction::Multiply)),
        "/=" => Some(Instruction::Store(StoreInstruction::Divide)),
        "ceilstore" => Some(Instruction::Store(StoreInstruction::Ceil)),
        "floorstore" => Some(Instruction::Store(StoreInstruction::Floor)),
        "rndstore" => Some(Instruction::Store(StoreInstruction::Random)),
        "sgnstore" => Some(Instruction::Store(StoreInstruction::Sign)),
        "absstore" => Some(Instruction::Store(StoreInstruction::Absolute)),
        "sqrstore" => Some(Instruction::Store(StoreInstruction::SquareRoot)),
        "negstore" => Some(Instruction::Store(StoreInstruction::Negate)),
        _ => None,
    };
    if let Some(compiled) = compiled { return Ok(compiled); }
    const COMMANDS: &[&str] = &[
        "add", "sub", "mult", "div", "mod", "rnd", "sqrt", "pow", "pyth", "angle",
        "dist", "ceil", "floor", "abs", "sgn", "min", "max", "log", "store", "inc",
        "dec", "and", "or", "xor", "not", "=", "!=", ">", "<", ">=", "<=", "%=",
        "!%=", "cond", "start", "else", "stop", "end", "clear", "drop", "dup", "swap",
        "over", "rot", "true", "false", "bitand", "bitor", "bitxor", "bitnot", "bitinc",
        "bitdec", "<<", ">>", "~", "neg", "--", "++", "dupbool", "clearbool", "sqr",
        "=>", "=<", "=!", "+", "-", "*", "/", "dropbool", "|", "&", "stopd", "increase",
        "dropint", "clearint", "swapint", "overint", "swapbool", "overbool", "debugint",
        "debugbool", "root", "logx", "sin", "cos", "~=", "!~=", "+=", "-=", "*=", "/=",
        "ceilstore", "floorstore", "rndstore", "sgnstore", "absstore", "sqrstore", "negstore",
    ];
    if COMMANDS.contains(&command.as_str()) {
        Ok(Instruction::Command(command))
    } else {
        Err(parse_error(line, format!("unknown instruction {token}")))
    }
}

fn split_number_address(token: &str) -> Option<(i32, &str)> {
    let dot = token.find('.')?;
    if dot == 0 {
        return None;
    }
    let number = token[..dot].parse::<i32>().ok()?;
    let address = &token[dot + 1..];
    if address.is_empty() || !address.chars().all(|character| character.is_ascii_alphanumeric() || character == '_') {
        return None;
    }
    Some((number, address))
}

fn validate_name(name: &str, line: usize) -> Result<&str, EngineError> {
    if !name.is_empty() && name.chars().all(|character| character.is_ascii_alphanumeric() || character == '_') {
        Ok(name)
    } else {
        Err(parse_error(line, format!("invalid system variable {name}")))
    }
}

fn parse_error(line: usize, message: impl Into<String>) -> EngineError {
    EngineError::DnaParse { line, message: message.into() }
}
