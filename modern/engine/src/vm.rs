use crate::{
    DnaBlock, DnaCosts, EngineError, LegacyDna, MAX_DNA_BLOCKS, TokenClass, sysvar_address,
};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

type IntegerStack = SmallVec<[i32; STACK_LIMIT]>;
type BooleanStack = SmallVec<[bool; STACK_LIMIT]>;

const STACK_LIMIT: usize = 100;
const MAX_MEMORY: i64 = 1_000;
const MAX_INTEGER: i64 = 2_000_000_000;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VmMemory {
    cells: Vec<(u16, i32)>,
}

impl VmMemory {
    pub fn read(&self, address: i32) -> i32 {
        let address = normalize_address(address) as u16;
        self.cells
            .binary_search_by_key(&address, |cell| cell.0)
            .map_or(0, |index| self.cells[index].1)
    }

    pub fn write(&mut self, address: i32, value: i32) {
        if address == 0 {
            return;
        }
        let address = normalize_address(address) as u16;
        let value = normalize_store(value);
        match self.cells.binary_search_by_key(&address, |cell| cell.0) {
            Ok(index) if value == 0 => {
                self.cells.remove(index);
            }
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

    pub fn entries(&self) -> impl Iterator<Item = (u16, i32)> + '_ {
        self.cells.iter().copied()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct VmContext<'a> {
    pub position: [f64; 2],
    pub costs: &'a DnaCosts,
    pub record_activations: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VmTermination {
    End,
    ProgramEnd,
    BlockLimit,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VmReport {
    pub blocks_executed: u32,
    pub stores_applied: u32,
    pub conditions_evaluated: u32,
    pub current_gene: u16,
    pub activations: Vec<bool>,
    pub debug_text: String,
    pub energy_cost: f64,
    pub termination: VmTermination,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivationFrame {
    pub tick: u64,
    pub activations: Vec<bool>,
}

pub struct DnaVm {
    random_state: u64,
}

impl DnaVm {
    pub fn new(seed: u64) -> Self {
        Self {
            random_state: seed.max(1),
        }
    }

    pub fn random_state(&self) -> u64 {
        self.random_state
    }

    pub fn execute(
        &mut self,
        program: &LegacyDna,
        memory: &mut VmMemory,
        context: VmContext<'_>,
    ) -> Result<VmReport, EngineError> {
        Ok(self.execute_blocks(program.blocks(), memory, context))
    }

    fn execute_blocks(
        &mut self,
        blocks: &[DnaBlock],
        memory: &mut VmMemory,
        context: VmContext<'_>,
    ) -> VmReport {
        let mut integers = IntegerStack::new();
        let mut booleans = BooleanStack::new();
        let mut flow = Flow::Clear;
        let mut branch_condition = true;
        let mut in_gene = false;
        let mut conditioned_else_pending = false;
        let mut report = VmReport {
            blocks_executed: 0,
            stores_applied: 0,
            conditions_evaluated: 0,
            current_gene: 0,
            activations: Vec::new(),
            debug_text: String::new(),
            energy_cost: blocks.len().saturating_sub(1) as f64
                * context.costs.dna_upkeep
                * context.costs.global_multiplier,
            termination: VmTermination::ProgramEnd,
        };

        for (index, block) in blocks.iter().copied().enumerate() {
            if report.blocks_executed as usize >= MAX_DNA_BLOCKS {
                report.termination = VmTermination::BlockLimit;
                break;
            }
            if block.class == TokenClass::End && block.value == 1 {
                report.termination = VmTermination::End;
                break;
            }
            report.blocks_executed = report.blocks_executed.saturating_add(1);

            match block.class {
                TokenClass::Number if flow != Flow::Clear => {
                    push(&mut integers, block.value);
                    charge(&mut report, context.costs.number, context.costs);
                }
                TokenClass::Dereference if flow != Flow::Clear => {
                    push(&mut integers, memory.read(block.value));
                    charge(&mut report, context.costs.dereference, context.costs);
                }
                TokenClass::Basic if flow != Flow::Clear => {
                    execute_basic(block.value, &mut integers, memory, &mut self.random_state);
                    charge(&mut report, context.costs.basic, context.costs);
                }
                TokenClass::Advanced if flow != Flow::Clear => {
                    execute_advanced(
                        block.value,
                        index + 1,
                        &mut integers,
                        &mut booleans,
                        context.position,
                        context.record_activations.then_some(&mut report.debug_text),
                    );
                    if block.value < 13 {
                        charge(&mut report, context.costs.advanced, context.costs);
                    }
                }
                TokenClass::Bitwise if flow != Flow::Clear => {
                    execute_bitwise(block.value, &mut integers);
                    charge(&mut report, context.costs.bitwise, context.costs);
                }
                TokenClass::Condition
                    if matches!(flow, Flow::Condition | Flow::Body | Flow::ElseBody) =>
                {
                    execute_condition(block.value, &mut integers, &mut booleans);
                    charge(&mut report, context.costs.condition, context.costs);
                }
                TokenClass::Logic
                    if matches!(flow, Flow::Condition | Flow::Body | Flow::ElseBody) =>
                {
                    execute_logic(block.value, &mut booleans);
                    charge(&mut report, context.costs.logic, context.costs);
                }
                TokenClass::Store
                    if matches!(flow, Flow::Body | Flow::ElseBody)
                        && booleans.last().copied().unwrap_or(true) =>
                {
                    if let Some(relative_cost) =
                        execute_store(block.value, &mut integers, memory, &mut self.random_state)
                    {
                        report.stores_applied = report.stores_applied.saturating_add(1);
                        charge(
                            &mut report,
                            context.costs.store * relative_cost,
                            context.costs,
                        );
                        mark_activation(&mut report, context.record_activations);
                    }
                }
                TokenClass::Flow => {
                    charge(&mut report, context.costs.flow, context.costs);
                    let closes_condition = flow == Flow::Condition && matches!(block.value, 2..=4);
                    if closes_condition {
                        branch_condition = booleans.iter().copied().all(|condition| condition);
                        booleans.clear();
                    }
                    if matches!(block.value, 2..=4) && matches!(flow, Flow::Body | Flow::ElseBody) {
                        mark_activation(&mut report, context.record_activations);
                    }
                    match block.value {
                        1 => {
                            flow = Flow::Condition;
                            booleans.clear();
                            report.current_gene = report.current_gene.saturating_add(1);
                            report.conditions_evaluated =
                                report.conditions_evaluated.saturating_add(1);
                            in_gene = true;
                            conditioned_else_pending = false;
                            ensure_activation_slot(&mut report, context.record_activations);
                        }
                        2 => {
                            let follows_condition = closes_condition;
                            if !follows_condition && !in_gene {
                                branch_condition = true;
                            }
                            if !in_gene {
                                report.current_gene = report.current_gene.saturating_add(1);
                                ensure_activation_slot(&mut report, context.record_activations);
                            }
                            in_gene = false;
                            conditioned_else_pending = follows_condition;
                            flow = if branch_condition {
                                Flow::Body
                            } else {
                                Flow::Clear
                            };
                        }
                        3 => {
                            if !in_gene && !conditioned_else_pending {
                                report.current_gene = report.current_gene.saturating_add(1);
                                ensure_activation_slot(&mut report, context.record_activations);
                            }
                            in_gene = false;
                            conditioned_else_pending = false;
                            flow = if branch_condition {
                                Flow::Clear
                            } else {
                                Flow::ElseBody
                            };
                        }
                        4 => {
                            flow = Flow::Clear;
                            branch_condition = true;
                            in_gene = false;
                            conditioned_else_pending = false;
                        }
                        _ => {}
                    }
                    memory.write_sysvar("thisgene", i32::from(report.current_gene));
                }
                TokenClass::Number
                | TokenClass::Dereference
                | TokenClass::Basic
                | TokenClass::Advanced
                | TokenClass::Bitwise
                | TokenClass::Condition
                | TokenClass::Logic
                | TokenClass::Store
                | TokenClass::Reserved
                | TokenClass::End => {}
            }
        }

        if report.termination == VmTermination::ProgramEnd
            && report.blocks_executed as usize >= MAX_DNA_BLOCKS
        {
            report.termination = VmTermination::BlockLimit;
        }
        report
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Flow {
    Clear,
    Condition,
    Body,
    ElseBody,
}

fn execute_basic(
    command: i32,
    stack: &mut IntegerStack,
    memory: &VmMemory,
    random_state: &mut u64,
) {
    match command {
        1 => binary(stack, legacy_add),
        2 => binary(stack, legacy_subtract),
        3 => binary(stack, |a, b| bounded(i64::from(a) * i64::from(b))),
        4 => binary(stack, |a, b| {
            if b == 0 {
                0
            } else {
                vb_round(f64::from(a) / f64::from(b))
            }
        }),
        5 => {
            let upper = pop(stack);
            push(stack, random_from_zero(random_state, upper));
        }
        6 => {
            let address = pop(stack);
            push(stack, memory.read(address));
        }
        7 => binary(stack, |a, b| if b == 0 { 0 } else { a % b }),
        8 => unary(stack, i32::signum),
        9 => unary(stack, i32::saturating_abs),
        10 => {
            let value = pop(stack);
            push(stack, value);
            push(stack, value);
        }
        11 => {
            pop(stack);
        }
        12 => stack.clear(),
        13 if stack.len() > 1 => {
            let top = stack.len() - 1;
            stack.swap(top, top - 1);
        }
        14 if stack.len() == 1 => push(stack, 0),
        14 if stack.len() > 1 => {
            let value = stack[stack.len() - 2];
            push(stack, value);
        }
        _ => {}
    }
}

fn execute_advanced(
    command: i32,
    position: usize,
    integers: &mut IntegerStack,
    booleans: &mut BooleanStack,
    robot_position: [f64; 2],
    debug_text: Option<&mut String>,
) {
    match command {
        1 => {
            let target_y = f64::from(pop(integers));
            let target_x = f64::from(pop(integers));
            let delta_x = target_x - robot_position[0];
            let delta_y = robot_position[1] - target_y;
            let mut angle = if delta_x == 0.0 {
                if delta_y < 0.0 {
                    std::f64::consts::PI * 1.5
                } else {
                    std::f64::consts::PI * 0.5
                }
            } else {
                let mut value = (delta_y / delta_x).atan();
                if delta_x < 0.0 {
                    value += std::f64::consts::PI;
                }
                value
            };
            while angle < 0.0 {
                angle += std::f64::consts::TAU;
            }
            while angle > std::f64::consts::TAU {
                angle -= std::f64::consts::TAU;
            }
            push(integers, vb_round(angle * 200.0));
        }
        2 => {
            let target_y = f64::from(pop(integers));
            let target_x = f64::from(pop(integers));
            let distance = ((robot_position[0] - target_x).powi(2)
                + (robot_position[1] - target_y).powi(2))
            .sqrt();
            push(integers, bounded_float(distance));
        }
        3 => binary(integers, i32::min),
        4 => binary(integers, i32::max),
        5 => unary(integers, |value| {
            if value <= 0 {
                0
            } else {
                vb_round(f64::from(value).sqrt())
            }
        }),
        6 => binary(integers, |a, b| {
            let exponent = b.clamp(-10, 10);
            if a == 0 {
                0
            } else {
                bounded_float(f64::from(a).powi(exponent))
            }
        }),
        7 => binary(integers, |a, b| {
            bounded_float(f64::from(a).hypot(f64::from(b)))
        }),
        8 => binary(integers, |a, b| {
            let mut left = a % 1_256;
            let mut right = b % 1_256;
            if left < 0 {
                left += 1_256;
            }
            if right < 0 {
                right += 1_256;
            }
            let mut difference = f64::from(left) / 200.0 - f64::from(right) / 200.0;
            if difference > std::f64::consts::PI {
                difference = -(std::f64::consts::TAU - difference);
            }
            if difference < -std::f64::consts::PI {
                difference += std::f64::consts::TAU;
            }
            vb_round(difference * 200.0)
        }),
        9 => binary(integers, |a, b| {
            let degree = b.saturating_abs();
            if degree == 0 {
                0
            } else {
                vb_round(f64::from(a.saturating_abs()).powf(1.0 / f64::from(degree)))
            }
        }),
        10 => binary(integers, |a, b| {
            let argument = a.saturating_abs();
            let base = b.saturating_abs();
            if argument == 0 || base < 2 {
                0
            } else {
                vb_round(f64::from(argument).ln() / f64::from(base).ln())
            }
        }),
        11 => unary(integers, |value| {
            vb_round((f64::from(value) / 200.0).sin() * 32_000.0)
        }),
        12 => unary(integers, |value| {
            vb_round((f64::from(value) / 200.0).cos() * 32_000.0)
        }),
        13 => {
            let value = pop(integers);
            if let Some(text) = debug_text {
                text.push_str(&format!("\r\n{value} at position {position}"));
            }
            push(integers, value);
        }
        14 => {
            let value = pop_bool(booleans);
            if let Some(text) = debug_text {
                let display = match value {
                    Some(true) => "True",
                    Some(false) => "False",
                    None => "True",
                };
                text.push_str(&format!("\r\n{display} at position {position}"));
            }
            push_bool(booleans, value.unwrap_or(true));
        }
        _ => {}
    }
}

fn execute_bitwise(command: i32, stack: &mut IntegerStack) {
    match command {
        1 => unary(stack, |value| !value),
        2 => binary(stack, |a, b| a & b),
        3 => binary(stack, |a, b| a | b),
        4 => binary(stack, |a, b| a ^ b),
        5 => unary(stack, |value| value.wrapping_add(1)),
        6 => unary(stack, |value| value.wrapping_sub(1)),
        7 => unary(stack, i32::wrapping_neg),
        8 => unary(stack, |value| value.wrapping_shl(1)),
        9 => unary(stack, |value| value.wrapping_shr(1)),
        _ => {}
    }
}

fn execute_condition(command: i32, integers: &mut IntegerStack, booleans: &mut BooleanStack) {
    let result = match command {
        1 => compare(integers, |a, b| a < b),
        2 => compare(integers, |a, b| a > b),
        3 => compare(integers, |a, b| a == b),
        4 => compare(integers, |a, b| a != b),
        5 => approximate(integers, false),
        6 => approximate(integers, true),
        7 => custom_approximate(integers, false),
        8 => custom_approximate(integers, true),
        9 => compare(integers, |a, b| a >= b),
        10 => compare(integers, |a, b| a <= b),
        _ => return,
    };
    push_bool(booleans, result);
}

fn execute_logic(command: i32, stack: &mut BooleanStack) {
    match command {
        1 => {
            let b = pop_bool(stack).unwrap_or(true);
            let a = pop_bool(stack);
            push_bool(stack, a.map_or(b, |value| value && b));
        }
        2 => {
            let b = pop_bool(stack).unwrap_or(true);
            let a = pop_bool(stack);
            push_bool(stack, a.is_none_or(|value| value || b));
        }
        3 => {
            let b = pop_bool(stack).unwrap_or(true);
            let a = pop_bool(stack);
            push_bool(stack, a.map_or(!b, |value| value ^ b));
        }
        4 => {
            let value = pop_bool(stack).unwrap_or(true);
            push_bool(stack, !value);
        }
        5 => push_bool(stack, true),
        6 => push_bool(stack, false),
        7 => {
            pop_bool(stack);
        }
        8 => stack.clear(),
        9 if !stack.is_empty() => {
            let value = *stack.last().expect("non-empty stack");
            push_bool(stack, value);
        }
        10 if stack.len() > 1 => {
            let top = stack.len() - 1;
            stack.swap(top, top - 1);
        }
        11 if stack.len() == 1 => push_bool(stack, true),
        11 if stack.len() > 1 => {
            let value = stack[stack.len() - 2];
            push_bool(stack, value);
        }
        _ => {}
    }
}

fn execute_store(
    command: i32,
    stack: &mut IntegerStack,
    memory: &mut VmMemory,
    random_state: &mut u64,
) -> Option<f64> {
    if !(1..=14).contains(&command) {
        return None;
    }
    let address = pop(stack);
    let binary_value = (7..=9).contains(&command).then(|| pop(stack));
    if address == 0 {
        return None;
    }

    let relative_cost = match command {
        1 => {
            memory.write(address, pop(stack));
            1.0
        }
        2 => {
            memory.write(address, memory.read(address).wrapping_add(1));
            0.1
        }
        3 => {
            memory.write(address, memory.read(address).wrapping_sub(1));
            0.1
        }
        4 => {
            memory.write(address, memory.read(address).saturating_add(pop(stack)));
            0.2
        }
        5 => {
            memory.write(address, memory.read(address).saturating_sub(pop(stack)));
            0.2
        }
        6 => {
            let value = normalize_store(pop(stack));
            memory.write(
                address,
                bounded(i64::from(memory.read(address)) * i64::from(value)),
            );
            0.2
        }
        7 => {
            let value = binary_value.expect("binary store value was captured");
            memory.write(
                address,
                if value == 0 {
                    0
                } else {
                    vb_round(f64::from(memory.read(address)) / f64::from(value))
                },
            );
            0.2
        }
        8 => {
            let limit = binary_value.expect("binary store value was captured");
            memory.write(address, memory.read(address).min(limit));
            0.2
        }
        9 => {
            let limit = binary_value.expect("binary store value was captured");
            memory.write(address, memory.read(address).max(limit));
            0.2
        }
        10 => {
            let current = memory.read(address);
            let magnitude = current.unsigned_abs() as u64;
            let value = if magnitude == 0 {
                0
            } else {
                (next_random(random_state) % (magnitude + 1)) as i32 * current.signum()
            };
            memory.write(address, value);
            1.0 / 7.0
        }
        11 => {
            memory.write(address, memory.read(address).signum());
            1.0 / 7.0
        }
        12 => {
            memory.write(address, memory.read(address).saturating_abs());
            0.125
        }
        13 => {
            let value = memory.read(address);
            memory.write(
                address,
                if value > 0 {
                    vb_round(f64::from(value).sqrt())
                } else {
                    0
                },
            );
            1.0 / 7.0
        }
        14 => {
            memory.write(address, memory.read(address).wrapping_neg());
            0.125
        }
        _ => unreachable!(),
    };
    Some(relative_cost)
}

fn legacy_add(a: i32, b: i32) -> i32 {
    let mut result = i64::from(a) % MAX_INTEGER + i64::from(b) % MAX_INTEGER;
    if result.abs() > MAX_INTEGER {
        result -= result.signum() * MAX_INTEGER;
    }
    result as i32
}

fn legacy_subtract(a: i32, b: i32) -> i32 {
    let mut result = i64::from(a) % MAX_INTEGER - i64::from(b) % MAX_INTEGER;
    if result.abs() > MAX_INTEGER {
        result -= result.signum() * MAX_INTEGER;
    }
    result as i32
}

fn normalize_address(address: i32) -> usize {
    let normalized = i64::from(address).abs() % MAX_MEMORY;
    if normalized == 0 {
        MAX_MEMORY as usize
    } else {
        normalized as usize
    }
}

fn normalize_store(value: i32) -> i32 {
    if value > 0 {
        let remainder = value % 32_000;
        if remainder == 0 { 32_000 } else { remainder }
    } else if value < 0 {
        let remainder = value % 32_000;
        if remainder == 0 { -32_000 } else { remainder }
    } else {
        0
    }
}

fn bounded(value: i64) -> i32 {
    value.clamp(-MAX_INTEGER, MAX_INTEGER) as i32
}

fn bounded_float(value: f64) -> i32 {
    vb_round(value.clamp(-(MAX_INTEGER as f64), MAX_INTEGER as f64))
}

fn vb_round(value: f64) -> i32 {
    value
        .round_ties_even()
        .clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32
}

fn push(stack: &mut IntegerStack, value: i32) {
    if stack.len() == STACK_LIMIT {
        stack.remove(0);
    }
    stack.push(value);
}

fn pop(stack: &mut IntegerStack) -> i32 {
    stack.pop().unwrap_or(0)
}

fn unary(stack: &mut IntegerStack, operation: impl FnOnce(i32) -> i32) {
    let value = pop(stack);
    push(stack, operation(value));
}

fn binary(stack: &mut IntegerStack, operation: impl FnOnce(i32, i32) -> i32) {
    let b = pop(stack);
    let a = pop(stack);
    push(stack, operation(a, b));
}

fn compare(stack: &mut IntegerStack, operation: impl FnOnce(i32, i32) -> bool) -> bool {
    let b = pop(stack);
    let a = pop(stack);
    operation(a, b)
}

fn push_bool(stack: &mut BooleanStack, value: bool) {
    if stack.len() == STACK_LIMIT {
        stack.remove(0);
    }
    stack.push(value);
}

fn pop_bool(stack: &mut BooleanStack) -> Option<bool> {
    stack.pop()
}

fn approximate(stack: &mut IntegerStack, invert: bool) -> bool {
    let b = f64::from(pop(stack));
    let a = f64::from(pop(stack));
    let tolerance = a / 10.0;
    let result = a - tolerance <= b && a + tolerance >= b;
    result ^ invert
}

fn custom_approximate(stack: &mut IntegerStack, invert: bool) -> bool {
    let percent = f64::from(pop(stack));
    let b = f64::from(pop(stack));
    let a = f64::from(pop(stack));
    let mut tolerance = a / 100.0 * percent;
    if invert {
        tolerance = tolerance.clamp(-(MAX_INTEGER as f64), MAX_INTEGER as f64);
    }
    let result = a - tolerance <= b && a + tolerance >= b;
    result ^ invert
}

fn random_from_zero(state: &mut u64, upper: i32) -> i32 {
    if upper >= 0 {
        (next_random(state) % (u64::from(upper as u32) + 1)) as i32
    } else {
        -((next_random(state) % u64::from(upper.unsigned_abs())) as i32)
    }
}

fn next_random(state: &mut u64) -> u64 {
    let mut value = *state;
    value ^= value << 13;
    value ^= value >> 7;
    value ^= value << 17;
    *state = value.max(1);
    value
}

fn charge(report: &mut VmReport, category_cost: f64, costs: &DnaCosts) {
    report.energy_cost += category_cost * costs.global_multiplier;
}

fn ensure_activation_slot(report: &mut VmReport, record: bool) {
    if record {
        report
            .activations
            .resize(usize::from(report.current_gene) + 1, false);
    }
}

fn mark_activation(report: &mut VmReport, record: bool) {
    if record {
        ensure_activation_slot(report, true);
        report.activations[usize::from(report.current_gene)] = true;
    }
}
