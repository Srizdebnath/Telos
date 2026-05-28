# Telos Language Specification (`claude.md`)

This document provides a comprehensive, highly structured engineering specification for the **Telos** ($\tau\epsilon\lambda o\varsigma$) programming language. It is optimized for structural ingestion by advanced LLMs, autonomous agents, and compiler engineers. Telos is a systems programming language designed to unify bare-metal performance, compile-time memory safety without garbage collection, strict structural type safety, and formal mathematical verification of intent.

---

## 1. Formal Grammar, Type Foundations & Intent Syntax

Telos treats human intent as a first-class citizen within its type and grammar engine, merging execution paths with mathematical specifications.

### Formal Grammar (EBNF Blueprint)

The grammar explicitly separates operational syntax from specification syntax via the `intent`, `satisfies`, and `assert` blocks.

```ebnf
Program             ::= ( ModuleDeclaration | ImportDeclaration | TopLevelDeclaration )* ;
TopLevelDeclaration ::= StructDeclaration | FunctionDeclaration | IntentDeclaration ;

IntentDeclaration   ::= "intent" Identifier '(' ParameterList ')' '{' IntentBody '}' ;
IntentBody          ::= [ Preconditions ] [ Postconditions ] [ Invariants ] ;
Preconditions       ::= "preconditions" ':' '[' ExpressionList ']' ;
Postconditions      ::= "postconditions" ':' '[' ExpressionList ']' ;
Invariants          ::= "invariants"     ':' '[' ExpressionList ']' ;

FunctionDeclaration ::= [ Visibility ] "fn" Identifier '(' ParameterList ')' [ "->" Type ] [ "satisfies" Identifier ] Block ;

```

### Type Foundations

Telos implements a static, structural type system with **Refinement Types** and **Dependent Types**. This allows values to be constrained mathematically during compilation.

* **Primitive Types:** `Int`, `Float`, `Bool`, `String`, `Char`, `Byte`.
* **Refinement Types:** Types paired with a predicate logic constraint. For example, `type PositiveInt = Int where value > 0`.
* **Dependent Types:** Types whose definitions depend on runtime values, evaluated statically by the compiler when inputs are bounded (e.g., `Tensor<Float, dims>`).

### Intent Syntax in Action

The following code snippet shows how a safety-critical banking subsystem is defined. The compiler ensures that the logical contract is structurally verified before compiling machine primitives.

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

---

## 2. Linear Ownership & Zero-Cost Memory Management

Telos achieves bare-metal speeds with zero garbage collection overhead by implementing a compile-time **Linear Ownership and Affine Type System**.

### Memory Management Axioms

1. **Single Owner:** Every resource allocation has a single owner variable.
2. **Move Semantics:** Assigning an owned variable to another context transfers ownership, invalidating the previous variable identifier.
3. **Controlled Borrowing:** Resources can be borrowed via references (`&` for immutable, `&mut` for mutable).

### Borrow Checker Mechanics

The compiler evaluates reference lifetimes statically via explicit or inferred lifetime variables (`'a`). The borrow checker enforces these two core rules:

* Any number of immutable references (`&T`) can exist simultaneously for a resource.
* Only **one** mutable reference (`&mut T`) can exist at a time, completely exposing the data and blocking any other access.

```telos
fn process_payload(payload: UserData) {
    let internal_ref = &payload; // Immutable borrow
    let mut_ref = &mut payload;  // COMPILE ERROR: Cannot borrow mutably while immutably borrowed.
}

```

Unlike Rust, Telos uses its ownership lifetimes to feed topological memory trace data directly into the intent verification solver. This allows the compiler to guarantee that data race conditions are structurally impossible across async tasks.

---

## 3. SMT Integration & Intent Verification Engine

The core differentiator of Telos is its automated verification engine. It sits directly inside the compilation loop to resolve the **Semantic Intent Gap**.

### The Verification Loop

When a function includes a `satisfies <Intent>` clause, the compiler converts the function's structural operations and the intent's predicates into an intermediate mathematical formula. This formula is passed directly to an integrated, highly optimized instance of a **Satisfiability Modulo Theories (SMT)** solver (built on native Z3 core components).

```
          [ Telos EIR Core Subsystem ]
                       │
                       ▼
    [ Convert Code Blocks to Logical Predicates ]
                       │
                       ▼
  [ Negate the Intent Postcondition (Goal: Find Bugs) ]
                       │
                       ▼
      [ Execute SMT Solver Engine (Z3 Core) ]
         ⚡               ⚡
         │               │
  (If SATISFIABLE)       (If UNSATISFIABLE)
         │               │
         ▼               ▼
 [ Counter-Example Found ] [ Mathematical Proof Absolute ]
 [ HALT COMPILATION ]      [ EMIT TO BACKEND LOWERING ]

```

### Proving Correctness, Not Guessing

The solver does not run tests. It mathematically checks every possible input state. To prove that a function is correct, the engine attempts to prove the *opposite*: it searches for any input that satisfies the preconditions but violates the postconditions.

* If the solver finds a valid conflict state (**SAT**), the compiler rejects the build. It outputs a detailed counter-example trace explaining exactly which input values trigger the failure.
* If no conflict state exists (**UNSAT**), the compiler has mathematically proven the code is correct, and execution moves to the backend.

---

## 4. Multi-Stage Staged Compilation & EIR Pipeline

The Telos compilation framework operates via a series of highly specialized processing steps to ensure zero logical leakage between user scripts and machine binaries.

```
[ .tl Source ] ──> [ Parser ] ──> [ Structural AST ] ──> [ Borrow Checker ]
                                                                │
                                                                ▼
[ Native Machine Code ] <── [ LLVM ] <── [ LLVM IR ] <── [ EIR Subsystem ]

```

### 1. Abstract Syntax Tree (AST) Generation

The source code text is converted into an AST, validating language grammar rules, modules, and basic layout mappings.

### 2. High-Level Intermediate Representation (HIR)

The AST is lowered to HIR to run semantic analysis, type resolution, structural type inference, and affine borrow-checking.

### 3. Execution Intermediate Representation (EIR)

This is the critical "semantic freeze point" of Telos. The EIR decouples operational instructions from raw code text and couples them directly with their corresponding mathematical constraints. The EIR contains:

* **The Data Flow Graph:** Highlighting strict memory layout lifespans and state updates.
* **The Constraint Assert Layer:** Translating refinement types and intent blocks into static logic formulas.

### 4. Code Generation

Once the EIR is formally cleared by the SMT verification engine, it drops directly into standard LLVM IR or clean C targets, dropping all intent overhead and producing highly optimized machine binaries.

---

## 5. Standard Library Architecture & Pre-baked Axioms

The Telos Standard Library (`std`) provides a collection of safe data structures, utility components, and pre-baked mathematical axioms.

### Core Architecture

The standard library is organized into minimal, isolated modules to keep dependencies lean and compile times fast:

* `std::sys` : Direct bare-metal interfaces, memory pinning, and system operations.
* `std::alloc` : Deterministic heap allocators for specialized bare-metal environments.
* `std::collections` : Fast data structures (vectors, hash maps, rings) verified to prevent buffer overflows and out-of-bounds indexing.

### Pre-baked Axioms

`std` includes fundamental mathematical axioms that form the backbone of safe computing. These are pre-verified rules that the SMT engine can use without re-proving them during every compilation step.

```telos
module std::math::axioms

// Universal verification contract for non-overflowing summation
pub axiom SafeAddition(a: Int, b: Int) {
    preconditions: [
        b > 0 -> a < Int::MAX - b,
        b < 0 -> a > Int::MIN - b
    ]
}

```

---

## 6. Bare-Metal Execution & LLVM Lowering Layers

Telos delivers execution performance matching C and Rust by eliminating runtime interpreters, garbage collection tracking, and virtual machines.

### LLVM Translation Mechanics

Once the EIR passes type, memory, and intent verification, it is translated into **LLVM Intermediate Representation (LLVM IR)**. During this transition:

* All `intent`, `precondition`, and `postcondition` blocks are completely removed from the pipeline. They exist solely to guide the compiler and cause zero runtime performance penalties.
* Linear ownership boundaries are transformed into explicit, deterministic allocation and deallocation statements (`llvm.lifetime.start` / `llvm.lifetime.end`).

### Bare-Metal Optimizations

By leveraging the LLVM compiler infrastructure, Telos automatically utilizes deep hardware optimizations:

* **Aggressive Devirtualization:** Because types are evaluated structurally and statically, indirect function calls are flattened into direct execution jumps wherever possible.
* **Autovectorization:** Loops verified as data-race-free are compiled directly into SIMD (Single Instruction, Multiple Data) instructions.
* **Link-Time Optimization (LTO):** Telos cross-compiles modules into monolithic binary layouts, allowing deep optimization passes across distinct package targets.

---

## 7. Adversarial Test Suite & Differential Fuzzing

Because Telos relies heavily on mathematical proof checking, it includes an aggressive internal testing pipeline designed to find loopholes in its own compiler components.

### Differential Fuzzing Subsystem

The Telos repository runs continuous **Differential Fuzzing** tests. This framework feeds identical random operations into the Telos compiler pipeline, a verified reference interpreter, and an isolated target language runtime (like Rust). Any variation in execution paths, memory footprints, or logic outputs flags an immediate compilation defect.

```
                         ┌──> Telos Native Binary ──> Output A
                         │
[ Random Fuzz Payload ] ─┼──> Ref Reference Model ──> Output B
                         │
                         ┌──> Clang/GCC Environment ──> Output C
                         │
                         ▼
             [ Divergence Evaluation ] ──> If A != B != C -> Alert Defect

```

### Adversarial Intent Testing

To stress-test the built-in SMT solver engine, the testing framework generates complex, contradictory, and intentionally flawed code blocks. The test passes *only* if the compiler correctly identifies the logical flaws and cleanly blocks compilation. If a collection of flawed logic compiles successfully, the engine flags a validation failure.

---

## 8. CI/CD Guardrails & Multi-Agent PR Protocols

To ensure smooth development workflows across distributed teams and automated AI coding agents, the repository enforces a strict, deterministic continuous integration pipeline.

### Commit and Pull Request Gates

Every pull request submitted to the repository must pass through three automated validation gates:

1. **Gate 1: The Linter & Formatter (`telos fmt --check`):** Enforces explicit, standardized style layouts and blocks unconventional code styles.
2. **Gate 2: Bootstrap Verification:** The proposed updates must successfully re-compile the entire language framework itself without emitting warnings or optimization drops.
3. **Gate 3: The Proof Verification Regression Suite:** Runs thousands of complex intent validation tests to ensure new updates don't break existing logical contracts.

### Multi-Agent Integration Rules

For AI systems contributing to the repository, pull requests are monitored by automated reviewer bots:

* **Static Scope Bounds:** Agents can only modify files within their assigned sub-modules (e.g., `src/frontend/`). Cross-module updates require multi-maintainer approvals.
* **Exhaustive Specification Requirement:** Any PR that changes a core function block must also update its corresponding `intent` specification. Pure code updates without updated mathematical proofs are automatically blocked.

---

## 9. Bootstrapping Phases & Self-Hosting Strategy

To eliminate external dependencies and prove the security of its own ecosystem, Telos follows a rigorous, multi-stage self-hosting strategy.

### Phase 1: The Bootstrap Compiler (`Stage 0`)

The initial Telos compiler core is built using pure **Rust**. This temporary compiler parses basic Telos files, runs the initial borrow checking models, and lowers code using basic LLVM targets.

* *Limitation:* The Stage 0 compiler is slow and lacks advanced SMT integration. Its main job is compiling the next generation of the engine.

### Phase 2: The Self-Hosted Transition (`Stage 1`)

The entire Telos source tree is rewritten entirely in **Telos syntax**. The Stage 0 compiler compiles this source tree, producing a native Telos compiler binary (`Stage 1`).

### Phase 3: Total Ecosystem Self-Hosting (`Stage 2`)

The `Stage 1` compiler is used to compile its own source code again. This step generates the production-ready `Stage 2` compiler.

```
[ Telos Source in Telos ] ──> Compiled by [ Stage 0 (Rust) ] ──> [ Stage 1 Binary ]
                                                                        │
                                                                        ▼
[ Telos Source in Telos ] ──> Compiled by [ Stage 1 (Telos) ] ──> [ Stage 2 (Production Engine) ]

```

When the Stage 2 compiler matches the Stage 1 compiler binary bit-for-bit, the language achieves **Total Self-Hosting Correctness**. From that point on, the language's core memory safety and intent validation are completely verified by Telos itself.