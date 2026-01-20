# Feature 06: Settings, Theming & Credentials

## Overview

Implement the settings system, theme management (light/dark/system), and secure credential storage using OS keychain. This feature provides the configuration foundation for all user preferences.

## Goals

- Complete settings UI with all categories from design doc
- Theme switching with system preference detection
- OS keychain integration for passwords and SSH passphrases
- Settings persistence and sync with backend
- Keyboard shortcut customization framework

## Technical Specification

### 1. Settings Dialog Component

```svelte
<!-- components/dialogs/SettingsDialog.svelte -->
<script lang="ts">
	import Dialog from './Dialog.svelte';
	import { settingsStore, type Settings } from '$stores/settings';
	import { storageCommands } from '$services/ipc';

	interface Props {
		open: boolean;
		onClose: () => void;
	}

	let { open, onClose }: Props = $props();

	let activeCategory = $state('general');
	let settings = $state<Settings>($settingsStore);
	let isDirty = $state(false);
	let isSaving = $state(false);

	const categories = [
		{ id: 'general', label: 'General', icon: 'settings' },
		{ id: 'editor', label: 'Editor', icon: 'code' },
		{ id: 'results', label: 'Results', icon: 'table' },
		{ id: 'query', label: 'Query Execution', icon: 'play' },
		{ id: 'connections', label: 'Connections', icon: 'database' },
		{ id: 'shortcuts', label: 'Keyboard Shortcuts', icon: 'keyboard' }
	];

	function handleChange() {
		isDirty = true;
	}

	async function handleSave() {
		isSaving = true;
		try {
			await storageCommands.saveSettings(settings);
			settingsStore.set(settings);
			isDirty = false;
		} catch (error) {
			console.error('Failed to save settings:', error);
		} finally {
			isSaving = false;
		}
	}

	function handleCancel() {
		settings = $settingsStore;
		isDirty = false;
		onClose();
	}

	function handleReset() {
		// Reset current category to defaults
		switch (activeCategory) {
			case 'editor':
				settings.editor = getDefaultEditorSettings();
				break;
			case 'results':
				settings.results = getDefaultResultsSettings();
				break;
			// ... etc
		}
		isDirty = true;
	}
</script>

<Dialog {open} onClose={handleCancel} title="Settings" size="large">
	<div class="flex h-[500px]">
		<!-- Sidebar -->
		<nav class="w-48 border-r border-gray-200 dark:border-gray-700 p-2">
			{#each categories as category}
				<button
					class="w-full flex items-center gap-2 px-3 py-2 rounded text-sm text-left"
					class:bg-blue-100={activeCategory === category.id}
					class:dark:bg-blue-900={activeCategory === category.id}
					onclick={() => (activeCategory = category.id)}
				>
					<Icon name={category.icon} size={16} />
					{category.label}
				</button>
			{/each}
		</nav>

		<!-- Content -->
		<div class="flex-1 p-4 overflow-auto">
			{#if activeCategory === 'general'}
				<GeneralSettings bind:settings onchange={handleChange} />
			{:else if activeCategory === 'editor'}
				<EditorSettings bind:settings onchange={handleChange} />
			{:else if activeCategory === 'results'}
				<ResultsSettings bind:settings onchange={handleChange} />
			{:else if activeCategory === 'query'}
				<QueryExecutionSettings bind:settings onchange={handleChange} />
			{:else if activeCategory === 'connections'}
				<ConnectionsSettings bind:settings onchange={handleChange} />
			{:else if activeCategory === 'shortcuts'}
				<ShortcutsSettings bind:settings onchange={handleChange} />
			{/if}
		</div>
	</div>

	<svelte:fragment slot="footer">
		<div class="flex items-center justify-between w-full">
			<button class="px-3 py-1.5 text-sm text-gray-600 hover:text-gray-800" onclick={handleReset}>
				Reset to Defaults
			</button>
			<div class="flex gap-2">
				<button class="px-4 py-1.5 text-sm border rounded hover:bg-gray-50" onclick={handleCancel}>
					Cancel
				</button>
				<button
					class="px-4 py-1.5 text-sm bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50"
					onclick={handleSave}
					disabled={!isDirty || isSaving}
				>
					{isSaving ? 'Saving...' : 'Save'}
				</button>
			</div>
		</div>
	</svelte:fragment>
</Dialog>
```

### 2. Settings Categories Components

```svelte
<!-- components/settings/GeneralSettings.svelte -->
<script lang="ts">
	import type { Settings } from '$stores/settings';
	import FormField from '$components/forms/FormField.svelte';
	import Select from '$components/forms/Select.svelte';
	import Checkbox from '$components/forms/Checkbox.svelte';

	interface Props {
		settings: Settings;
		onchange: () => void;
	}

	let { settings = $bindable(), onchange }: Props = $props();

	const themeOptions = [
		{ value: 'light', label: 'Light' },
		{ value: 'dark', label: 'Dark' },
		{ value: 'system', label: 'System' }
	];

	const startupOptions = [
		{ value: 'restore', label: 'Restore previous session' },
		{ value: 'fresh', label: 'Start fresh' }
	];
</script>

<div class="space-y-6">
	<h3 class="text-lg font-medium">General</h3>

	<FormField label="Theme">
		<Select options={themeOptions} bind:value={settings.theme.mode} {onchange} />
	</FormField>

	<FormField label="Startup">
		<Select options={startupOptions} bind:value={settings.general.startup} {onchange} />
	</FormField>

	<FormField label="Auto-save">
		<div class="flex items-center gap-2">
			<Checkbox bind:checked={settings.general.autoSave} {onchange} />
			<span class="text-sm text-gray-600">Save query tabs every</span>
			<input
				type="number"
				class="w-16 px-2 py-1 text-sm border rounded"
				bind:value={settings.general.autoSaveIntervalSec}
				{onchange}
				min="5"
				max="300"
			/>
			<span class="text-sm text-gray-600">seconds</span>
		</div>
	</FormField>
</div>
```

```svelte
<!-- components/settings/EditorSettings.svelte -->
<script lang="ts">
	import type { Settings } from '$stores/settings';
	import FormField from '$components/forms/FormField.svelte';
	import Select from '$components/forms/Select.svelte';
	import Checkbox from '$components/forms/Checkbox.svelte';
	import Input from '$components/forms/Input.svelte';

	interface Props {
		settings: Settings;
		onchange: () => void;
	}

	let { settings = $bindable(), onchange }: Props = $props();

	const fontFamilies = [
		{ value: 'JetBrains Mono', label: 'JetBrains Mono' },
		{ value: 'Fira Code', label: 'Fira Code' },
		{ value: 'Monaco', label: 'Monaco' },
		{ value: 'Consolas', label: 'Consolas' },
		{ value: 'monospace', label: 'System Monospace' }
	];

	const tabSizes = [
		{ value: 2, label: '2 spaces' },
		{ value: 4, label: '4 spaces' },
		{ value: 8, label: '8 spaces' }
	];
</script>

<div class="space-y-6">
	<h3 class="text-lg font-medium">Editor</h3>

	<div class="grid grid-cols-2 gap-4">
		<FormField label="Font Family">
			<Select options={fontFamilies} bind:value={settings.editor.fontFamily} {onchange} />
		</FormField>

		<FormField label="Font Size">
			<Input type="number" bind:value={settings.editor.fontSize} {onchange} min="8" max="24" />
		</FormField>
	</div>

	<div class="grid grid-cols-2 gap-4">
		<FormField label="Tab Size">
			<Select options={tabSizes} bind:value={settings.editor.tabSize} {onchange} />
		</FormField>

		<FormField label="Indentation">
			<Select
				options={[
					{ value: true, label: 'Spaces' },
					{ value: false, label: 'Tabs' }
				]}
				bind:value={settings.editor.useSpaces}
				{onchange}
			/>
		</FormField>
	</div>

	<div class="space-y-3">
		<Checkbox bind:checked={settings.editor.lineNumbers} {onchange} label="Show line numbers" />

		<Checkbox bind:checked={settings.editor.minimap} {onchange} label="Show minimap" />

		<Checkbox bind:checked={settings.editor.wordWrap} {onchange} label="Word wrap" />

		<Checkbox bind:checked={settings.editor.bracketMatching} {onchange} label="Bracket matching" />
	</div>

	<FormField label="Autocomplete delay (ms)">
		<Input
			type="number"
			bind:value={settings.editor.autocompleteDelayMs}
			{onchange}
			min="0"
			max="1000"
		/>
	</FormField>
</div>
```

```svelte
<!-- components/settings/ResultsSettings.svelte -->
<script lang="ts">
	import type { Settings } from '$stores/settings';
	import FormField from '$components/forms/FormField.svelte';
	import Select from '$components/forms/Select.svelte';
	import Input from '$components/forms/Input.svelte';

	interface Props {
		settings: Settings;
		onchange: () => void;
	}

	let { settings = $bindable(), onchange }: Props = $props();

	const copyFormats = [
		{ value: 'tsv', label: 'Tab-separated (TSV)' },
		{ value: 'csv', label: 'Comma-separated (CSV)' },
		{ value: 'json', label: 'JSON' }
	];

	const dateFormats = [
		{ value: 'YYYY-MM-DD HH:mm:ss', label: 'ISO (2024-03-15 10:30:00)' },
		{ value: 'MM/DD/YYYY h:mm A', label: 'US (03/15/2024 10:30 AM)' },
		{ value: 'DD/MM/YYYY HH:mm', label: 'EU (15/03/2024 10:30)' },
		{ value: 'relative', label: 'Relative (2 hours ago)' }
	];
</script>

<div class="space-y-6">
	<h3 class="text-lg font-medium">Results</h3>

	<FormField label="Default row limit">
		<Input
			type="number"
			bind:value={settings.results.defaultRowLimit}
			{onchange}
			min="100"
			max="100000"
		/>
		<p class="text-xs text-gray-500 mt-1">Maximum rows to fetch for SELECT queries without LIMIT</p>
	</FormField>

	<FormField label="Date/Time format">
		<Select options={dateFormats} bind:value={settings.results.dateFormat} {onchange} />
	</FormField>

	<FormField label="NULL display">
		<Input type="text" bind:value={settings.results.nullDisplay} {onchange} placeholder="NULL" />
	</FormField>

	<FormField label="Truncate text at (characters)">
		<Input
			type="number"
			bind:value={settings.results.truncateTextAt}
			{onchange}
			min="50"
			max="10000"
		/>
	</FormField>

	<FormField label="Copy format">
		<Select options={copyFormats} bind:value={settings.results.copyFormat} {onchange} />
	</FormField>
</div>
```

### 3. Theme System

```typescript
// stores/theme.ts
import { writable, derived } from 'svelte/store';
import { browser } from '$app/environment';
import { storageCommands } from '$services/ipc';

export type ThemeMode = 'light' | 'dark' | 'system';

interface ThemeState {
	mode: ThemeMode;
	resolved: 'light' | 'dark';
}

function getSystemTheme(): 'light' | 'dark' {
	if (!browser) return 'light';
	return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

function resolveTheme(mode: ThemeMode): 'light' | 'dark' {
	if (mode === 'system') {
		return getSystemTheme();
	}
	return mode;
}

function createThemeStore() {
	const { subscribe, set, update } = writable<ThemeState>({
		mode: 'system',
		resolved: getSystemTheme()
	});

	// Listen for system theme changes
	if (browser) {
		const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
		mediaQuery.addEventListener('change', () => {
			update((state) => {
				if (state.mode === 'system') {
					return { ...state, resolved: getSystemTheme() };
				}
				return state;
			});
		});
	}

	return {
		subscribe,

		async setMode(mode: ThemeMode) {
			const resolved = resolveTheme(mode);
			set({ mode, resolved });

			// Apply to document
			if (browser) {
				document.documentElement.classList.toggle('dark', resolved === 'dark');
			}

			// Persist to settings
			try {
				const settings = await storageCommands.getSettings();
				settings.theme.mode = mode;
				await storageCommands.saveSettings(settings);
			} catch (error) {
				console.error('Failed to save theme setting:', error);
			}
		},

		async initialize() {
			try {
				const settings = await storageCommands.getSettings();
				const mode = (settings.theme?.mode as ThemeMode) || 'system';
				const resolved = resolveTheme(mode);
				set({ mode, resolved });

				if (browser) {
					document.documentElement.classList.toggle('dark', resolved === 'dark');
				}
			} catch (error) {
				console.error('Failed to load theme setting:', error);
			}
		}
	};
}

export const themeStore = createThemeStore();

// Derived store for easy access to resolved theme
export const currentTheme = derived(themeStore, ($theme) => $theme.resolved);
```

### 4. Credential Management (Keyring)

```rust
// services/keyring.rs
use keyring::Entry;
use crate::error::{Result, TuskError};

const SERVICE_NAME: &str = "tusk";

pub struct KeyringService;

impl KeyringService {
    /// Store a password for a connection
    pub fn store_password(connection_id: &str, password: &str) -> Result<()> {
        let entry = Entry::new(SERVICE_NAME, &format!("conn:{}", connection_id))
            .map_err(|e| TuskError::KeyringError(e.to_string()))?;

        entry.set_password(password)
            .map_err(|e| TuskError::KeyringError(e.to_string()))?;

        tracing::debug!("Stored password for connection: {}", connection_id);
        Ok(())
    }

    /// Retrieve a password for a connection
    pub fn get_password(connection_id: &str) -> Result<Option<String>> {
        let entry = Entry::new(SERVICE_NAME, &format!("conn:{}", connection_id))
            .map_err(|e| TuskError::KeyringError(e.to_string()))?;

        match entry.get_password() {
            Ok(password) => Ok(Some(password)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(TuskError::KeyringError(e.to_string())),
        }
    }

    /// Delete a password for a connection
    pub fn delete_password(connection_id: &str) -> Result<()> {
        let entry = Entry::new(SERVICE_NAME, &format!("conn:{}", connection_id))
            .map_err(|e| TuskError::KeyringError(e.to_string()))?;

        match entry.delete_credential() {
            Ok(()) => {
                tracing::debug!("Deleted password for connection: {}", connection_id);
                Ok(())
            }
            Err(keyring::Error::NoEntry) => Ok(()), // Already deleted
            Err(e) => Err(TuskError::KeyringError(e.to_string())),
        }
    }

    /// Store SSH passphrase for a connection
    pub fn store_ssh_passphrase(connection_id: &str, passphrase: &str) -> Result<()> {
        let entry = Entry::new(SERVICE_NAME, &format!("ssh:{}", connection_id))
            .map_err(|e| TuskError::KeyringError(e.to_string()))?;

        entry.set_password(passphrase)
            .map_err(|e| TuskError::KeyringError(e.to_string()))?;

        tracing::debug!("Stored SSH passphrase for connection: {}", connection_id);
        Ok(())
    }

    /// Retrieve SSH passphrase for a connection
    pub fn get_ssh_passphrase(connection_id: &str) -> Result<Option<String>> {
        let entry = Entry::new(SERVICE_NAME, &format!("ssh:{}", connection_id))
            .map_err(|e| TuskError::KeyringError(e.to_string()))?;

        match entry.get_password() {
            Ok(passphrase) => Ok(Some(passphrase)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(TuskError::KeyringError(e.to_string())),
        }
    }

    /// Delete SSH passphrase for a connection
    pub fn delete_ssh_passphrase(connection_id: &str) -> Result<()> {
        let entry = Entry::new(SERVICE_NAME, &format!("ssh:{}", connection_id))
            .map_err(|e| TuskError::KeyringError(e.to_string()))?;

        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(TuskError::KeyringError(e.to_string())),
        }
    }

    /// Delete all credentials for a connection
    pub fn delete_all_for_connection(connection_id: &str) -> Result<()> {
        Self::delete_password(connection_id)?;
        Self::delete_ssh_passphrase(connection_id)?;
        Ok(())
    }
}
```

### 5. Credential Commands

```rust
// commands/credentials.rs
use tauri::command;
use crate::error::Result;
use crate::services::keyring::KeyringService;

#[command]
pub fn store_password(connection_id: String, password: String) -> Result<()> {
    KeyringService::store_password(&connection_id, &password)
}

#[command]
pub fn get_password(connection_id: String) -> Result<Option<String>> {
    KeyringService::get_password(&connection_id)
}

#[command]
pub fn delete_password(connection_id: String) -> Result<()> {
    KeyringService::delete_password(&connection_id)
}

#[command]
pub fn store_ssh_passphrase(connection_id: String, passphrase: String) -> Result<()> {
    KeyringService::store_ssh_passphrase(&connection_id, &passphrase)
}

#[command]
pub fn get_ssh_passphrase(connection_id: String) -> Result<Option<String>> {
    KeyringService::get_ssh_passphrase(&connection_id)
}

#[command]
pub fn delete_ssh_passphrase(connection_id: String) -> Result<()> {
    KeyringService::delete_ssh_passphrase(&connection_id)
}
```

### 6. Settings Store

```typescript
// stores/settings.ts
import { writable, get } from 'svelte/store';
import { storageCommands } from '$services/ipc';

export interface Settings {
	general: GeneralSettings;
	theme: ThemeSettings;
	editor: EditorSettings;
	results: ResultsSettings;
	queryExecution: QueryExecutionSettings;
	connections: ConnectionsSettings;
	shortcuts: ShortcutsSettings;
}

export interface GeneralSettings {
	startup: 'restore' | 'fresh';
	autoSave: boolean;
	autoSaveIntervalSec: number;
}

export interface ThemeSettings {
	mode: 'light' | 'dark' | 'system';
}

export interface EditorSettings {
	fontFamily: string;
	fontSize: number;
	tabSize: number;
	useSpaces: boolean;
	lineNumbers: boolean;
	minimap: boolean;
	wordWrap: boolean;
	autocompleteDelayMs: number;
	bracketMatching: boolean;
}

export interface ResultsSettings {
	defaultRowLimit: number;
	dateFormat: string;
	nullDisplay: string;
	truncateTextAt: number;
	copyFormat: 'tsv' | 'csv' | 'json';
}

export interface QueryExecutionSettings {
	defaultStatementTimeoutMs: number | null;
	confirmDdl: boolean;
	confirmDestructive: boolean;
	autoUppercaseKeywords: boolean;
	autoLimitSelect: boolean;
}

export interface ConnectionsSettings {
	defaultSslMode: string;
	defaultConnectTimeoutSec: number;
	autoReconnectAttempts: number;
	keepaliveIntervalSec: number;
}

export interface ShortcutsSettings {
	shortcuts: Record<string, string>;
}

const defaultSettings: Settings = {
	general: {
		startup: 'restore',
		autoSave: true,
		autoSaveIntervalSec: 30
	},
	theme: {
		mode: 'system'
	},
	editor: {
		fontFamily: 'JetBrains Mono',
		fontSize: 13,
		tabSize: 2,
		useSpaces: true,
		lineNumbers: true,
		minimap: false,
		wordWrap: false,
		autocompleteDelayMs: 100,
		bracketMatching: true
	},
	results: {
		defaultRowLimit: 1000,
		dateFormat: 'YYYY-MM-DD HH:mm:ss',
		nullDisplay: 'NULL',
		truncateTextAt: 500,
		copyFormat: 'tsv'
	},
	queryExecution: {
		defaultStatementTimeoutMs: null,
		confirmDdl: true,
		confirmDestructive: true,
		autoUppercaseKeywords: false,
		autoLimitSelect: true
	},
	connections: {
		defaultSslMode: 'prefer',
		defaultConnectTimeoutSec: 10,
		autoReconnectAttempts: 3,
		keepaliveIntervalSec: 60
	},
	shortcuts: {
		shortcuts: {}
	}
};

function createSettingsStore() {
	const { subscribe, set, update } = writable<Settings>(defaultSettings);

	return {
		subscribe,
		set,
		update,

		async load() {
			try {
				const settings = await storageCommands.getSettings();
				set({ ...defaultSettings, ...settings });
			} catch (error) {
				console.error('Failed to load settings:', error);
			}
		},

		async save(settings: Settings) {
			try {
				await storageCommands.saveSettings(settings);
				set(settings);
			} catch (error) {
				console.error('Failed to save settings:', error);
				throw error;
			}
		},

		reset() {
			set(defaultSettings);
		}
	};
}

export const settingsStore = createSettingsStore();
```

### 7. Keyboard Shortcuts Settings

```svelte
<!-- components/settings/ShortcutsSettings.svelte -->
<script lang="ts">
	import type { Settings } from '$stores/settings';
	import { defaultShortcuts, type ShortcutAction } from '$utils/keyboard';
	import Input from '$components/forms/Input.svelte';

	interface Props {
		settings: Settings;
		onchange: () => void;
	}

	let { settings = $bindable(), onchange }: Props = $props();

	let searchQuery = $state('');
	let editingAction = $state<string | null>(null);
	let recordedKeys = $state('');

	const shortcutCategories = [
		{
			name: 'General',
			shortcuts: [
				{ action: 'settings', label: 'Open Settings' },
				{ action: 'commandPalette', label: 'Command Palette' },
				{ action: 'newTab', label: 'New Query Tab' },
				{ action: 'closeTab', label: 'Close Tab' },
				{ action: 'nextTab', label: 'Next Tab' },
				{ action: 'prevTab', label: 'Previous Tab' },
				{ action: 'toggleSidebar', label: 'Toggle Sidebar' }
			]
		},
		{
			name: 'Editor',
			shortcuts: [
				{ action: 'execute', label: 'Execute Query' },
				{ action: 'executeAll', label: 'Execute All' },
				{ action: 'cancelQuery', label: 'Cancel Query' },
				{ action: 'format', label: 'Format SQL' },
				{ action: 'save', label: 'Save' },
				{ action: 'comment', label: 'Toggle Comment' },
				{ action: 'find', label: 'Find' },
				{ action: 'replace', label: 'Find and Replace' },
				{ action: 'goToLine', label: 'Go to Line' }
			]
		},
		{
			name: 'Results',
			shortcuts: [
				{ action: 'copy', label: 'Copy Selection' },
				{ action: 'selectAll', label: 'Select All' },
				{ action: 'export', label: 'Export Results' },
				{ action: 'toggleEditMode', label: 'Toggle Edit Mode' }
			]
		},
		{
			name: 'Navigation',
			shortcuts: [
				{ action: 'focusEditor', label: 'Focus Editor' },
				{ action: 'focusResults', label: 'Focus Results' },
				{ action: 'focusSidebar', label: 'Focus Sidebar' },
				{ action: 'searchObjects', label: 'Search Objects' }
			]
		}
	];

	function getShortcut(action: string): string {
		return settings.shortcuts.shortcuts[action] || defaultShortcuts[action] || '';
	}

	function formatShortcut(shortcut: string): string {
		const isMac = navigator.platform.includes('Mac');
		return shortcut
			.replace('Mod', isMac ? '⌘' : 'Ctrl')
			.replace('Shift', isMac ? '⇧' : 'Shift')
			.replace('Alt', isMac ? '⌥' : 'Alt');
	}

	function startRecording(action: string) {
		editingAction = action;
		recordedKeys = '';
	}

	function handleKeyDown(e: KeyboardEvent) {
		if (!editingAction) return;

		e.preventDefault();
		e.stopPropagation();

		const parts: string[] = [];
		if (e.metaKey || e.ctrlKey) parts.push('Mod');
		if (e.shiftKey) parts.push('Shift');
		if (e.altKey) parts.push('Alt');

		if (!['Control', 'Shift', 'Alt', 'Meta'].includes(e.key)) {
			parts.push(e.key.length === 1 ? e.key.toUpperCase() : e.key);

			const shortcut = parts.join('+');
			settings.shortcuts.shortcuts[editingAction] = shortcut;
			editingAction = null;
			onchange();
		} else {
			recordedKeys = parts.join('+');
		}
	}

	function resetShortcut(action: string) {
		delete settings.shortcuts.shortcuts[action];
		onchange();
	}

	function filteredCategories() {
		if (!searchQuery) return shortcutCategories;

		return shortcutCategories
			.map((cat) => ({
				...cat,
				shortcuts: cat.shortcuts.filter((s) =>
					s.label.toLowerCase().includes(searchQuery.toLowerCase())
				)
			}))
			.filter((cat) => cat.shortcuts.length > 0);
	}
</script>

<svelte:window onkeydown={handleKeyDown} />

<div class="space-y-4">
	<h3 class="text-lg font-medium">Keyboard Shortcuts</h3>

	<Input type="text" placeholder="Search shortcuts..." bind:value={searchQuery} />

	<div class="space-y-6">
		{#each filteredCategories() as category}
			<div>
				<h4 class="text-sm font-medium text-gray-500 mb-2">{category.name}</h4>
				<div class="space-y-1">
					{#each category.shortcuts as shortcut}
						<div class="flex items-center justify-between py-1">
							<span class="text-sm">{shortcut.label}</span>
							<div class="flex items-center gap-2">
								{#if editingAction === shortcut.action}
									<span class="px-2 py-1 text-sm bg-blue-100 dark:bg-blue-900 rounded">
										{recordedKeys || 'Press keys...'}
									</span>
									<button
										class="text-xs text-gray-500 hover:text-gray-700"
										onclick={() => (editingAction = null)}
									>
										Cancel
									</button>
								{:else}
									<button
										class="px-2 py-1 text-sm font-mono bg-gray-100 dark:bg-gray-800 rounded hover:bg-gray-200"
										onclick={() => startRecording(shortcut.action)}
									>
										{formatShortcut(getShortcut(shortcut.action)) || 'Not set'}
									</button>
									{#if settings.shortcuts.shortcuts[shortcut.action]}
										<button
											class="text-xs text-gray-400 hover:text-gray-600"
											onclick={() => resetShortcut(shortcut.action)}
											title="Reset to default"
										>
											×
										</button>
									{/if}
								{/if}
							</div>
						</div>
					{/each}
				</div>
			</div>
		{/each}
	</div>
</div>
```

### 8. Keyboard Utility

```typescript
// utils/keyboard.ts
import { browser } from '$app/environment';

export const defaultShortcuts: Record<string, string> = {
	// General
	settings: 'Mod+,',
	commandPalette: 'Mod+Shift+P',
	newTab: 'Mod+N',
	closeTab: 'Mod+W',
	nextTab: 'Mod+Tab',
	prevTab: 'Mod+Shift+Tab',
	toggleSidebar: 'Mod+B',

	// Editor
	execute: 'Mod+Enter',
	executeAll: 'Mod+Shift+Enter',
	cancelQuery: 'Mod+.',
	format: 'Mod+Shift+F',
	save: 'Mod+S',
	comment: 'Mod+/',
	find: 'Mod+F',
	replace: 'Mod+H',
	goToLine: 'Mod+G',

	// Results
	copy: 'Mod+C',
	selectAll: 'Mod+A',
	export: 'Mod+E',
	toggleEditMode: 'Mod+Shift+E',

	// Navigation
	focusEditor: 'Mod+1',
	focusResults: 'Mod+2',
	focusSidebar: 'Mod+0',
	searchObjects: 'Mod+P'
};

export function parseShortcut(shortcut: string): {
	mod: boolean;
	shift: boolean;
	alt: boolean;
	key: string;
} {
	const parts = shortcut.split('+');
	return {
		mod: parts.includes('Mod'),
		shift: parts.includes('Shift'),
		alt: parts.includes('Alt'),
		key: parts[parts.length - 1]
	};
}

export function matchesShortcut(e: KeyboardEvent, shortcut: string): boolean {
	const parsed = parseShortcut(shortcut);
	const isMac = browser && navigator.platform.includes('Mac');

	const modKey = isMac ? e.metaKey : e.ctrlKey;

	return (
		modKey === parsed.mod &&
		e.shiftKey === parsed.shift &&
		e.altKey === parsed.alt &&
		e.key.toUpperCase() === parsed.key.toUpperCase()
	);
}

export type ShortcutAction = keyof typeof defaultShortcuts;
```

## Acceptance Criteria

1. [ ] Settings dialog opens with Cmd/Ctrl+,
2. [ ] All settings categories display correctly
3. [ ] Settings persist across app restarts
4. [ ] Theme switching works (light/dark/system)
5. [ ] System theme detection works
6. [ ] Passwords stored in OS keychain, not in SQLite
7. [ ] SSH passphrases stored in OS keychain
8. [ ] Keyboard shortcuts can be customized
9. [ ] Shortcut conflicts are detected and warned
10. [ ] Reset to defaults works per category

## Testing with MCP

```
1. Start app: npm run tauri dev
2. Connect: driver_session action=start
3. Open settings: webview_keyboard action=press key="," modifiers=["Meta"]
4. Verify dialog: webview_dom_snapshot type=accessibility
5. Change theme: webview_click selector="[data-value='dark']"
6. Verify theme applied: webview_get_styles selector="html" properties=["color-scheme"]
7. Test credential storage: ipc_execute_command command="store_password" args={...}
8. Verify in keychain: ipc_execute_command command="get_password" args={...}
```

## Dependencies on Other Features

- 03-frontend-architecture.md
- 04-ipc-layer.md
- 05-local-storage.md

## Dependent Features

- 07-connection-management.md
- 12-monaco-editor.md
- All features that use settings
