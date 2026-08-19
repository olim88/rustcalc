import { EditorView, WidgetType } from '@codemirror/view';
import RustCalcPlugin from './main';

export class ResultWidget extends WidgetType {

	static activeWidget: ResultWidget | null = null;
	insertLocation!: number;
	resultText!: string;
	keyListener!: (event: KeyboardEvent) => void;

	constructor(
		public view: EditorView,
		public index: number,
		public text: string,
	) {
		super();
	}

	toDOM(_view: EditorView): HTMLElement {
		activeDocument.removeEventListener('keydown', this.keyListener, true);

		const div = createSpan({ cls: 'result-text', text: this.text });

		this.insertLocation = this.index;
		this.resultText = this.text;

		ResultWidget.activeWidget = this;

		div.onclick = () => {
			this.insertToDOM();
		};


		return div;
	}

	destroy(dom: HTMLElement): void {
		if (ResultWidget.activeWidget === this) {
			ResultWidget.activeWidget = null;
		}

		dom.remove();
	}

	insertToDOM() {
		const transaction = this.view.state.update({
			changes: {
				from: this.insertLocation,
				to: this.insertLocation,
				insert: this.resultText,
			},
			selection: {
				anchor: this.insertLocation + this.resultText.length,
				head: this.insertLocation + this.resultText.length,
			},
		});
		this.view.dispatch(transaction);

		activeDocument.removeEventListener('keydown', this.keyListener, true);
	}
}
