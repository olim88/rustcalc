import init, { evaluate_latex } from '../pkg/rust_calculator.js';
import wasmBytes from '../pkg/rust_calculator_bg.wasm';

export interface EvaluateRequest {
	formula: string;
	previousLines: string[];
	approximate: boolean;
	precision: number;
}

const ERROR_LATEX = '⚡';

let ready = false;

export async function initCalculator(): Promise<void> {
	if (ready) {
		return;
	}

	await init(wasmBytes);
	ready = true;
}

export function evaluateLatex(request: EvaluateRequest): string {
	if (!ready) {
		return ERROR_LATEX;
	}

	try {
		return evaluate_latex(
			JSON.stringify({
				formula: request.formula,
				previous_lines: request.previousLines,
				approximate: request.approximate,
				precision: request.precision,
			}),
		);
	} catch (error) {
		console.error('Rust calculator failed:', error);
		return ERROR_LATEX;
	}
}

export function isCalculatorReady(): boolean {
	return ready;
}
