use crate::{sysvar_address, EngineError, Instruction, LegacyDna};
use crate::dna::{FlowInstruction, StoreInstruction};
use smallvec::SmallVec;

type IntegerStack = SmallVec<[i32; 100]>;
type BooleanStack = SmallVec<[bool; 100]>;
use serde::{Deserialize, Serialize};

const MAX_MEMORY: i32 = 1_000;
const MAX_INTEGER: i64 = 2_000_000_000;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VmMemory {
    cells: Vec<(u16, i32)>,
}

impl Default for VmMemory {
    fn default() -> Self {
        Self { cells: Vec::new() }
    }
}

impl VmMemory {
    pub fn read(&self, address: i32) -> i32 {
        let address = normalize_address(address) as u16;
        self.cells.binary_search_by_key(&address, |cell| cell.0)
            .map_or(0, |index| self.cells[index].1)
    }

    pub fn write(&mut self, address: i32, value: i32) {
        if address == 0 {
            return;
        }
        let address = normalize_address(address) as u16;
        let value = normalize_store(value);
        match self.cells.binary_search_by_key(&address, |cell| cell.0) {
            Ok(index) if value == 0 => { self.cells.remove(index); }
            Ok(index) => self.cells[index].1 = value,
            Err(_) if value == 0 => {}
            Err(index) => self.cells.insert(index, (address, value)),
        }
    }

    pub fn read_sysvar(&self, name: &str) -> i32 {
        sysvar_address(name).map_or(0, |address| self.read(address))
    }

    pub fn write_sysvar(&mut self, name: &str, value: i32) {
        if let Some(address) = sysvar_address(name) {
            self.write(address, value);
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VmReport {
    pub instructions_executed: u32,
    pub stores_applied: u32,
}

pub struct DnaVm {
    random_state: u64,
}

impl DnaVm {
    pub fn new(seed: u64) -> Self {
        Self { random_state: seed.max(1) }
    }

    pub fn random_state(&self) -> u64 {
        self.random_state
    }

    pub fn execute(&mut self, program: &LegacyDna, memory: &mut VmMemory) -> Result<VmReport, EngineError> {
        let mut integers = IntegerStack::new();
        let mut booleans = BooleanStack::new();
        let mut flow = Flow::Clear;
        let mut branch_condition = true;
        let mut report = VmReport::default();

        for instruction in program.instructions() {
            report.instructions_executed = report.instructions_executed.saturating_add(1);
            let flow_instruction = match instruction {
                Instruction::Flow(command) => Some(*command),
                Instruction::Command(command) => match command.as_str() {
                    "cond" => Some(FlowInstruction::Cond),
                    "start" => Some(FlowInstruction::Start),
                    "else" => Some(FlowInstruction::Else),
                    "stop" | "stopd" => Some(FlowInstruction::Stop),
                    "end" => Some(FlowInstruction::End),
                    _ => None,
                },
                _ => None,
            };
            if let Some(command) = flow_instruction {
                match command {
                    FlowInstruction::Cond => {
                        flow = Flow::Condition;
                        booleans.clear();
                        continue;
                    }
                    FlowInstruction::Start => {
                        branch_condition = booleans.last().copied().unwrap_or(true);
                        flow = if branch_condition { Flow::Body } else { Flow::Clear };
                        booleans.clear();
                        continue;
                    }
                    FlowInstruction::Else => {
                        flow = if branch_condition { Flow::Clear } else { Flow::ElseBody };
                        booleans.clear();
                        continue;
                    }
                    FlowInstruction::Stop => {
                        flow = Flow::Clear;
                        integers.clear();
                        booleans.clear();
                        branch_condition = true;
                        continue;
                    }
                    FlowInstruction::End => break,
                }
            }

            if flow == Flow::Clear {
                continue;
            }

            match instruction {
                Instruction::Number(value) => push(&mut integers, *value),
                Instruction::ReadAddress(address) => {
                    let value = memory.read(*address);
                    push(&mut integers, value);
                }
                Instruction::Read(name) => {
                    let address = resolve_address(program, name);
                    push(&mut integers, memory.read(address));
                }
                Instruction::Address(name) => push(&mut integers, resolve_address(program, name)),
                Instruction::ReadResolved(address) => push(&mut integers, memory.read(*address)),
                Instruction::AddressResolved(address) => push(&mut integers, *address),
                Instruction::Flow(_) => {}
                Instruction::Store(command) => {
                    if matches!(flow, Flow::Body | Flow::ElseBody) && booleans.last().copied().unwrap_or(true) {
                        if execute_compiled_store(*command, &mut integers, memory, &mut self.random_state) {
                            report.stores_applied = report.stores_applied.saturating_add(1);
                        }
                    }
                }
                Instruction::Command(command) => {
                    if is_store(command) {
                        if matches!(flow, Flow::Body | Flow::ElseBody) && booleans.last().copied().unwrap_or(true) {
                            if execute_store(command, &mut integers, memory, &mut self.random_state) {
                                report.stores_applied = report.stores_applied.saturating_add(1);
                            }
                        }
                    } else if is_condition(command) {
                        execute_condition(command, &mut integers, &mut booleans);
                    } else if is_logic(command) {
                        execute_logic(command, &mut booleans);
                    } else {
                        execute_integer(command, &mut integers, memory, &mut self.random_state);
                    }
                }
            }
        }
        Ok(report)
    }
}

fn execute_compiled_store(
    command: StoreInstruction,
    stack: &mut IntegerStack,
    memory: &mut VmMemory,
    random_state: &mut u64,
) -> bool {
    let address = pop(stack);
    if address == 0 { return false; }
    match command {
        StoreInstruction::Store => memory.write(address, pop(stack)),
        StoreInstruction::Inc => memory.write(address, memory.read(address).wrapping_add(1)),
        StoreInstruction::Dec => memory.write(address, memory.read(address).wrapping_sub(1)),
        StoreInstruction::Add => memory.write(address, memory.read(address).saturating_add(pop(stack))),
        StoreInstruction::Subtract => memory.write(address, memory.read(address).saturating_sub(pop(stack))),
        StoreInstruction::Multiply => memory.write(address, bounded(memory.read(address) as i64 * pop(stack) as i64)),
        StoreInstruction::Divide => {
            let value = pop(stack);
            memory.write(address, if value == 0 { 0 } else { memory.read(address) / value });
        }
        StoreInstruction::Ceil => { let value = pop(stack); memory.write(address, memory.read(address).min(value)); }
        StoreInstruction::Floor => { let value = pop(stack); memory.write(address, memory.read(address).max(value)); }
        StoreInstruction::Random => {
            let value = pop(stack).unsigned_abs() as u64;
            memory.write(address, if value == 0 { 0 } else { (next_random(random_state) % (value + 1)) as i32 });
        }
        StoreInstruction::Sign => memory.write(address, memory.read(address).signum()),
        StoreInstruction::Absolute => memory.write(address, memory.read(address).saturating_abs()),
        StoreInstruction::SquareRoot => memory.write(address, (memory.read(address).max(0) as f64).sqrt().round() as i32),
        StoreInstruction::Negate => memory.write(address, memory.read(address).wrapping_neg()),
    }
    true
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Flow {
    Clear,
    Condition,
    Body,
    ElseBody,
}

fn execute_integer(command: &str, stack: &mut IntegerStack, memory: &VmMemory, random_state: &mut u64) {
    match command {
        "add" | "+" => binary(stack, |a, b| bounded(a as i64 + b as i64)),
        "sub" | "-" => binary(stack, |a, b| bounded(a as i64 - b as i64)),
        "mult" | "*" => binary(stack, |a, b| bounded(a as i64 * b as i64)),
        "div" | "/" => binary(stack, |a, b| if b == 0 { 0 } else { a / b }),
        "mod" => binary(stack, |a, b| if b == 0 { 0 } else { a % b }),
        "rnd" => {
            let upper = pop(stack).abs() as u64;
            push(stack, if upper == 0 { 0 } else { (next_random(random_state) % (upper + 1)) as i32 });
        }
        "sgn" => unary(stack, i32::signum),
        "abs" => unary(stack, |value| value.saturating_abs()),
        "dup" | "dupint" => { let value = pop(stack); push(stack, value); push(stack, value); }
        "drop" | "dropint" => { pop(stack); }
        "clear" | "clearint" => stack.clear(),
        "swap" | "swapint" => { let b = pop(stack); let a = pop(stack); push(stack, b); push(stack, a); }
        "over" | "overint" => {
            let b = pop(stack); let a = pop(stack); push(stack, a); push(stack, b); push(stack, a);
        }
        "sqr" | "sqrt" => unary(stack, |value| if value <= 0 { 0 } else { (value as f64).sqrt().round() as i32 }),
        "pow" => binary(stack, |a, b| {
            let exponent = b.clamp(-10, 10);
            if a == 0 || exponent < 0 { 0 } else { bounded((a as i64).saturating_pow(exponent as u32)) }
        }),
        "pyth" => binary(stack, |a, b| (((a as f64).powi(2) + (b as f64).powi(2)).sqrt().round() as i64).clamp(-MAX_INTEGER, MAX_INTEGER) as i32),
        "ceil" => binary(stack, i32::min),
        "floor" => binary(stack, i32::max),
        "root" => binary(stack, |a, b| if b == 0 { 0 } else { (a.abs() as f64).powf(1.0 / b.abs() as f64).round() as i32 }),
        "logx" => binary(stack, |a, b| if a == 0 || b.abs() < 2 { 0 } else { ((a.abs() as f64).ln() / (b.abs() as f64).ln()).round() as i32 }),
        "sin" => unary(stack, |value| ((value as f64 / 200.0).sin() * 32_000.0).round() as i32),
        "cos" => unary(stack, |value| ((value as f64 / 200.0).cos() * 32_000.0).round() as i32),
        "~" | "bitnot" => unary(stack, |value| !value),
        "&" | "bitand" => binary(stack, |a, b| a & b),
        "|" | "bitor" => binary(stack, |a, b| a | b),
        "bitxor" => binary(stack, |a, b| a ^ b),
        "++" | "bitinc" => unary(stack, i32::wrapping_add_one),
        "--" | "bitdec" => unary(stack, i32::wrapping_sub_one),
        "neg" => unary(stack, i32::wrapping_neg),
        "<<" => unary(stack, |value| value.wrapping_shl(1)),
        ">>" => unary(stack, |value| value.wrapping_shr(1)),
        "dereference" => { let address = pop(stack); push(stack, memory.read(address)); }
        _ => {}
    }
}

fn execute_condition(command: &str, integers: &mut IntegerStack, booleans: &mut BooleanStack) {
    let result = match command {
        "<" | "=<" => compare(integers, |a, b| a < b),
        ">" | "=>" => compare(integers, |a, b| a > b),
        "=" => compare(integers, |a, b| a == b),
        "!=" | "=!" => compare(integers, |a, b| a != b),
        ">=" => compare(integers, |a, b| a >= b),
        "<=" => compare(integers, |a, b| a <= b),
        "%=" => approximate(integers, false),
        "!%=" => approximate(integers, true),
        "~=" => custom_approximate(integers, false),
        "!~=" => custom_approximate(integers, true),
        _ => return,
    };
    push_bool(booleans, result);
}

fn execute_logic(command: &str, stack: &mut BooleanStack) {
    match command {
        "and" => { let b = pop_bool(stack).unwrap_or(true); let a = pop_bool(stack); push_bool(stack, a.map_or(b, |a| a && b)); }
        "or" => { let b = pop_bool(stack).unwrap_or(true); let a = pop_bool(stack); push_bool(stack, a.map_or(true, |a| a || b)); }
        "xor" => { let b = pop_bool(stack).unwrap_or(true); let a = pop_bool(stack); push_bool(stack, a.map_or(!b, |a| a ^ b)); }
        "not" => { let value = pop_bool(stack).unwrap_or(true); push_bool(stack, !value); }
        "true" => push_bool(stack, true),
        "false" => push_bool(stack, false),
        "dropbool" => { pop_bool(stack); }
        "clearbool" => stack.clear(),
        "dupbool" => { let value = pop_bool(stack).unwrap_or(true); push_bool(stack, value); push_bool(stack, value); }
        "swapbool" => { let b = pop_bool(stack).unwrap_or(true); let a = pop_bool(stack).unwrap_or(true); push_bool(stack, b); push_bool(stack, a); }
        "overbool" => { let b = pop_bool(stack).unwrap_or(true); let a = pop_bool(stack).unwrap_or(true); push_bool(stack, a); push_bool(stack, b); push_bool(stack, a); }
        _ => {}
    }
}

fn execute_store(command: &str, stack: &mut IntegerStack, memory: &mut VmMemory, random_state: &mut u64) -> bool {
    let address = pop(stack);
    if address == 0 {
        return false;
    }
    match command {
        "store" => memory.write(address, pop(stack)),
        "inc" => memory.write(address, memory.read(address).wrapping_add(1)),
        "dec" => memory.write(address, memory.read(address).wrapping_sub(1)),
        "+=" => memory.write(address, memory.read(address).saturating_add(pop(stack))),
        "-=" => memory.write(address, memory.read(address).saturating_sub(pop(stack))),
        "*=" => memory.write(address, bounded(memory.read(address) as i64 * pop(stack) as i64)),
        "/=" => { let value = pop(stack); memory.write(address, if value == 0 { 0 } else { memory.read(address) / value }); }
        "ceilstore" => { let value = pop(stack); memory.write(address, memory.read(address).min(value)); }
        "floorstore" => { let value = pop(stack); memory.write(address, memory.read(address).max(value)); }
        "rndstore" => { let value = pop(stack).unsigned_abs() as u64; memory.write(address, if value == 0 { 0 } else { (next_random(random_state) % (value + 1)) as i32 }); }
        "sgnstore" => memory.write(address, memory.read(address).signum()),
        "absstore" => memory.write(address, memory.read(address).saturating_abs()),
        "sqrstore" => memory.write(address, (memory.read(address).max(0) as f64).sqrt().round() as i32),
        "negstore" => memory.write(address, memory.read(address).wrapping_neg()),
        _ => return false,
    }
    true
}

fn resolve_address(program: &LegacyDna, name: &str) -> i32 {
    program.user_variable(name).or_else(|| sysvar_address(name)).unwrap_or(0)
}

fn normalize_address(address: i32) -> usize {
    let normalized = address.saturating_abs() % MAX_MEMORY;
    if normalized == 0 { MAX_MEMORY as usize } else { normalized as usize }
}

fn normalize_store(value: i32) -> i32 {
    if value.abs() > 32_000 { value % 32_000 } else { value }
}

fn bounded(value: i64) -> i32 {
    value.clamp(-MAX_INTEGER, MAX_INTEGER) as i32
}

fn push(stack: &mut IntegerStack, value: i32) {
    if stack.len() == 100 { stack.remove(0); }
    stack.push(value);
}

fn pop(stack: &mut IntegerStack) -> i32 { stack.pop().unwrap_or(0) }
fn unary(stack: &mut IntegerStack, operation: impl FnOnce(i32) -> i32) { let value = pop(stack); push(stack, operation(value)); }
fn binary(stack: &mut IntegerStack, operation: impl FnOnce(i32, i32) -> i32) { let b = pop(stack); let a = pop(stack); push(stack, operation(a, b)); }
fn compare(stack: &mut IntegerStack, operation: impl FnOnce(i32, i32) -> bool) -> bool { let b = pop(stack); let a = pop(stack); operation(a, b) }
fn push_bool(stack: &mut BooleanStack, value: bool) { if stack.len() == 100 { stack.remove(0); } stack.push(value); }
fn pop_bool(stack: &mut BooleanStack) -> Option<bool> { stack.pop() }

fn approximate(stack: &mut IntegerStack, invert: bool) -> bool {
    let b = pop(stack) as f64;
    let a = pop(stack) as f64;
    let tolerance = a / 10.0;
    let result = a - tolerance <= b && a + tolerance >= b;
    result ^ invert
}

fn custom_approximate(stack: &mut IntegerStack, invert: bool) -> bool {
    let percent = pop(stack) as f64;
    let b = pop(stack) as f64;
    let a = pop(stack) as f64;
    let tolerance = a / 100.0 * percent;
    let result = a - tolerance <= b && a + tolerance >= b;
    result ^ invert
}

fn is_store(command: &str) -> bool {
    matches!(command, "store" | "inc" | "dec" | "+=" | "-=" | "*=" | "/=" | "ceilstore" | "floorstore" | "rndstore" | "sgnstore" | "absstore" | "sqrstore" | "negstore")
}

fn is_condition(command: &str) -> bool {
    matches!(command, "<" | ">" | "=" | "!=" | "%=" | "!%=" | "~=" | "!~=" | ">=" | "<=" | "=>" | "=<" | "=!")
}

fn is_logic(command: &str) -> bool {
    matches!(command, "and" | "or" | "xor" | "not" | "true" | "false" | "dropbool" | "clearbool" | "dupbool" | "swapbool" | "overbool")
}

fn next_random(state: &mut u64) -> u64 {
    let mut value = *state;
    value ^= value << 13;
    value ^= value >> 7;
    value ^= value << 17;
    *state = value.max(1);
    value
}

trait WrappingOne {
    fn wrapping_add_one(self) -> Self;
    fn wrapping_sub_one(self) -> Self;
}

impl WrappingOne for i32 {
    fn wrapping_add_one(self) -> Self { self.wrapping_add(1) }
    fn wrapping_sub_one(self) -> Self { self.wrapping_sub(1) }
}
