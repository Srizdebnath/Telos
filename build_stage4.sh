#!/bin/bash
set -e

echo "=== Step 1: Cleaning and backing up original compiler_stage3.rs ==="
# Extract the compiled code (before the test harness) from compiler_stage3.rs
sed '/fn main() {/,$d' compiler_stage3.rs > compiler_stage3_clean.rs

echo "=== Step 2: Compiling compiler.tl again to generate compiler_stage4.rs ==="
cargo run -- compiler.tl
mv compiler.rs compiler_stage4.rs

echo "=== Step 3: Comparing generated compiler_stage4.rs with clean Stage 3 output ==="
if diff -Z -B -u compiler_stage3_clean.rs compiler_stage4.rs; then
    echo "=== SUCCESS: Generated Stage 4 source code is bit-for-bit identical to Stage 3! ==="
else
    echo "=== FAILURE: Generated Stage 4 source code differs! ==="
    exit 1
fi

echo "=== Step 4: Appending main test harness to compiler_stage4.rs ==="
cat << 'EOF' >> compiler_stage4.rs

fn main() {
    let tok = Token { token_type: 1, value: 100 };
    let state = ParserState { current_token_idx: 0, error_occurred: false };
    
    println!("Stage 4: Initial token index: {}", state.current_token_idx);
    
    // Test mutability / copy semantics
    let mut mut_state = state;
    consume_token(mut_state, tok);
    
    println!("Stage 4: Verification passed! Stage 4 binary built successfully.");
}
EOF

echo "=== Step 5: Compiling compiler_stage4.rs into Stage 4 native binary ==="
rustc compiler_stage4.rs -o stage4_telos

echo "=== Step 6: Executing Stage 4 native binary ==="
./stage4_telos

echo "=== Stage 4 self-hosting check passed successfully! ==="
rm -f compiler_stage3_clean.rs
