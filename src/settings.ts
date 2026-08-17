import { App, PluginSettingTab } from 'obsidian';

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
    display(): void {
        throw new Error("Method not implemented. Not needed anymore but i can't remove it");
    }
	plugin: RustCalcPlugin;

	constructor(app: App, plugin: RustCalcPlugin) {
		super(app, plugin);
		this.plugin = plugin;
	}

	getSettingDefinitions() {
		return [
			{
				name: 'Calculation trigger string',
				desc: 'The string that triggers calculation.',
				control: { type: 'text', key: 'calculationTriggerString' },
			},
			{
				name: 'Approximation trigger string',
				desc: 'The string that triggers approximation.',
				control: { type: 'text', key: 'approxCalculationTriggerString' },
			},
			{
				name: 'Approximation precision',
				desc: 'The precision used when approximating (-1 for max).',
				control: { type: 'number', key: 'approxDecimalPrecision', min: -1 },
			},
			{
				name: 'Shift for Simplification',
				desc: 'Toggle between using shift to show only simplified answer and full answer e.g. (leaving sin(0.234) as is and trying to stick to whole numbers)',
				control: { type: 'toggle', key: 'shiftForExact' },
			},
			]
	}
}
