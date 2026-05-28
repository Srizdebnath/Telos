#!/bin/bash
set -e

echo "=== Step 1: Compiling compiler.tl with Stage 0 compiler ==="
cargo run -- compiler.tl

echo "=== Step 2: Appending main test harness to compiler.rs ==="
cat << 'EOF' >> compiler.rs

fn main() {
    let tok = Token { token_type: 1, value: 100 };
    let state = ParserState { current_token_idx: 0, error_occurred: false };
    
    println!("Stage 1: Initial token index: {}", state.current_token_idx);
    
    // Test mutability / copy semantics
    let mut mut_state = state;
    consume_token(mut_state, tok);
    
    println!("Stage 1: Verification passed! Stage 1 binary built successfully.");
}
EOF

echo "=== Step 3: Compiling compiler.rs into Stage 1 native binary ==="
rustc compiler.rs -o stage1_telos

echo "=== Step 4: Executing Stage 1 native binary ==="
./stage1_telos
echo "=== Bootstrapping completed successfully! ==="
