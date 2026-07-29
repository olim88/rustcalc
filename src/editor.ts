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

						const splitFormula = trimmedLatexLine
							.split(calcTrigger)
							.filter(
								(part) => part.replace('\\\\', '').trim().length > 0,
							);
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
