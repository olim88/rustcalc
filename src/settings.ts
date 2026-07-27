import { App, PluginSettingTab, Setting } from 'obsidian';
import RustCalcPlugin from './main';

export interface RustCalcSettings {
	calculationTriggerString: string;
	approxCalculationTriggerString: string;
	approxDecimalPrecision: number;
	completionTriggerKey: string;
	multiplicationSymbol: string;
	groupSeparator: string;
	decimalSeparator: string;
}

export const DEFAULT_SETTINGS: RustCalcSettings = {
	calculationTriggerString: '=',
	approxCalculationTriggerString: '\\approx',
	approxDecimalPrecision: 3,
	completionTriggerKey: 'Tab',
	multiplicationSymbol: '*',
	groupSeparator: "'",
	decimalSeparator: '.',
};

export class RustCalcSettingTab extends PluginSettingTab {
	plugin: RustCalcPlugin;

	constructor(app: App, plugin: RustCalcPlugin) {
		super(app, plugin);
		this.plugin = plugin;
	}

	display(): void {
		const { containerEl } = this;
		containerEl.empty();

		new Setting(containerEl)
			.setName('Calculation trigger string')
			.setDesc('The string that triggers calculation.')
			.addText((text) =>
				text
					.setPlaceholder('Type a string here')
					.setValue(this.plugin.settings.calculationTriggerString)
					.onChange(async (value) => {
						this.plugin.settings.calculationTriggerString = value;
						await this.plugin.saveSettings();
					}),
			);

		new Setting(containerEl)
			.setName('Approximation trigger string')
			.setDesc('The string that triggers approximation.')
			.addText((text) =>
				text
					.setPlaceholder('Type a string here')
					.setValue(this.plugin.settings.approxCalculationTriggerString)
					.onChange(async (value) => {
						this.plugin.settings.approxCalculationTriggerString = value;
						await this.plugin.saveSettings();
					}),
			);

		new Setting(containerEl)
			.setName('Approximation precision')
			.setDesc('The precision used when approximating (-1 for max).')
			.addText((text) =>
				text
					.setPlaceholder('Type a number here')
					.setValue(
						this.plugin.settings.approxDecimalPrecision.toString(),
					)
					.onChange(async (value) => {
						const parsed = parseInt(value, 10);
						this.plugin.settings.approxDecimalPrecision = Number.isNaN(
							parsed,
						)
							? -1
							: parsed;
						await this.plugin.saveSettings();
					}),
			);

		new Setting(containerEl)
			.setName('Completion trigger key')
			.setDesc('The key that inserts the calculated result.')
			.addText((text) =>
				text
					.setPlaceholder('Type name of a key here')
					.setValue(this.plugin.settings.completionTriggerKey)
					.onChange(async (value) => {
						this.plugin.settings.completionTriggerKey = value;
						await this.plugin.saveSettings();
					}),
			);

		new Setting(containerEl)
			.setName('Multiplication symbol')
			.setDesc('The symbol used for multiplication (e.g. * or \\times).')
			.addText((text) =>
				text
					.setPlaceholder('Type a symbol here')
					.setValue(this.plugin.settings.multiplicationSymbol)
					.onChange(async (value) => {
						this.plugin.settings.multiplicationSymbol = value;
						await this.plugin.saveSettings();
					}),
			);

		new Setting(containerEl)
			.setName('Group separator')
			.setDesc("The symbol used for grouping numbers (e.g. ' or ,).")
			.addText((text) =>
				text
					.setPlaceholder('Type a symbol here')
					.setValue(this.plugin.settings.groupSeparator)
					.onChange(async (value) => {
						this.plugin.settings.groupSeparator = value;
						await this.plugin.saveSettings();
					}),
			);

		new Setting(containerEl)
			.setName('Decimal separator')
			.setDesc(
				'The symbol used for defining where the decimal point is (e.g. , or .).',
			)
			.addText((text) =>
				text
					.setPlaceholder('Type a symbol here')
					.setValue(this.plugin.settings.decimalSeparator)
					.onChange(async (value) => {
						this.plugin.settings.decimalSeparator = value;
						await this.plugin.saveSettings();
					}),
			);
	}
}
