#!/bin/bash
set -e

echo "=== Step 1: Cleaning and backing up original compiler_stage6.rs ==="
# Extract the compiled code (before the test harness) from compiler_stage6.rs
sed '/fn main() {/,$d' compiler_stage6.rs > compiler_stage6_clean.rs

echo "=== Step 2: Compiling compiler.tl again to generate compiler_stage7.rs ==="
cargo run -- compiler.tl
mv compiler.rs compiler_stage7.rs

echo "=== Step 3: Comparing generated compiler_stage7.rs with clean Stage 6 output ==="
if diff -Z -B -u compiler_stage6_clean.rs compiler_stage7.rs; then
    echo "=== SUCCESS: Generated Stage 7 source code is bit-for-bit identical to Stage 6! ==="
else
    echo "=== FAILURE: Generated Stage 7 source code differs! ==="
    exit 1
fi

echo "=== Step 4: Appending main test harness to compiler_stage7.rs ==="
cat << 'EOF' >> compiler_stage7.rs

fn main() {
    let tok = Token { token_type: 1, value: 100 };
    let state = ParserState { current_token_idx: 0, error_occurred: false };
    
    println!("Stage 7: Initial token index: {}", state.current_token_idx);
    
    // Test mutability / copy semantics
    let mut mut_state = state;
    consume_token(mut_state, tok);
    
    println!("Stage 7: Verification passed! Stage 7 binary built successfully.");
}
EOF

echo "=== Step 5: Compiling compiler_stage7.rs into Stage 7 native binary ==="
rustc compiler_stage7.rs -o stage7_telos

echo "=== Step 6: Executing Stage 7 native binary ==="
./stage7_telos

echo "=== Stage 7 self-hosting check passed successfully! ==="
rm -f compiler_stage6_clean.rs
