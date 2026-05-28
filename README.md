# Telos ($\tau\epsilon\lambda o\varsigma$) Language Compiler

Telos is an **Intent-Oriented Systems Programming Language (IOPL)** designed to unify bare-metal execution performance, compile-time affine memory safety, and formal mathematical verification of program intent. Telos bridges the **Semantic Intent Gap** by making logical contracts first-class compiler constructs.

---

## Key Features

1. **Formal Intent Verification Loop**:
   - Uses **Satisfiability Modulo Theories (SMT)** via an integrated Z3 solver.
   - Translates code blocks into Single Static Assignment (SSA) mathematical formulas.
   - Assertions of preconditions and negated postconditions are evaluated at compile time. Compilation halts immediately with counterexamples if a bug is found.
2. **Refinement Types**:
   - Supports predicate logic constraints on types (e.g. `type PositiveInt = Int where value > 0`).
   - Recursively resolves type alias references and verifies refinement assertions on local bindings (`let`) and assignments at compile time.
3. **Invariant Tracking Loop**:
   - Enforces safety-critical constraints across mutational state changes. Checks pre-invariants on entry, mid-block invariants after mutating statements, and terminal invariants.
4. **Linear Ownership & Affine Types**:
   - Enforces single-owner resource management.
   - Prevents mutable/immutable aliasing and resource racing during compilation.
5. **Zero-Cost Abstractions**:
   - Verification code (`intent`, `preconditions`, `postconditions`, `invariants`, `satisfies`, type refinements) exists solely for compilation guardrails.
   - The transpiler strips these assertions at target generation, translating Telos directly to raw, high-performance machine code.

---

## Codebase Architecture

The Stage 0 compiler is written in **Rust** and consists of:
- `src/ast.rs`: Abstract Syntax Tree representation of Telos types, expressions, and structures.
- `src/lexer.rs`: Lexical analyzer parsing keywords, operators, and literals.
- `src/parser.rs`: Recursive descent parser supporting structs, functions, parameter mutability, and unary references (`&` and `&mut`).
- `src/borrow_checker.rs`: Formally tracks mutable and immutable borrow boundaries.
- `src/verifier.rs`: Maps AST code structures to Z3 variables and runs SAT checks.
- `src/codegen.rs`: Translates verified Telos constructs into native Rust output.
- `src/main.rs`: CLI compiler interface driving the compilation pipeline.

---

## Bootstrapping & Self-Hosting

Telos features a multi-generational bootstrapping pipeline to establish compiler stability:
- **Stage 0 (Rust)**: The initial bootstrap compiler.
- **Stage 1 (Self-Hosted)**: The core compiler token consumption and state machine logic written in Telos syntax (`compiler.tl`), compiled using Stage 0.
- **Stages 2–7 (Equivalence & Stability Check)**: Subsequent generations of self-hosted compilers. In each stage, the compiler compiles its own source tree (`compiler.tl`) and verifies bit-for-bit equivalence against the previous run.

---

## How to Build & Run

### Prerequisites
Make sure `libz3` is installed and registered with `pkg-config` (verified compatible on this machine).

### Running Telos Source Files
Compile a Telos source file (e.g., the wallet payouts engine example):
```bash
cargo run -- banking_test.tl --transpile
```

To run only checking passes (Verification & Borrow Checking):
```bash
cargo run -- banking_test.tl --verify-only
```

### Running Self-Hosting Bootstrap Scripts
Run Stage 1 bootstrap:
```bash
./build_stage1.sh
```

Validate idempotence and bit-for-bit correctness up to Stage 7:
```bash
./build_stage2.sh
./build_stage3.sh
./build_stage4.sh
./build_stage5.sh
./build_stage6.sh
./build_stage7.sh
```

---

## Examples

### Correct Wallet Payout (`banking_test.tl`)
```telos
module FinancialEngine

type Wallet = struct {
    owner_id: String,
    balance: Int,
}

intent StrictTransfer(source: Wallet, target: Wallet, amount: Int) {
    preconditions: [
        amount > 0,
        source.balance >= amount
    ],
    postconditions: [
        source.balance == initial(source.balance) - amount,
        target.balance == initial(target.balance) + amount
    ]
}

pub fn execute_payout(mut source: Wallet, mut target: Wallet, amount: Int) satisfies StrictTransfer {
    source.balance -= amount;
    target.balance += amount;
}
```
*Compiles successfully and transpiles to Rust.*

### Buggy Wallet Payout (`buggy_banking_test.tl`)
```telos
pub fn execute_payout(mut source: Wallet, mut target: Wallet, amount: Int) satisfies StrictTransfer {
    source.balance -= amount;
    target.balance += amount - 1; // Logical bug!
}
```
*Fails compilation immediately with a counterexample:*
```
Verification Failed! Counter-example found:
  source (struct Wallet):
    .balance = 1
  target (struct Wallet):
    .balance = 0
  amount = 1
```

### Borrow Checker Failure (`borrow_fail.tl`)
```telos
pub fn process_payload(mut payload: UserData) {
    let internal_ref = &payload;
    let mut_ref = &mut payload; // Aliasing violation!
}
```
*Fails compilation during the borrow checking phase:*
```
Borrow Checker Error in function 'process_payload': Cannot borrow 'payload' mutably because it is already borrowed immutably
```
