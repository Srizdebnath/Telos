# Changelog - Telos Compiler Project

All notable changes to the Telos compiler bootstrap chain are documented here.

## [Stage 0] - Initial Rust Compiler
### Added
- **Lexical Analyzer (`src/lexer.rs`)**: Complete tokenizer for Telos syntax (identifiers, literals, operators, comments).
- **Parser (`src/parser.rs`)**: Custom recursive descent parser to output structural Abstract Syntax Trees. Added support for unary reference borrow parsing (`&` and `&mut`).
- **Borrow Checker (`src/borrow_checker.rs`)**: Exclusivity borrow tracker preventing aliasing conflicts (mutable borrowing during immutable reference scopes).
- **SMT Verifier (`src/verifier.rs`)**: Z3-based mathematical proof verifier. Translates function operations to Static Single Assignment (SSA) form, asserts preconditions, negates postconditions, and runs check loops. Prints detailed counterexamples on verification failures.
- **Transpiler Codegen (`src/codegen.rs`)**: Translates validated Telos AST structures and logic directly to standard, zero-overhead Rust code.
- **Main Driver CLI (`src/main.rs`)**: Integrates the frontend components and transpiler backend.

## [Stage 1] - Self-Hosted Bootstrap
### Added
- **Compiler Source Tree in Telos (`compiler.tl`)**: Implemented parsing token consumption states and functions written in Telos syntax. Uses `intent` and `satisfies` blocks to mathematically prove consumption safety logic.
- **Stage 1 Bootstrapping Script (`build_stage1.sh`)**: Automates compiling `compiler.tl` via the Stage 0 compiler to generate `compiler.rs`, appending a test harness, compiling using `rustc`, and verifying execution.

## [Stage 2] - Total Self-Hosting Correctness
### Added
- **Stage 2 Verification Pipeline (`build_stage2.sh`)**: Runs the Stage 1 compiler against its own source tree `compiler.tl` to output `compiler_stage2.rs`. Compiles and verifies the resulting `stage2_telos` binary, verifying bit-for-bit equivalence of generated source layouts to prove compiler idempotence and self-hosting correctness.

## [Stage 3] - Bootstrap Stability Validation
### Added
- **Stage 3 Idempotence Validation (`build_stage3.sh`)**: Compiles `compiler.tl` using compiled Stage 2 outputs to produce `compiler_stage3.rs`. Performs bit-for-bit source verification against clean Stage 2 output to guarantee absolute compilation stability and idempotence across successive self-hosted compile generations. Builds and validates the `stage3_telos` binary.

## [Stage 4] - Bootstrap Stability Validation Generation II
### Added
- **Stage 4 Idempotence Validation (`build_stage4.sh`)**: Compiles `compiler.tl` using compiled Stage 3 outputs to produce `compiler_stage4.rs`. Performs bit-for-bit source verification against clean Stage 3 output to guarantee absolute compiler stability and idempotence across successive self-hosted compile generations. Builds and validates the `stage4_telos` binary.

## [Stage 5] - Bootstrap Stability Validation Generation III
### Added
- **Stage 5 Idempotence Validation (`build_stage5.sh`)**: Compiles `compiler.tl` using compiled Stage 4 outputs to produce `compiler_stage5.rs`. Performs bit-for-bit source verification against clean Stage 4 output to guarantee absolute compiler stability and idempotence across successive self-hosted compile generations. Builds and validates the `stage5_telos` binary.

## [Stage 6] - Bootstrap Stability Validation Generation IV
### Added
- **Stage 6 Idempotence Validation (`build_stage6.sh`)**: Compiles `compiler.tl` using compiled Stage 5 outputs to produce `compiler_stage6.rs`. Performs bit-for-bit source verification against clean Stage 5 output to guarantee absolute compiler stability and idempotence across successive self-hosted compile generations. Builds and validates the `stage6_telos` binary.

## [Stage 7] - Bootstrap Stability Validation Generation V
### Added
- **Stage 7 Idempotence Validation (`build_stage7.sh`)**: Compiles `compiler.tl` using compiled Stage 6 outputs to produce `compiler_stage7.rs`. Performs bit-for-bit source verification against clean Stage 6 output to guarantee absolute compiler stability and idempotence across successive self-hosted compile generations. Builds and validates the `stage7_telos` binary.
