import { syntaxTree } from '@codemirror/language';
import { RangeSetBuilder } from '@codemirror/state';
import {
	Decoration,
	DecorationSet,
	EditorView,
	PluginSpec,
	PluginValue,
	ViewPlugin,
	ViewUpdate,
} from '@codemirror/view';
import { evaluateLatex } from './calculator';
import RustCalcPlugin from './main';
import { ResultWidget } from './widget';

class RustCalcHintRenderer implements PluginValue {
	decorations: DecorationSet;

	constructor(view: EditorView) {
		this.decorations = this.buildDecorations(view);
	}

	update(_update: ViewUpdate) {
		this.decorations = this.buildDecorations(_update.view);
	}

	destroy() {}

	buildDecorations(view: EditorView): DecorationSet {
		const builder = new RangeSetBuilder<Decoration>();

		for (const { from, to } of view.visibleRanges) {
			const cursorPos = view.state.selection.main.from;
			let mathBegin: number | null = null;

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

						if (cursorPos < mathBegin || mathEnd < cursorPos) return;
						const relativeCursorPos = cursorPos - mathBegin;

						const latexContentLines = view.state
							.sliceDoc(mathBegin, mathEnd)
							.split('\n');
						const focusedLatexLine =
							latexContentLines.find(
								(_line, i) =>
									relativeCursorPos <
									latexContentLines.slice(0, i + 1).join('\n').length + 1,
							) ?? '';
						const trimmedLatexLine = focusedLatexLine
							.replace('\\\\', '')
							.trim();
						const previousLatexLines = latexContentLines.slice(
							0,
							latexContentLines.indexOf(focusedLatexLine),
						);

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

						const splitFormula = focusedLatexLine
							.split(calcTrigger)
							.filter(
								(part) => part.replace('\\\\', '').trim().length > 0,
							);
						const formula = splitFormula[splitFormula.length - 1];
						if (!formula) return;


						const previousLines = previousLatexLines.map((line) =>
							line.replace('\\\\', '').replace('&', '').trim(),
						);

						const result = evaluateLatex({
							formula: formula,
							previousLines,
							approximate: isApproximation,
							precision: settings.approxDecimalPrecision,
						});

						let insertIndex =
							mathBegin +
							previousLatexLines.join('\n').length +
							focusedLatexLine.trimEnd().length;
						if (previousLatexLines.length > 0) insertIndex += 1;

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
