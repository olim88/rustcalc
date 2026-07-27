//! CLI helper for testing the Rust evaluator outside Obsidian.
//!
//! Usage:
//!   cargo run --bin rust-calc -- '{"formula":"1+2","previous_lines":[],"approximate":false,"precision":-1}'

use rust_calculator::{evaluate_latex_impl, EvaluateRequest};
use std::{env, process};

fn main() {
	let request_json = env::args().nth(1).unwrap_or_else(|| {
		eprintln!(
			"Usage: rust-calc '{{\"formula\":\"1+2\",\"previous_lines\":[],\"approximate\":false,\"precision\":-1}}'"
		);
		process::exit(1);
	});

	let request: EvaluateRequest = match serde_json::from_str(&request_json) {
		Ok(request) => request,
		Err(error) => {
			eprintln!("Invalid JSON request: {error}");
			process::exit(1);
		}
	};

	match evaluate_latex_impl(&request) {
		Ok(result) => println!("{result}"),
		Err(error) => {
			eprintln!("{error}");
			process::exit(1);
		}
	}
}
