import { App, PluginSettingTab, Setting } from 'obsidian';
import RustCalcPlugin from './main';

export interface RustCalcSettings {
	calculationTriggerString: string;
	approxCalculationTriggerString: string;
	approxDecimalPrecision: number;
	completionTriggerKey: string;
	shiftForExact: boolean;
}

export const DEFAULT_SETTINGS: RustCalcSettings = {
	calculationTriggerString: '=',
	approxCalculationTriggerString: '\\approx',
	approxDecimalPrecision: 3,
	completionTriggerKey: 'Tab',
	shiftForExact: true

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
			.setName('Shift for exact value')
			.setDesc('By default only simplify answer without removing accuracy. And require shift to get a value')
			.addToggle(toggle => toggle
				.setValue(this.plugin.settings.shiftForExact)
				.onChange(async (value) => {
					this.plugin.settings.shiftForExact = value;
					await this.plugin.saveSettings();
				}));



	}
}
