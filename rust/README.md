# Rust calculator core

This folder contains the LaTeX evaluation logic for the plugin. The Obsidian
plugin calls it through WebAssembly; you can also test it locally from the CLI.

## Where to edit

Replace the body of `evaluate_latex_impl` in `src/evaluator.rs`:

```rust
pub fn evaluate_latex_impl(request: &EvaluateRequest) -> Result<String, String> {
    // request.formula        -> LaTeX expression to evaluate
    // request.previous_lines -> earlier lines in the same math block
    // request.approximate    -> true when triggered with \approx
    // request.precision      -> decimal places for approximations (-1 = max)
}
```

Return `Ok(latex_string)` on success or `Err(message)` on failure. The WASM
layer converts errors into `⚡` in the editor.

## Local testing (no Obsidian)

Run the CLI with a JSON request:

```bash
cargo run --bin rust-calc -- '{"formula":"1+2","previous_lines":[],"approximate":false,"precision":-1}'
```

Run unit tests:

```bash
cargo test
```

## Build WASM for the plugin

From the plugin root:

```bash
npm run build:wasm
```

This writes generated bindings to `../pkg`, which TypeScript imports at build time.
