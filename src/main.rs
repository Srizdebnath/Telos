pub mod ast;
pub mod lexer;
pub mod parser;
pub mod borrow_checker;
pub mod verifier;
pub mod codegen;

use std::collections::HashMap;
use std::env;
use std::fs;
use std::process;
use lexer::Lexer;
use parser::Parser;
use borrow_checker::BorrowChecker;
use verifier::Verifier;
use codegen::Codegen;
use ast::TopLevel;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <file.tl> [--transpile] [--verify-only]", args[0]);
        process::exit(1);
    }

    let file_path = &args[1];
    let transpile_flag = args.contains(&"--transpile".to_string());
    let verify_only_flag = args.contains(&"--verify-only".to_string());

    let source = match fs::read_to_string(file_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading file {}: {}", file_path, e);
            process::exit(1);
        }
    };

    // 1. Lexer
    let mut lexer = Lexer::new(&source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Lexer Error: {}", e);
            process::exit(1);
        }
    };

    // 2. Parser
    let mut parser = Parser::new(tokens);
    let program = match parser.parse_program() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Parser Error: {}", e);
            process::exit(1);
        }
    };

    // Gather structs, intents, and functions
    let mut structs = Vec::new();
    let mut intents = HashMap::new();
    let mut functions = Vec::new();



    for decl in &program.declarations {
        match decl {
            TopLevel::Struct(s) => structs.push(s.clone()),
            TopLevel::Intent(i) => {
                intents.insert(i.name.clone(), i.clone());
            }
            TopLevel::Function(f) => functions.push(f.clone()),
            _ => {}
        }
    }

    // 3. Borrow Checker
    let mut bc = BorrowChecker::new();
    for func in &functions {
        if let Err(e) = bc.check_function(func) {
            eprintln!("Borrow Checker Error in function '{}': {}", func.name, e);
            process::exit(1);
        }
    }
    println!("Borrow Checker passed.");

    // 4. Verifier (Z3 SMT solver)
    let cfg = z3::Config::new();
    let ctx = z3::Context::new(&cfg);
    
    for func in &functions {
        if let Some(ref intent_name) = func.satisfies {
            let intent = match intents.get(intent_name) {
                Some(i) => i,
                None => {
                    eprintln!("Verification Error: Function '{}' satisfies undefined intent '{}'", func.name, intent_name);
                    process::exit(1);
                }
            };

            let mut verifier = Verifier::new(&ctx, &structs);
            match verifier.verify_function(func, intent) {
                Ok(_) => {
                    println!("Verification Succeeded for function '{}' against intent '{}'.", func.name, intent_name);
                }
                Err(e) => {
                    eprintln!("Verification Failed for function '{}' satisfies '{}':\n{}", func.name, intent_name, e);
                    process::exit(1);
                }
            }
        }
    }

    if verify_only_flag {
        println!("All checks passed successfully.");
        return;
    }

    // 5. Codegen
    let transpiled = Codegen::transpile(&program);
    if transpile_flag {
        println!("// --- Transpiled Rust output ---");
        println!("{}", transpiled);
    } else {
        let output_path = file_path.replace(".tl", ".rs");
        if let Err(e) = fs::write(&output_path, &transpiled) {
            eprintln!("Error writing transpiled file {}: {}", output_path, e);
            process::exit(1);
        }
        println!("Compilation succeeded. Written transpiled file to: {}", output_path);
    }
}
