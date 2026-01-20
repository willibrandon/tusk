# Feature 03: Frontend Architecture

## Overview

Establish the Svelte 5 frontend structure with component organization, state management using Svelte stores, routing, and the shell layout (sidebar, tabs, status bar).

## Goals

- Create scalable component architecture
- Implement Svelte 5 runes for reactivity
- Set up global state management with stores
- Build the application shell layout
- Establish styling patterns with Tailwind

## Technical Specification

### 1. Component Structure

```
src/lib/
├── components/
│   ├── shell/
│   │   ├── Shell.svelte           # Main app layout
│   │   ├── Sidebar.svelte         # Left sidebar container
│   │   ├── TabBar.svelte          # Tab management
│   │   ├── Tab.svelte             # Individual tab
│   │   ├── StatusBar.svelte       # Bottom status bar
│   │   └── Resizer.svelte         # Panel resize handle
│   ├── editor/
│   │   ├── Editor.svelte          # Monaco wrapper
│   │   ├── EditorToolbar.svelte   # Run, stop, format buttons
│   │   └── EditorStatusLine.svelte
│   ├── grid/
│   │   ├── DataGrid.svelte        # TanStack Table wrapper
│   │   ├── GridCell.svelte        # Cell renderer
│   │   ├── GridHeader.svelte      # Column header
│   │   └── GridToolbar.svelte     # Grid actions
│   ├── tree/
│   │   ├── Tree.svelte            # Generic tree component
│   │   ├── TreeNode.svelte        # Tree node
│   │   ├── ConnectionTree.svelte  # Connection browser
│   │   └── SchemaTree.svelte      # Schema browser
│   ├── dialogs/
│   │   ├── Dialog.svelte          # Base dialog
│   │   ├── ConfirmDialog.svelte   # Confirmation dialog
│   │   ├── ConnectionDialog.svelte
│   │   └── SettingsDialog.svelte
│   ├── forms/
│   │   ├── Input.svelte
│   │   ├── Select.svelte
│   │   ├── Checkbox.svelte
│   │   ├── Button.svelte
│   │   └── FormField.svelte
│   └── common/
│       ├── Icon.svelte
│       ├── Tooltip.svelte
│       ├── ContextMenu.svelte
│       ├── Dropdown.svelte
│       ├── Spinner.svelte
│       └── Badge.svelte
├── stores/
│   ├── connections.ts             # Connection state
│   ├── tabs.ts                    # Tab state
│   ├── schema.ts                  # Schema cache
│   ├── settings.ts                # App settings
│   ├── theme.ts                   # Theme state
│   └── ui.ts                      # UI state (sidebar width, etc.)
├── services/
│   ├── ipc.ts                     # Tauri IPC wrapper
│   ├── connection.ts              # Connection service
│   ├── query.ts                   # Query service
│   └── storage.ts                 # Local storage service
├── utils/
│   ├── format.ts                  # Value formatting
│   ├── sql.ts                     # SQL utilities
│   ├── keyboard.ts                # Keyboard handling
│   └── platform.ts                # Platform detection
└── types/
    ├── connection.ts
    ├── schema.ts
    ├── query.ts
    └── settings.ts
```

### 2. Shell Layout (Shell.svelte)

```svelte
<script lang="ts">
	import { onMount } from 'svelte';
	import Sidebar from './Sidebar.svelte';
	import TabBar from './TabBar.svelte';
	import StatusBar from './StatusBar.svelte';
	import Resizer from './Resizer.svelte';
	import { uiStore } from '$stores/ui';
	import { settingsStore } from '$stores/settings';
	import { themeStore } from '$stores/theme';

	let sidebarWidth = $state(260);
	let sidebarCollapsed = $state(false);

	// Persist sidebar width
	$effect(() => {
		uiStore.setSidebarWidth(sidebarWidth);
	});

	// Apply theme class to document
	$effect(() => {
		const theme = $themeStore.current;
		document.documentElement.classList.toggle('dark', theme === 'dark');
	});

	function handleSidebarResize(delta: number) {
		sidebarWidth = Math.max(200, Math.min(500, sidebarWidth + delta));
	}

	function toggleSidebar() {
		sidebarCollapsed = !sidebarCollapsed;
	}

	// Keyboard shortcut: Cmd/Ctrl+B to toggle sidebar
	function handleKeydown(e: KeyboardEvent) {
		if ((e.metaKey || e.ctrlKey) && e.key === 'b') {
			e.preventDefault();
			toggleSidebar();
		}
	}

	onMount(() => {
		window.addEventListener('keydown', handleKeydown);
		return () => window.removeEventListener('keydown', handleKeydown);
	});
</script>

<div
	class="shell h-screen w-screen flex flex-col bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100"
>
	<!-- Main Content Area -->
	<div class="flex flex-1 overflow-hidden">
		<!-- Sidebar -->
		{#if !sidebarCollapsed}
			<aside
				class="flex-shrink-0 border-r border-gray-200 dark:border-gray-700 overflow-hidden"
				style="width: {sidebarWidth}px"
			>
				<Sidebar />
			</aside>

			<Resizer direction="horizontal" onResize={handleSidebarResize} />
		{/if}

		<!-- Main Panel -->
		<main class="flex-1 flex flex-col overflow-hidden">
			<!-- Tab Bar -->
			<TabBar />

			<!-- Content Area (tabs render here) -->
			<div class="flex-1 overflow-hidden">
				<slot />
			</div>
		</main>
	</div>

	<!-- Status Bar -->
	<StatusBar />
</div>

<style>
	.shell {
		/* Prevent text selection on UI elements */
		user-select: none;
	}

	/* Allow text selection in specific areas */
	:global(.selectable) {
		user-select: text;
	}
</style>
```

### 3. Sidebar Component

```svelte
<!-- components/shell/Sidebar.svelte -->
<script lang="ts">
	import ConnectionTree from '$components/tree/ConnectionTree.svelte';
	import { connectionsStore, type ConnectionState } from '$stores/connections';

	let searchQuery = $state('');
	let connections = $derived($connectionsStore.connections);
	let activeConnectionId = $derived($connectionsStore.activeConnectionId);

	function handleNewConnection() {
		// Open connection dialog
		connectionsStore.openDialog();
	}
</script>

<div class="sidebar h-full flex flex-col">
	<!-- Header -->
	<div class="flex items-center justify-between p-3 border-b border-gray-200 dark:border-gray-700">
		<h1 class="font-semibold text-sm">Connections</h1>
		<button
			class="p-1 rounded hover:bg-gray-100 dark:hover:bg-gray-800"
			onclick={handleNewConnection}
			title="New Connection"
		>
			<Icon name="plus" size={16} />
		</button>
	</div>

	<!-- Search -->
	<div class="p-2 border-b border-gray-200 dark:border-gray-700">
		<input
			type="text"
			placeholder="Search connections..."
			class="w-full px-2 py-1 text-sm rounded border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800"
			bind:value={searchQuery}
		/>
	</div>

	<!-- Connection Tree -->
	<div class="flex-1 overflow-auto">
		<ConnectionTree {connections} {activeConnectionId} filter={searchQuery} />
	</div>
</div>
```

### 4. Tab Bar Component

```svelte
<!-- components/shell/TabBar.svelte -->
<script lang="ts">
	import Tab from './Tab.svelte';
	import { tabsStore, type TabState } from '$stores/tabs';
	import { dndzone } from 'svelte-dnd-action';

	let tabs = $derived($tabsStore.tabs);
	let activeTabId = $derived($tabsStore.activeTabId);

	function handleTabClick(tabId: string) {
		tabsStore.setActive(tabId);
	}

	function handleTabClose(tabId: string) {
		tabsStore.close(tabId);
	}

	function handleTabMiddleClick(e: MouseEvent, tabId: string) {
		if (e.button === 1) {
			e.preventDefault();
			handleTabClose(tabId);
		}
	}

	function handleNewTab() {
		tabsStore.createQueryTab();
	}

	function handleDndConsider(e: CustomEvent) {
		tabsStore.reorder(e.detail.items);
	}

	function handleDndFinalize(e: CustomEvent) {
		tabsStore.reorder(e.detail.items);
	}
</script>

<div
	class="tab-bar flex items-center h-9 bg-gray-50 dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700"
>
	<!-- Tabs Container (scrollable) -->
	<div
		class="flex-1 flex items-center overflow-x-auto scrollbar-none"
		use:dndzone={{ items: tabs, flipDurationMs: 200 }}
		onconsider={handleDndConsider}
		onfinalize={handleDndFinalize}
	>
		{#each tabs as tab (tab.id)}
			<Tab
				{tab}
				isActive={tab.id === activeTabId}
				onclick={() => handleTabClick(tab.id)}
				onclose={() => handleTabClose(tab.id)}
				onauxclick={(e) => handleTabMiddleClick(e, tab.id)}
			/>
		{/each}
	</div>

	<!-- New Tab Button -->
	<button
		class="flex-shrink-0 p-2 hover:bg-gray-200 dark:hover:bg-gray-700"
		onclick={handleNewTab}
		title="New Query Tab (Cmd+N)"
	>
		<Icon name="plus" size={14} />
	</button>
</div>

<style>
	.scrollbar-none::-webkit-scrollbar {
		display: none;
	}
</style>
```

### 5. Tab Component

```svelte
<!-- components/shell/Tab.svelte -->
<script lang="ts">
	import Icon from '$components/common/Icon.svelte';
	import type { Tab } from '$stores/tabs';

	interface Props {
		tab: Tab;
		isActive: boolean;
		onclick: () => void;
		onclose: () => void;
		onauxclick: (e: MouseEvent) => void;
	}

	let { tab, isActive, onclick, onclose, onauxclick }: Props = $props();

	const iconMap: Record<string, string> = {
		query: 'code',
		table: 'table',
		view: 'eye',
		function: 'function'
	};
</script>

<div
	class="tab group flex items-center gap-1 px-3 py-1.5 text-sm cursor-pointer border-r border-gray-200 dark:border-gray-700 min-w-0 max-w-[200px]"
	class:bg-white={isActive}
	class:dark:bg-gray-900={isActive}
	class:bg-gray-50={!isActive}
	class:dark:bg-gray-800={!isActive}
	role="tab"
	tabindex="0"
	aria-selected={isActive}
	{onclick}
	{onauxclick}
>
	<!-- Connection color indicator -->
	{#if tab.connectionColor}
		<span class="w-2 h-2 rounded-full flex-shrink-0" style="background-color: {tab.connectionColor}"
		></span>
	{/if}

	<!-- Tab icon -->
	<Icon name={iconMap[tab.type] || 'file'} size={14} class="flex-shrink-0 text-gray-500" />

	<!-- Tab title -->
	<span class="truncate">
		{tab.title}
	</span>

	<!-- Modified indicator -->
	{#if tab.isModified}
		<span class="w-1.5 h-1.5 rounded-full bg-blue-500 flex-shrink-0"></span>
	{/if}

	<!-- Close button -->
	<button
		class="ml-1 p-0.5 rounded opacity-0 group-hover:opacity-100 hover:bg-gray-200 dark:hover:bg-gray-700"
		onclick|stopPropagation={onclose}
		title="Close"
	>
		<Icon name="x" size={12} />
	</button>
</div>
```

### 6. Status Bar Component

```svelte
<!-- components/shell/StatusBar.svelte -->
<script lang="ts">
	import { connectionsStore } from '$stores/connections';
	import { tabsStore } from '$stores/tabs';

	let activeConnection = $derived($connectionsStore.activeConnection);
	let activeTab = $derived($tabsStore.activeTab);

	// Query result info from active tab
	let queryInfo = $derived(activeTab?.type === 'query' ? activeTab.queryResult : null);
</script>

<footer
	class="status-bar flex items-center h-6 px-2 text-xs bg-gray-100 dark:bg-gray-800 border-t border-gray-200 dark:border-gray-700"
>
	<!-- Connection Status -->
	<div class="flex items-center gap-2">
		{#if activeConnection}
			<span
				class="w-2 h-2 rounded-full"
				class:bg-green-500={activeConnection.status === 'connected'}
				class:bg-yellow-500={activeConnection.status === 'connecting'}
				class:bg-red-500={activeConnection.status === 'error'}
				class:bg-gray-400={activeConnection.status === 'disconnected'}
			></span>
			<span class="text-gray-600 dark:text-gray-400">
				{activeConnection.name} @ {activeConnection.config.host}:{activeConnection.config.port}
			</span>
		{:else}
			<span class="text-gray-500">No connection</span>
		{/if}
	</div>

	<!-- Spacer -->
	<div class="flex-1"></div>

	<!-- Query Info -->
	{#if queryInfo}
		<div class="flex items-center gap-4 text-gray-600 dark:text-gray-400">
			{#if queryInfo.totalRows !== undefined}
				<span>{queryInfo.totalRows.toLocaleString()} rows</span>
			{/if}
			{#if queryInfo.elapsedMs !== undefined}
				<span>{queryInfo.elapsedMs}ms</span>
			{/if}
		</div>
	{/if}

	<!-- Position info for editor -->
	{#if activeTab?.type === 'query' && activeTab.cursorPosition}
		<div class="ml-4 text-gray-600 dark:text-gray-400">
			Ln {activeTab.cursorPosition.line}, Col {activeTab.cursorPosition.column}
		</div>
	{/if}
</footer>
```

### 7. Resizer Component

```svelte
<!-- components/shell/Resizer.svelte -->
<script lang="ts">
	interface Props {
		direction: 'horizontal' | 'vertical';
		onResize: (delta: number) => void;
	}

	let { direction, onResize }: Props = $props();

	let isResizing = $state(false);
	let startPos = $state(0);

	function handleMouseDown(e: MouseEvent) {
		isResizing = true;
		startPos = direction === 'horizontal' ? e.clientX : e.clientY;

		document.addEventListener('mousemove', handleMouseMove);
		document.addEventListener('mouseup', handleMouseUp);
		document.body.style.cursor = direction === 'horizontal' ? 'col-resize' : 'row-resize';
		document.body.style.userSelect = 'none';
	}

	function handleMouseMove(e: MouseEvent) {
		if (!isResizing) return;

		const currentPos = direction === 'horizontal' ? e.clientX : e.clientY;
		const delta = currentPos - startPos;
		startPos = currentPos;

		onResize(delta);
	}

	function handleMouseUp() {
		isResizing = false;
		document.removeEventListener('mousemove', handleMouseMove);
		document.removeEventListener('mouseup', handleMouseUp);
		document.body.style.cursor = '';
		document.body.style.userSelect = '';
	}
</script>

<div
	class="resizer flex-shrink-0"
	class:w-1={direction === 'horizontal'}
	class:h-1={direction === 'vertical'}
	class:cursor-col-resize={direction === 'horizontal'}
	class:cursor-row-resize={direction === 'vertical'}
	class:bg-blue-500={isResizing}
	class:hover:bg-gray-300={!isResizing}
	class:dark:hover:bg-gray-600={!isResizing}
	role="separator"
	aria-orientation={direction}
	onmousedown={handleMouseDown}
></div>
```

### 8. Stores Implementation

```typescript
// stores/connections.ts
import { writable, derived } from 'svelte/store';
import type { ConnectionConfig, ConnectionStatus } from '$types/connection';

export interface ConnectionState {
	id: string;
	config: ConnectionConfig;
	status: ConnectionStatus;
	name: string;
}

interface ConnectionsState {
	connections: ConnectionState[];
	groups: ConnectionGroup[];
	activeConnectionId: string | null;
	dialogOpen: boolean;
	editingConnection: ConnectionConfig | null;
}

function createConnectionsStore() {
	const { subscribe, update, set } = writable<ConnectionsState>({
		connections: [],
		groups: [],
		activeConnectionId: null,
		dialogOpen: false,
		editingConnection: null
	});

	return {
		subscribe,

		setConnections(connections: ConnectionState[]) {
			update((s) => ({ ...s, connections }));
		},

		setActive(id: string | null) {
			update((s) => ({ ...s, activeConnectionId: id }));
		},

		updateStatus(id: string, status: ConnectionStatus) {
			update((s) => ({
				...s,
				connections: s.connections.map((c) => (c.id === id ? { ...c, status } : c))
			}));
		},

		openDialog(connection?: ConnectionConfig) {
			update((s) => ({
				...s,
				dialogOpen: true,
				editingConnection: connection || null
			}));
		},

		closeDialog() {
			update((s) => ({
				...s,
				dialogOpen: false,
				editingConnection: null
			}));
		},

		// Derived store for active connection
		get activeConnection() {
			return derived(
				this,
				($s) => $s.connections.find((c) => c.id === $s.activeConnectionId) || null
			);
		}
	};
}

export const connectionsStore = createConnectionsStore();
```

```typescript
// stores/tabs.ts
import { writable, derived } from 'svelte/store';
import { v4 as uuidv4 } from 'uuid';

export interface Tab {
	id: string;
	type: 'query' | 'table' | 'view' | 'function';
	title: string;
	connectionId: string | null;
	connectionColor?: string;
	isModified: boolean;
	content: string;
	cursorPosition?: { line: number; column: number };
	queryResult?: QueryResultInfo;
}

interface QueryResultInfo {
	totalRows?: number;
	elapsedMs?: number;
	status?: 'running' | 'success' | 'error';
}

interface TabsState {
	tabs: Tab[];
	activeTabId: string | null;
}

function createTabsStore() {
	const { subscribe, update } = writable<TabsState>({
		tabs: [],
		activeTabId: null
	});

	let tabCounter = 1;

	return {
		subscribe,

		createQueryTab(connectionId?: string) {
			const id = uuidv4();
			const tab: Tab = {
				id,
				type: 'query',
				title: `Query ${tabCounter++}`,
				connectionId: connectionId || null,
				isModified: false,
				content: ''
			};

			update((s) => ({
				tabs: [...s.tabs, tab],
				activeTabId: id
			}));

			return id;
		},

		createTableTab(connectionId: string, schema: string, table: string) {
			const id = uuidv4();
			const tab: Tab = {
				id,
				type: 'table',
				title: `${schema}.${table}`,
				connectionId,
				isModified: false,
				content: ''
			};

			update((s) => ({
				tabs: [...s.tabs, tab],
				activeTabId: id
			}));

			return id;
		},

		setActive(tabId: string) {
			update((s) => ({ ...s, activeTabId: tabId }));
		},

		close(tabId: string) {
			update((s) => {
				const index = s.tabs.findIndex((t) => t.id === tabId);
				const newTabs = s.tabs.filter((t) => t.id !== tabId);

				let newActiveId = s.activeTabId;
				if (s.activeTabId === tabId) {
					// Activate adjacent tab
					if (newTabs.length > 0) {
						newActiveId = newTabs[Math.min(index, newTabs.length - 1)].id;
					} else {
						newActiveId = null;
					}
				}

				return { tabs: newTabs, activeTabId: newActiveId };
			});
		},

		updateContent(tabId: string, content: string) {
			update((s) => ({
				...s,
				tabs: s.tabs.map((t) => (t.id === tabId ? { ...t, content, isModified: true } : t))
			}));
		},

		markSaved(tabId: string) {
			update((s) => ({
				...s,
				tabs: s.tabs.map((t) => (t.id === tabId ? { ...t, isModified: false } : t))
			}));
		},

		reorder(tabs: Tab[]) {
			update((s) => ({ ...s, tabs }));
		},

		updateQueryResult(tabId: string, result: QueryResultInfo) {
			update((s) => ({
				...s,
				tabs: s.tabs.map((t) => (t.id === tabId ? { ...t, queryResult: result } : t))
			}));
		},

		get activeTab() {
			return derived(this, ($s) => $s.tabs.find((t) => t.id === $s.activeTabId) || null);
		}
	};
}

export const tabsStore = createTabsStore();
```

```typescript
// stores/theme.ts
import { writable } from 'svelte/store';
import { browser } from '$app/environment';

type Theme = 'light' | 'dark' | 'system';

interface ThemeState {
	setting: Theme;
	current: 'light' | 'dark';
}

function getSystemTheme(): 'light' | 'dark' {
	if (!browser) return 'light';
	return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

function createThemeStore() {
	const stored = browser ? (localStorage.getItem('theme') as Theme) : null;
	const setting = stored || 'system';
	const current = setting === 'system' ? getSystemTheme() : setting;

	const { subscribe, update, set } = writable<ThemeState>({ setting, current });

	// Listen for system theme changes
	if (browser) {
		window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
			update((s) => {
				if (s.setting === 'system') {
					return { ...s, current: getSystemTheme() };
				}
				return s;
			});
		});
	}

	return {
		subscribe,

		setTheme(theme: Theme) {
			const current = theme === 'system' ? getSystemTheme() : theme;
			set({ setting: theme, current });

			if (browser) {
				localStorage.setItem('theme', theme);
			}
		}
	};
}

export const themeStore = createThemeStore();
```

```typescript
// stores/ui.ts
import { writable } from 'svelte/store';
import { browser } from '$app/environment';

interface UIState {
	sidebarWidth: number;
	sidebarCollapsed: boolean;
	resultsHeight: number;
}

function createUIStore() {
	const defaults: UIState = {
		sidebarWidth: 260,
		sidebarCollapsed: false,
		resultsHeight: 300
	};

	// Load from localStorage
	const stored = browser ? localStorage.getItem('ui-state') : null;
	const initial = stored ? { ...defaults, ...JSON.parse(stored) } : defaults;

	const { subscribe, update } = writable<UIState>(initial);

	// Persist changes
	subscribe((state) => {
		if (browser) {
			localStorage.setItem('ui-state', JSON.stringify(state));
		}
	});

	return {
		subscribe,

		setSidebarWidth(width: number) {
			update((s) => ({ ...s, sidebarWidth: width }));
		},

		toggleSidebar() {
			update((s) => ({ ...s, sidebarCollapsed: !s.sidebarCollapsed }));
		},

		setResultsHeight(height: number) {
			update((s) => ({ ...s, resultsHeight: height }));
		}
	};
}

export const uiStore = createUIStore();
```

### 9. Types

```typescript
// types/connection.ts
export interface ConnectionConfig {
	id: string;
	name: string;
	color?: string;
	groupId?: string;

	host: string;
	port: number;
	database: string;
	username: string;
	passwordInKeyring: boolean;

	sslMode: SslMode;
	sslCaCert?: string;
	sslClientCert?: string;
	sslClientKey?: string;

	sshTunnel?: SshTunnelConfig;
	options: ConnectionOptions;
}

export type SslMode = 'disable' | 'prefer' | 'require' | 'verify-ca' | 'verify-full';

export interface SshTunnelConfig {
	enabled: boolean;
	host: string;
	port: number;
	username: string;
	auth: 'password' | 'key';
	keyPath?: string;
	passphraseInKeyring: boolean;
}

export interface ConnectionOptions {
	connectTimeoutSec: number;
	statementTimeoutMs?: number;
	applicationName: string;
	readonly: boolean;
}

export type ConnectionStatus = 'disconnected' | 'connecting' | 'connected' | 'error';

export interface ConnectionGroup {
	id: string;
	name: string;
	parentId?: string;
	sortOrder: number;
	color?: string;
}
```

### 10. Main App Entry

```svelte
<!-- src/routes/+layout.svelte -->
<script lang="ts">
	import '../app.css';
	import Shell from '$components/shell/Shell.svelte';
</script>

<Shell>
	<slot />
</Shell>
```

```svelte
<!-- src/routes/+page.svelte -->
<script lang="ts">
	import { tabsStore } from '$stores/tabs';

	let activeTab = $derived($tabsStore.activeTab);
</script>

{#if activeTab}
	{#if activeTab.type === 'query'}
		<!-- Query editor will be implemented in feature 12 -->
		<div class="h-full p-4">
			<p class="text-gray-500">Query tab: {activeTab.title}</p>
		</div>
	{:else if activeTab.type === 'table'}
		<!-- Table viewer will be implemented in feature 17 -->
		<div class="h-full p-4">
			<p class="text-gray-500">Table tab: {activeTab.title}</p>
		</div>
	{/if}
{:else}
	<div class="h-full flex items-center justify-center text-gray-400">
		<div class="text-center">
			<p class="text-lg mb-2">No tabs open</p>
			<p class="text-sm">Create a new query or open a table from the sidebar</p>
		</div>
	</div>
{/if}
```

## Acceptance Criteria

1. [ ] Shell layout renders with sidebar, tab bar, and status bar
2. [ ] Sidebar is resizable with drag handle
3. [ ] Sidebar can be collapsed/expanded with Cmd/Ctrl+B
4. [ ] Tabs can be created, selected, closed, and reordered
5. [ ] Middle-click closes tabs
6. [ ] Theme switching works (light/dark/system)
7. [ ] Stores persist UI state to localStorage
8. [ ] Status bar shows connection status
9. [ ] All components are accessible (keyboard navigation, ARIA)

## Testing with MCP

```
1. Start app: npm run tauri dev
2. Connect: driver_session action=start
3. Get accessibility tree: webview_dom_snapshot type=accessibility
4. Verify sidebar: webview_find_element selector=".sidebar"
5. Click new tab: webview_interact action=click selector="[title='New Query Tab']"
6. Verify tab created: webview_dom_snapshot type=accessibility
7. Test keyboard: webview_keyboard action=press key="b" modifiers=["Meta"]
8. Verify sidebar collapsed
```

## Dependencies on Other Features

- 01-project-initialization.md

## Dependent Features

- 04-ipc-layer.md
- 06-settings-theming-credentials.md
- 09-connection-ui.md
- 12-monaco-editor.md
- 14-results-grid.md
