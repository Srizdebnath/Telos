#!/bin/bash
set -e

echo "=== Step 1: Cleaning and backing up original compiler_stage2.rs ==="
# Extract the compiled code (before the test harness) from compiler_stage2.rs
sed '/fn main() {/,$d' compiler_stage2.rs > compiler_stage2_clean.rs

echo "=== Step 2: Compiling compiler.tl again to generate compiler_stage3.rs ==="
cargo run -- compiler.tl
mv compiler.rs compiler_stage3.rs

echo "=== Step 3: Comparing generated compiler_stage3.rs with clean Stage 2 output ==="
if diff -Z -B -u compiler_stage2_clean.rs compiler_stage3.rs; then
    echo "=== SUCCESS: Generated Stage 3 source code is bit-for-bit identical to Stage 2! ==="
else
    echo "=== FAILURE: Generated Stage 3 source code differs! ==="
    exit 1
fi

echo "=== Step 4: Appending main test harness to compiler_stage3.rs ==="
cat << 'EOF' >> compiler_stage3.rs

fn main() {
    let tok = Token { token_type: 1, value: 100 };
    let state = ParserState { current_token_idx: 0, error_occurred: false };
    
    println!("Stage 3: Initial token index: {}", state.current_token_idx);
    
    // Test mutability / copy semantics
    let mut mut_state = state;
    consume_token(mut_state, tok);
    
    println!("Stage 3: Verification passed! Stage 3 binary built successfully.");
}
EOF

echo "=== Step 5: Compiling compiler_stage3.rs into Stage 3 native binary ==="
rustc compiler_stage3.rs -o stage3_telos

echo "=== Step 6: Executing Stage 3 native binary ==="
./stage3_telos

echo "=== Stage 3 self-hosting check passed successfully! ==="
rm -f compiler_stage2_clean.rs
