pub mod ast;
pub mod parser;
pub mod interpreter;
pub mod ai;
pub mod error;

use wasm_bindgen::prelude::*;
use crate::parser::parse;
use crate::interpreter::run_program;

#[wasm_bindgen]
pub fn run_velora(code: &str) -> String {
    match parse(code) {
        Ok(program) => {
            match run_program(&program) {
                Ok(output) => output.join("\n"),
                Err(e) => format!("Runtime error: {}", e),
            }
        }
        Err(e) => format!("Parse error: {}", e),
    }
}

#[wasm_bindgen]
pub fn validate_velora(code: &str) -> bool {
    parse(code).is_ok()
}
