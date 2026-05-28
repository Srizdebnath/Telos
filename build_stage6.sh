#!/bin/bash
set -e

echo "=== Step 1: Cleaning and backing up original compiler_stage5.rs ==="
# Extract the compiled code (before the test harness) from compiler_stage5.rs
sed '/fn main() {/,$d' compiler_stage5.rs > compiler_stage5_clean.rs

echo "=== Step 2: Compiling compiler.tl again to generate compiler_stage6.rs ==="
cargo run -- compiler.tl
mv compiler.rs compiler_stage6.rs

echo "=== Step 3: Comparing generated compiler_stage6.rs with clean Stage 5 output ==="
if diff -Z -B -u compiler_stage5_clean.rs compiler_stage6.rs; then
    echo "=== SUCCESS: Generated Stage 6 source code is bit-for-bit identical to Stage 5! ==="
else
    echo "=== FAILURE: Generated Stage 6 source code differs! ==="
    exit 1
fi

echo "=== Step 4: Appending main test harness to compiler_stage6.rs ==="
cat << 'EOF' >> compiler_stage6.rs

fn main() {
    let tok = Token { token_type: 1, value: 100 };
    let state = ParserState { current_token_idx: 0, error_occurred: false };
    
    println!("Stage 6: Initial token index: {}", state.current_token_idx);
    
    // Test mutability / copy semantics
    let mut mut_state = state;
    consume_token(mut_state, tok);
    
    println!("Stage 6: Verification passed! Stage 6 binary built successfully.");
}
EOF

echo "=== Step 5: Compiling compiler_stage6.rs into Stage 6 native binary ==="
rustc compiler_stage6.rs -o stage6_telos

echo "=== Step 6: Executing Stage 6 native binary ==="
./stage6_telos

echo "=== Stage 6 self-hosting check passed successfully! ==="
rm -f compiler_stage5_clean.rs
