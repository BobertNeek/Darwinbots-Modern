# Darwinbots 2.48.32 DNA runtime specification

## 1. Model

Darwinbots DNA is a compact, stack-based, reverse-Polish language interpreted once per living, DNA-enabled robot per simulation cycle. A source file is plain text. The loader tokenizes it into an array of two-integer blocks: `tipo` selects a token class and `value` selects a literal or operation within that class. The runtime implementation is [`DNA.bas`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/DNA.bas); parsing and the built-in symbol table are in [`DNATokenizing.bas`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/DNATokenizing.bas).

Every robot owns:

- A tokenized DNA array ending in the master `end` token.
- 1,000 signed integer memory cells.
- Mutation parameters and mutation history.
- Twenty hereditary memory locations at 971–990, with 15 staged through a birth tie.
- Fifteen internal epigenetic staging values.
- A current-gene counter and an optional gene-activation bitmap for inspection.

The interpreter uses a process-global integer stack and Boolean stack, each with capacity 100. Both are cleared before each robot executes, so values never carry between robots or cycles. Memory is the persistent state.

## 2. Source format and parser

Whitespace separates tokens. Tabs become spaces. A single quote begins a comment; a slash in column one also denotes a comment/header line. An inline single quote removes the remainder of a code line. The parser is case-insensitive for commands and system names.

`def name address` declares a robot-file-local alias. Built-in `.name` tokens resolve through the compiled system-variable table. Prefixing a numeric or symbolic address with `*` dereferences robot memory. For example:

```text
def threshold 50

cond
*.nrg *threshold >
start
10 .up store
stop
end
```

The loader appends `end` even when the source omits it. Unknown tokens parse as numeric zero rather than halting the load. Old `shp` and `use` directives remain in comments but are disabled in 2.48.32.

Header comments may contain `'#generation:`, `'#mutations:`, `'#tag:`, and `'#hash:` fields. A lightweight 20-character hash protects recorded generation/mutation metadata from casual changes; on mismatch, those counts reset. It is not cryptographic authentication.

## 3. Token classes

| `tipo` | Class | Meaning |
|---:|---|---|
| 0 | number | Push signed literal |
| 1 | `*number` | Normalize address, read memory, push value |
| 2 | basic | Integer arithmetic and stack operations |
| 3 | advanced | Geometry, roots, powers, trigonometry, debug |
| 4 | bitwise | Bit operations and unary negation |
| 5 | condition | Pop integer operands and push Boolean result |
| 6 | logic | Operate on Boolean stack |
| 7 | store | Write or transform robot memory |
| 8 | reserved | No current behavior |
| 9 | flow | `cond`, `start`, `else`, `stop` |
| 10 | master flow | `end` terminates the genome |

DNA length includes the end block in runtime bookkeeping. The interpreter starts at array element 1 and stops at `tipo=10,value=1`, an array bound, or the 32,000-token safety bound.

## 4. Numeric and address rules

Integer-stack values are signed `Long` values, generally constrained to approximately ±2,000,000,000 by individual operations. Memory values use Darwinbots' signed ±32,000 convention. A memory write uses a custom modulo rule: positive multiples of 32,000 become 32,000; negative multiples become -32,000; other values use remainder modulo 32,000.

Memory addresses normalize as follows:

1. Address zero is a no-op for store commands.
2. Any other address becomes `abs(address) mod 1000`.
3. A remainder of zero maps to address 1000.

This rule applies to explicit dereference and store-family commands, making random or mutated addresses likely to remain meaningful. The complete compiled name-to-address table is [the sysvar reference](SYSVARS_REFERENCE.md).

Popping an empty integer stack returns zero. The Boolean stack uses an internal empty sentinel; conditions with no explicit Boolean result default toward true in several flow/logic paths. Over-capacity pushes discard old bottom entries through the stack implementation.

## 5. Execution and gene flow

At the start of DNA execution, the runtime clears both stacks, resets condition counts and debug text, and clears the activation bitmap when inspection requires it.

The canonical gene pattern is:

```text
cond
  integer expressions and conditions
start
  actions and stores for true
else
  actions and stores for false
stop
```

`cond` clears the Boolean stack, enters condition mode, and advances the gene number. Conditions may appear in a body as well as the formal condition area. On `start`, all Boolean values left by the condition block are ANDed. A true result enables the start body. `else` enables the alternate body only for false. `stop` clears flow state. A body with no preceding `cond` is true by default.

Numbers and integer operators execute in any non-clear flow state. Conditions and Boolean logic execute in condition or body states. Store-family operations execute only in a body selected by the current condition. A store marks the current gene active for the focus/console inspector.

Memory writes are immediate during the robot's DNA pass. Physical actions are mostly deferred because the engine reads command memory later in the cycle. A later gene can therefore overwrite an earlier gene's `.shoot`, `.repro`, movement, or tie request before that subsystem consumes it.

## 6. Basic integer operations

Stack notation below writes the older item first and the top item last.

| Token | Stack behavior | Edge behavior |
|---|---|---|
| `add` | `a b → a+b` | Inputs modulo 2 billion; overflow wrapped once |
| `sub` | `a b → a-b` | Same bound treatment |
| `mult` | `a b → a×b` | Saturates magnitude at 2 billion |
| `div` | `a b → a/b` | Division by zero yields 0; VB numeric conversion rounds to integer stack |
| `rnd` | `a → random(0,a)` | Uses Darwinbots random helper |
| `*` | `address → mem[address]` | Normalizes to 1–1000 |
| `mod` | `a b → a mod b` | Zero divisor consumes `a` and yields 0 |
| `sgn` | `a → -1,0,1` | Sign only |
| `abs` | `a → |a|` | — |
| `dup`, `dupint` | `a → a a` | — |
| `drop`, `dropint` | `a →` | Empty pop is harmless |
| `clear`, `clearint` | `… →` | Clears integer stack |
| `swap`, `swapint` | `a b → b a` | Stack helper defines empty defaults |
| `over`, `overint` | `a b → a b a` | Copies second item |

Each executed basic command subtracts the configured Basic DNA Command cost times the global cost multiplier.

## 7. Advanced operations

Angles use 200 units per radian. A full turn is approximately 1,256 units.

| Token | Stack behavior and semantics |
|---|---|
| `angle` | `x y → angle from current robot to (x,y)`, normalized to a turn and scaled by 200 |
| `dist` | `x y → Euclidean distance from current robot to (x,y)` using display/world divisors |
| `ceil` | `value maximum → min(value, maximum)` |
| `floor` | `value minimum → max(value, minimum)` |
| `sqr` | Positive square root; nonpositive input yields 0 |
| `pow` | `a b → a^b`; exponent magnitude is capped at 10; result capped at 2 billion |
| `pyth` | `a b → sqrt(a²+b²)`, capped at 2 billion |
| `anglecmp` | Normalizes two angles into 0–1255 and returns their shortest signed difference |
| `root` | `a b → abs(a)^(1/abs(b))`; zero root yields 0 |
| `logx` | `a b → log(abs(a))/log(abs(b))`; base below 2 or zero argument yields 0 |
| `sin` | `a → sin(a/200)×32000` |
| `cos` | `a → cos(a/200)×32000` |
| `debugint` | Pops integer, appends value and DNA position to robot debug text, then restores value |
| `debugbool` | Equivalent Boolean-stack inspection |

The configurable Advanced cost applies to the first twelve. Debug commands are excluded from that charge and ignored during mutation processing.

## 8. Bitwise operations

| Token | Behavior |
|---|---|
| `~` | Complement the internal double-word bit representation |
| `&` | Bitwise AND |
| `\|` | Bitwise OR |
| `^` | Bitwise XOR; exponentiation is the separate `pow` token |
| `++` | Increment bit representation |
| `--` | Decrement bit representation |
| `-` | Unary arithmetic negation |
| `<<` | Shift left one position |
| `>>` | Shift right one position |

All use the configured Bitwise cost. The conversion helpers define signed bit representation and overflow behavior.

## 9. Conditions and Boolean logic

Binary comparisons consume `a b` and compare in that written order despite the reverse pop sequence.

| Token | True when |
|---|---|
| `<` | `a < b` |
| `>` | `a > b` |
| `=` | `a = b` |
| `!=` | `a ≠ b` |
| `%=` | `b` lies within `a ± a/10` |
| `!%=` | Not `%=` |
| `~=` | `a b percent`: `b` lies within `a ± a×percent/100` |
| `!~=` | Not `~=`; tolerance capped against 2-billion overflow |
| `>=` | `a ≥ b` |
| `<=` | `a ≤ b` |

Because the tolerance uses signed `a`, approximate comparisons around negative anchors can behave counterintuitively. This is shipped behavior.

Boolean operators are `and`, `or`, `xor`, `not`, `true`, `false`, `dropbool`, `clearbool`, `dupbool`, `swapbool`, and `overbool`. Empty-stack defaults are deliberately permissive: an empty condition block evaluates true, and logic handlers substitute true or a related fallback when operands are absent.

## 10. Store operations

All stores act on normalized memory addresses. A destination of literal zero does nothing and costs nothing.

| Token | Input and effect | Relative store cost |
|---|---|---:|
| `store` | `value address`; replace memory | 1 |
| `inc` | `address`; add 1 | 1/10 |
| `dec` | `address`; subtract 1 | 1/10 |
| `addstore` | `value address`; memory += value | 1/5 |
| `substore` | `value address`; memory -= value | 1/5 |
| `multstore` | `value address`; memory *= normalized value | 1/5 |
| `divstore` | `value address`; memory /= value; zero yields 0 | 1/5 |
| `ceilstore` | `limit address`; lower memory to limit if above | 1/5 |
| `floorstore` | `limit address`; raise memory to limit if below | 1/5 |
| `rndstore` | `address`; replace with random value from zero to its magnitude, preserving sign | 1/7 |
| `sgnstore` | `address`; replace with sign | 1/7 |
| `absstore` | `address`; replace with absolute value | 1/8 |
| `sqrstore` | `address`; replace positive value with square root, otherwise zero | 1/7 |
| `negstore` | `address`; negate value | 1/8 |

The relative fractions multiply the configured Store cost and global multiplier. Writes to the `.tieang1`–`.tieang4` and `.tielen1`–`.tielen4` addresses also set overwrite flags so the tie subsystem does not immediately replace explicit DNA commands with sensed values.

## 11. Command-memory lifecycle

Memory cells serve three roles:

- Persistent private memory: values remain until DNA or an external effect changes them.
- Sensor/readback memory: the engine writes age, energy, body, eyes, reference values, touch, taste, ties, position, velocity, and other state.
- Command registers: the engine consumes and usually clears them later in the cycle.

Important one-shot resets include `.shoot`, `.shootval`, `.aimshoot`, `.backshot`, `.strbody`, `.fdbody`, `.mkchlr`, `.rmchlr`, `.vshoot`, `.mkvirus` after firing, and many tie registers. Movement registers are interpreted during force calculation and cleared by the position/update paths. A bot author must not assume every sysvar persists merely because it occupies ordinary memory.

Sensors are not all sampled at one instant. Touch and shot taste are generated by collision processing. Vision/reference, pain/pleasure, body gain/loss, position, daytime, and other senses are refreshed later during robot update. DNA therefore acts primarily on the prior cycle's completed sensory state. The main product spec defines exact cycle order.

## 12. Costs and failure behavior

Token execution subtracts category-specific energy costs for numbers, dereferences, basic, advanced, bitwise, conditions, logic, stores, and flow. The cost editor can make these zero, positive, or under advanced settings negative. DNA upkeep separately charges per DNA block each cycle, and copying charges apply during reproduction/virus construction.

The interpreter is mutation-tolerant by design:

- Unknown tokens become zero.
- Bad addresses wrap into memory.
- Empty stack reads use defaults.
- Division by zero returns zero.
- Most large results wrap or clamp.
- Missing `end` is bounded by array and 32,000-token checks.

These rules are not just defensive programming. They increase the chance that randomly mutated genomes remain executable and selectable.

## 13. Complete language acceptance cases

A faithful runtime must pass at least these behavioral checks:

- Parse and detokenize every token listed in this document.
- Resolve all names and aliases in [the canonical sysvar table](SYSVARS_REFERENCE.md).
- Append `end` and tolerate unknown source tokens.
- Clear both stacks for each robot.
- Preserve immediate within-genome memory writes.
- Enforce body/else flow and permissive empty conditions.
- Match address and ±32,000 normalization, including exact multiples.
- Track `.thisgene`, condition count, and gene activations.
- Charge configurable per-token costs.
- Load and save generation/mutation header metadata with the legacy hash behavior.
- Continue executing damaged but bounded genomes without crossing the 32,000-block limit.
