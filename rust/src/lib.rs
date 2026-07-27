mod evaluator;

pub use evaluator::{evaluate_latex_impl, EvaluateRequest};

use evaluator::evaluate_latex_impl as evaluate_impl;
use wasm_bindgen::prelude::*;

const ERROR_LATEX: &str = "⚡";

/// WASM entry point: JSON request in, LaTeX result out.
#[wasm_bindgen]
pub fn evaluate_latex(request_json: &str) -> String {
	match serde_json::from_str::<EvaluateRequest>(request_json) {
		Ok(request) => evaluate_impl(&request).unwrap_or_else(|e| e), //todo don't show error like this when finised debugging
		Err(_) => ERROR_LATEX.to_string(),
	}
}
