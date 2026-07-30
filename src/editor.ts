import {syntaxTree} from '@codemirror/language';
import {RangeSetBuilder} from '@codemirror/state';
import {
	Decoration,
	DecorationSet,
	EditorView,
	PluginSpec,
	PluginValue,
	ViewPlugin,
	ViewUpdate,
} from '@codemirror/view';
import {evaluateLatex} from './calculator';
import RustCalcPlugin from './main';
import {ResultWidget} from './widget';

class RustCalcHintRenderer implements PluginValue {
	decorations: DecorationSet;

	constructor(view: EditorView) {
		this.decorations = this.buildDecorations(view);
	}

	update(_update: ViewUpdate) {
		this.decorations = this.buildDecorations(_update.view);
	}

	destroy() {
	}

	buildDecorations(view: EditorView): DecorationSet {
		const builder = new RangeSetBuilder<Decoration>();

		for (const {from, to} of view.visibleRanges) {
			const cursorPos = view.state.selection.main.from;
			let mathBegin: number | null = null;
			let previousLines: string[] = [];

			syntaxTree(view.state).iterate({
				from,
				to,
				enter(node) {
					if (nodeTagsIncludes(node.type.name, 'formatting-math-begin')) {
						mathBegin = node.to;
					}
					if (
						nodeTagsIncludes(node.type.name, 'formatting-math-end') &&
						mathBegin != null
					) {
						const mathEnd = node.from;


						const latexContentLines = view.state
							.sliceDoc(mathBegin, mathEnd);

						const trimmedLatexLine = latexContentLines.split('\n').join("")
							.trim();

						if (cursorPos < mathBegin || mathEnd < cursorPos) {
							//if the line is before the current equation keep it to possible use variables
							previousLines.push(trimmedLatexLine)
							return;
						}

						const settings = RustCalcPlugin.INSTANCE.settings;

						if (
							!trimmedLatexLine.endsWith(
								settings.calculationTriggerString,
							) &&
							!trimmedLatexLine.endsWith(
								settings.approxCalculationTriggerString,
							)
						) {
							return;
						}

						const calcTrigger = new RegExp(
							`${settings.calculationTriggerString}|${settings.approxCalculationTriggerString.replace('\\', '\\\\')}`,
						);

						const isApproximation = trimmedLatexLine.endsWith(
							settings.approxCalculationTriggerString,
						);

						const splitFormula: string[] = splitOutsideBrackets(trimmedLatexLine, calcTrigger)
							.filter((part) => part.replace('\\\\', '').trim().length > 0);
						const formula = splitFormula[splitFormula.length - 1];
						if (!formula) return;


						const result = evaluateLatex({
							formula: formula,
							previousLines,
							approximate: isApproximation,
							precision: settings.approxDecimalPrecision,
							shiftForExact: settings.shiftForExact
						});

						let insertIndex =
							mathBegin +
							latexContentLines.trimEnd().length;


						builder.add(
							insertIndex,
							insertIndex,
							Decoration.replace({
								widget: new ResultWidget(
									view,
									insertIndex,
									` ${result}`,
								),
							}),
						);
					}
				},
			});
		}

		return builder.finish();
	}
}
/**
 * Splits `str` on matches of `regex`, but only when the match occurs
 * outside of any (), [], or {} bracket nesting.
 * Backslash-escaped brackets (\{, \}, \(, \)) are treated as literal
 * characters, not grouping brackets.
 */
function splitOutsideBrackets(str: string, regex: RegExp): string[] {
	const flags = regex.flags.includes('g') ? regex.flags : regex.flags + 'g';
	const globalRegex = new RegExp(regex.source, flags);

	const OPEN: Record<string, number> = { '(': 1, '[': 1, '{': 1 };
	const CLOSE: Record<string, number> = { ')': -1, ']': -1, '}': -1 };

	const depthAt: number[] = new Array<number>(str.length + 1).fill(0);
	let depth = 0;
	for (let i = 0; i < str.length; i++) {
		const ch = str.charAt(i);
		const isEscaped = str.charAt(i - 1) === '\\';
		if (!isEscaped) {
			if (OPEN[ch]) depth += 1;
			else if (CLOSE[ch]) depth = Math.max(0, depth - 1);
		}
		depthAt[i + 1] = depth;
	}

	// If brackets never balance out (more opens than closes), we can't
	// reliably tell "inside" from "outside" for the rest of the string.
	// Fall back to a plain split rather than silently blocking every
	// remaining match.
	if (depthAt[str.length] !== 0) {
		return str.split(globalRegex);
	}

	const parts: string[] = [];
	let lastIndex = 0;
	let match: RegExpExecArray | null;
	while ((match = globalRegex.exec(str)) !== null) {
		const matchStart = match.index;
		if (depthAt[matchStart] === 0) {
			parts.push(str.slice(lastIndex, matchStart));
			lastIndex = matchStart + match[0].length;
		}
		if (match[0].length === 0) globalRegex.lastIndex++;
	}
	parts.push(str.slice(lastIndex));
	return parts;
}

function nodeTagsIncludes(nodeTypeName: string, tag: string): boolean {
	return nodeTypeName.split('_').includes(tag);
}

const pluginSpec: PluginSpec<RustCalcHintRenderer> = {
	decorations: (value: RustCalcHintRenderer) => value.decorations,
};

export const rustCalcHintRenderer = ViewPlugin.fromClass(
	RustCalcHintRenderer,
	pluginSpec,
);
