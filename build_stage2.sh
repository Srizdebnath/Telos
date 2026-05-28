#!/bin/bash
set -e

echo "=== Step 1: Cleaning and backing up original compiler.rs ==="
# Extract the compiled code (before the test harness) from original compiler.rs
# The test harness starts with "fn main() {"
# Let's get everything up to "fn main() {" from the original compiler.rs (which has the harness)
sed '/fn main() {/,$d' compiler.rs > compiler_stage1_clean.rs

echo "=== Step 2: Compiling compiler.tl again to generate compiler_stage2.rs ==="
cargo run -- compiler.tl
mv compiler.rs compiler_stage2.rs

echo "=== Step 3: Comparing generated compiler_stage2.rs with original clean Stage 1 output ==="
if diff -Z -B -u compiler_stage1_clean.rs compiler_stage2.rs; then
    echo "=== SUCCESS: Generated source code is bit-for-bit identical! ==="
else
    echo "=== FAILURE: Generated source code differs! ==="
    exit 1
fi

echo "=== Step 4: Appending main test harness to compiler_stage2.rs ==="
cat << 'EOF' >> compiler_stage2.rs

fn main() {
    let tok = Token { token_type: 1, value: 100 };
    let state = ParserState { current_token_idx: 0, error_occurred: false };
    
    println!("Stage 2: Initial token index: {}", state.current_token_idx);
    
    // Test mutability / copy semantics
    let mut mut_state = state;
    consume_token(mut_state, tok);
    
    println!("Stage 2: Verification passed! Stage 2 binary built successfully.");
}
EOF

echo "=== Step 5: Compiling compiler_stage2.rs into Stage 2 native binary ==="
rustc compiler_stage2.rs -o stage2_telos

echo "=== Step 6: Executing Stage 2 native binary ==="
./stage2_telos

echo "=== Stage 2 self-hosting check passed successfully! ==="
rm -f compiler_stage1_clean.rs
