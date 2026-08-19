import { Plugin } from 'obsidian';
import { initCalculator } from './calculator';
import { rustCalcHintRenderer } from './editor';
import {
	DEFAULT_SETTINGS,
	RustCalcSettings,
	RustCalcSettingTab,
} from './settings';
import {ResultWidget} from "./widget";

export default class RustCalcPlugin extends Plugin {
	static INSTANCE: RustCalcPlugin;
	settings!: RustCalcSettings;

	async onload() {
		await this.loadSettings();
		await initCalculator();

		this.addSettingTab(new RustCalcSettingTab(this.app, this));
		this.registerEditorExtension([rustCalcHintRenderer]);

		this.addCommand({
			id: 'complete-calculation',
			name: 'Complete calculation',
			hotkeys: [
				{
					modifiers: [],
					key: 'Tab',
				},
			],
			callback: () => {
				ResultWidget.activeWidget?.insertToDOM();
			},
		});


		RustCalcPlugin.INSTANCE = this;
	}

	async loadSettings() {
		this.settings = Object.assign(
			{},
			DEFAULT_SETTINGS,
			(await this.loadData()) as Partial<RustCalcSettings>,
		);
	}

	async saveSettings() {
		await this.saveData(this.settings);
	}
}
