# Feature 09: Connection UI

## Overview

Implement the connection user interface including the connection dialog for creating/editing connections, the connection tree in the sidebar, visual status indicators, groups/folders, and drag-drop reordering.

## Goals

- Full connection dialog with all fields from design doc
- Connection tree with groups and status indicators
- Drag-drop for reordering connections and groups
- Context menu for connection actions
- Visual feedback for connection status

## Technical Specification

### 1. Connection Dialog

```svelte
<!-- components/dialogs/ConnectionDialog.svelte -->
<script lang="ts">
	import Dialog from './Dialog.svelte';
	import FormField from '$components/forms/FormField.svelte';
	import Input from '$components/forms/Input.svelte';
	import Select from '$components/forms/Select.svelte';
	import Checkbox from '$components/forms/Checkbox.svelte';
	import Button from '$components/forms/Button.svelte';
	import Tabs from '$components/common/Tabs.svelte';
	import ColorPicker from '$components/forms/ColorPicker.svelte';
	import { connectionsStore } from '$stores/connections';
	import { connectionService } from '$services/connection';
	import { browseCertificate, browseSshKey } from '$services/dialog';
	import { v4 as uuidv4 } from 'uuid';
	import type { ConnectionConfig, SslMode } from '$types/connection';

	interface Props {
		open: boolean;
		editingConnection?: ConnectionConfig | null;
		onClose: () => void;
		onSaved?: (config: ConnectionConfig) => void;
	}

	let { open, editingConnection = null, onClose, onSaved }: Props = $props();

	let activeTab = $state('general');
	let isTesting = $state(false);
	let isSaving = $state(false);
	let testResult = $state<{ success: boolean; message: string } | null>(null);

	// Form state
	let config = $state<ConnectionConfig>(getDefaultConfig());
	let password = $state('');
	let sshPassword = $state('');
	let sshPassphrase = $state('');

	function getDefaultConfig(): ConnectionConfig {
		return {
			id: uuidv4(),
			name: '',
			color: '#3b82f6',
			groupId: undefined,
			host: 'localhost',
			port: 5432,
			database: 'postgres',
			username: 'postgres',
			passwordInKeyring: true,
			sslMode: 'prefer' as SslMode,
			sslCaCert: undefined,
			sslClientCert: undefined,
			sslClientKey: undefined,
			sshTunnel: {
				enabled: false,
				host: '',
				port: 22,
				username: '',
				auth: 'key',
				keyPath: undefined,
				passphraseInKeyring: true
			},
			options: {
				connectTimeoutSec: 10,
				statementTimeoutMs: undefined,
				applicationName: 'Tusk',
				readonly: false
			}
		};
	}

	// Reset form when dialog opens
	$effect(() => {
		if (open) {
			if (editingConnection) {
				config = { ...editingConnection };
			} else {
				config = getDefaultConfig();
			}
			password = '';
			sshPassword = '';
			sshPassphrase = '';
			testResult = null;
			activeTab = 'general';
		}
	});

	const tabs = [
		{ id: 'general', label: 'General' },
		{ id: 'ssl', label: 'SSL' },
		{ id: 'ssh', label: 'SSH Tunnel' },
		{ id: 'options', label: 'Options' }
	];

	const sslModes = [
		{ value: 'disable', label: 'Disable' },
		{ value: 'prefer', label: 'Prefer' },
		{ value: 'require', label: 'Require' },
		{ value: 'verify-ca', label: 'Verify CA' },
		{ value: 'verify-full', label: 'Verify Full' }
	];

	async function handleTest() {
		isTesting = true;
		testResult = null;

		try {
			const result = await connectionService.testConnection(config);
			testResult = {
				success: true,
				message: `Connected to ${result.version}\nLatency: ${result.latency_ms}ms`
			};
		} catch (error: any) {
			testResult = {
				success: false,
				message: error.message || 'Connection failed'
			};
		} finally {
			isTesting = false;
		}
	}

	async function handleSave() {
		isSaving = true;

		try {
			// Save connection (password will be stored in keyring by backend)
			await connectionsStore.save(config, password || undefined);

			// Store SSH credentials if provided
			if (config.sshTunnel?.enabled) {
				if (sshPassword) {
					await credentialCommands.storeSshPassword(config.id, sshPassword);
				}
				if (sshPassphrase) {
					await credentialCommands.storeSshPassphrase(config.id, sshPassphrase);
				}
			}

			onSaved?.(config);
			onClose();
		} catch (error) {
			console.error('Failed to save connection:', error);
		} finally {
			isSaving = false;
		}
	}

	function handleCancel() {
		onClose();
	}

	async function handleBrowseCaCert() {
		const path = await browseCertificate();
		if (path) config.sslCaCert = path;
	}

	async function handleBrowseClientCert() {
		const path = await browseCertificate();
		if (path) config.sslClientCert = path;
	}

	async function handleBrowseClientKey() {
		const path = await browseCertificate();
		if (path) config.sslClientKey = path;
	}

	async function handleBrowseSshKey() {
		const path = await browseSshKey();
		if (path && config.sshTunnel) config.sshTunnel.keyPath = path;
	}

	const isValid = $derived(
		config.name.trim() !== '' &&
			config.host.trim() !== '' &&
			config.port > 0 &&
			config.database.trim() !== '' &&
			config.username.trim() !== ''
	);
</script>

<Dialog
	{open}
	onClose={handleCancel}
	title={editingConnection ? 'Edit Connection' : 'New Connection'}
	size="large"
>
	<div class="flex h-[480px]">
		<!-- Tabs -->
		<nav class="w-32 border-r border-gray-200 dark:border-gray-700 p-2">
			{#each tabs as tab}
				<button
					class="w-full text-left px-3 py-2 text-sm rounded"
					class:bg-blue-100={activeTab === tab.id}
					class:dark:bg-blue-900={activeTab === tab.id}
					onclick={() => (activeTab = tab.id)}
				>
					{tab.label}
				</button>
			{/each}
		</nav>

		<!-- Content -->
		<div class="flex-1 p-4 overflow-auto">
			{#if activeTab === 'general'}
				<div class="space-y-4">
					<div class="flex gap-4">
						<FormField label="Connection Name" class="flex-1">
							<Input type="text" bind:value={config.name} placeholder="My Database" required />
						</FormField>

						<FormField label="Color">
							<ColorPicker bind:value={config.color} />
						</FormField>
					</div>

					<div class="grid grid-cols-3 gap-4">
						<FormField label="Host" class="col-span-2">
							<Input type="text" bind:value={config.host} placeholder="localhost" required />
						</FormField>

						<FormField label="Port">
							<Input type="number" bind:value={config.port} min="1" max="65535" required />
						</FormField>
					</div>

					<FormField label="Database">
						<Input type="text" bind:value={config.database} placeholder="postgres" required />
					</FormField>

					<FormField label="Username">
						<Input type="text" bind:value={config.username} placeholder="postgres" required />
					</FormField>

					<FormField label="Password">
						<Input
							type="password"
							bind:value={password}
							placeholder={editingConnection ? '(unchanged)' : ''}
							autocomplete="off"
						/>
						<p class="text-xs text-gray-500 mt-1">Stored securely in your system keychain</p>
					</FormField>
				</div>
			{:else if activeTab === 'ssl'}
				<div class="space-y-4">
					<FormField label="SSL Mode">
						<Select options={sslModes} bind:value={config.sslMode} />
						<p class="text-xs text-gray-500 mt-1">
							{#if config.sslMode === 'disable'}
								No SSL encryption
							{:else if config.sslMode === 'prefer'}
								Use SSL if server supports it
							{:else if config.sslMode === 'require'}
								Require SSL, skip certificate verification
							{:else if config.sslMode === 'verify-ca'}
								Verify server certificate against CA
							{:else if config.sslMode === 'verify-full'}
								Verify certificate and hostname
							{/if}
						</p>
					</FormField>

					{#if config.sslMode === 'verify-ca' || config.sslMode === 'verify-full'}
						<FormField label="CA Certificate" required>
							<div class="flex gap-2">
								<Input
									type="text"
									bind:value={config.sslCaCert}
									placeholder="/path/to/ca.crt"
									class="flex-1"
								/>
								<Button variant="secondary" onclick={handleBrowseCaCert}>Browse</Button>
							</div>
						</FormField>
					{/if}

					<FormField label="Client Certificate">
						<div class="flex gap-2">
							<Input
								type="text"
								bind:value={config.sslClientCert}
								placeholder="/path/to/client.crt"
								class="flex-1"
							/>
							<Button variant="secondary" onclick={handleBrowseClientCert}>Browse</Button>
						</div>
					</FormField>

					<FormField label="Client Key">
						<div class="flex gap-2">
							<Input
								type="text"
								bind:value={config.sslClientKey}
								placeholder="/path/to/client.key"
								class="flex-1"
							/>
							<Button variant="secondary" onclick={handleBrowseClientKey}>Browse</Button>
						</div>
					</FormField>
				</div>
			{:else if activeTab === 'ssh'}
				<div class="space-y-4">
					<Checkbox bind:checked={config.sshTunnel.enabled} label="Enable SSH Tunnel" />

					{#if config.sshTunnel?.enabled}
						<div class="grid grid-cols-3 gap-4">
							<FormField label="SSH Host" class="col-span-2">
								<Input
									type="text"
									bind:value={config.sshTunnel.host}
									placeholder="ssh.example.com"
									required
								/>
							</FormField>

							<FormField label="SSH Port">
								<Input type="number" bind:value={config.sshTunnel.port} min="1" max="65535" />
							</FormField>
						</div>

						<FormField label="SSH Username">
							<Input type="text" bind:value={config.sshTunnel.username} required />
						</FormField>

						<FormField label="Authentication">
							<Select
								options={[
									{ value: 'password', label: 'Password' },
									{ value: 'key', label: 'SSH Key' }
								]}
								bind:value={config.sshTunnel.auth}
							/>
						</FormField>

						{#if config.sshTunnel.auth === 'password'}
							<FormField label="SSH Password">
								<Input type="password" bind:value={sshPassword} autocomplete="off" />
							</FormField>
						{:else}
							<FormField label="SSH Key File">
								<div class="flex gap-2">
									<Input
										type="text"
										bind:value={config.sshTunnel.keyPath}
										placeholder="~/.ssh/id_rsa"
										class="flex-1"
									/>
									<Button variant="secondary" onclick={handleBrowseSshKey}>Browse</Button>
								</div>
							</FormField>

							<FormField label="Key Passphrase">
								<Input
									type="password"
									bind:value={sshPassphrase}
									placeholder="Leave empty if not encrypted"
									autocomplete="off"
								/>
							</FormField>
						{/if}
					{/if}
				</div>
			{:else if activeTab === 'options'}
				<div class="space-y-4">
					<FormField label="Connection Timeout (seconds)">
						<Input type="number" bind:value={config.options.connectTimeoutSec} min="1" max="300" />
					</FormField>

					<FormField label="Statement Timeout (milliseconds)">
						<Input
							type="number"
							bind:value={config.options.statementTimeoutMs}
							placeholder="No timeout"
							min="0"
						/>
						<p class="text-xs text-gray-500 mt-1">
							Maximum time for queries to run. Leave empty for no timeout.
						</p>
					</FormField>

					<FormField label="Application Name">
						<Input type="text" bind:value={config.options.applicationName} placeholder="Tusk" />
					</FormField>

					<Checkbox bind:checked={config.options.readonly} label="Read-only mode" />
					<p class="text-xs text-gray-500 ml-6 -mt-2">
						Prevents INSERT, UPDATE, DELETE, and DDL statements
					</p>
				</div>
			{/if}
		</div>
	</div>

	<!-- Test Result -->
	{#if testResult}
		<div
			class="mx-4 mb-4 p-3 rounded text-sm"
			class:bg-green-100={testResult.success}
			class:text-green-800={testResult.success}
			class:bg-red-100={!testResult.success}
			class:text-red-800={!testResult.success}
		>
			<pre class="whitespace-pre-wrap font-mono text-xs">{testResult.message}</pre>
		</div>
	{/if}

	<svelte:fragment slot="footer">
		<div class="flex justify-between w-full">
			<Button variant="secondary" onclick={handleTest} disabled={!isValid || isTesting}>
				{isTesting ? 'Testing...' : 'Test Connection'}
			</Button>

			<div class="flex gap-2">
				<Button variant="secondary" onclick={handleCancel}>Cancel</Button>
				<Button onclick={handleSave} disabled={!isValid || isSaving}>
					{isSaving ? 'Saving...' : 'Save'}
				</Button>
			</div>
		</div>
	</svelte:fragment>
</Dialog>
```

### 2. Connection Tree Component

```svelte
<!-- components/tree/ConnectionTree.svelte -->
<script lang="ts">
	import { connectionsStore, type ConnectionState } from '$stores/connections';
	import TreeNode from './TreeNode.svelte';
	import ContextMenu from '$components/common/ContextMenu.svelte';
	import Icon from '$components/common/Icon.svelte';
	import { dndzone } from 'svelte-dnd-action';

	interface Props {
		connections: ConnectionState[];
		groups: ConnectionGroup[];
		activeConnectionId: string | null;
		filter?: string;
	}

	let { connections, groups, activeConnectionId, filter = '' }: Props = $props();

	let contextMenu = $state<{
		x: number;
		y: number;
		connection?: ConnectionState;
		group?: ConnectionGroup;
	} | null>(null);

	// Build tree structure
	const tree = $derived(() => {
		// Filter connections
		const filtered = filter
			? connections.filter(
					(c) =>
						c.config.name.toLowerCase().includes(filter.toLowerCase()) ||
						c.config.host.toLowerCase().includes(filter.toLowerCase())
				)
			: connections;

		// Group connections by groupId
		const grouped = new Map<string | null, ConnectionState[]>();
		for (const conn of filtered) {
			const groupId = conn.config.groupId || null;
			if (!grouped.has(groupId)) {
				grouped.set(groupId, []);
			}
			grouped.get(groupId)!.push(conn);
		}

		// Build tree
		const rootConnections = grouped.get(null) || [];
		const groupNodes = groups
			.filter((g) => !g.parentId)
			.map((g) => ({
				group: g,
				connections: grouped.get(g.id) || [],
				children: buildGroupChildren(g.id)
			}));

		return { rootConnections, groupNodes };

		function buildGroupChildren(parentId: string) {
			return groups
				.filter((g) => g.parentId === parentId)
				.map((g) => ({
					group: g,
					connections: grouped.get(g.id) || [],
					children: buildGroupChildren(g.id)
				}));
		}
	});

	function handleConnectionClick(conn: ConnectionState) {
		if (conn.status === 'connected') {
			connectionsStore.setActive(conn.id);
		} else {
			connectionsStore.connect(conn.id);
		}
	}

	function handleConnectionDoubleClick(conn: ConnectionState) {
		// Open new query tab
		tabsStore.createQueryTab(conn.id);
	}

	function handleContextMenu(e: MouseEvent, conn?: ConnectionState, group?: ConnectionGroup) {
		e.preventDefault();
		contextMenu = { x: e.clientX, y: e.clientY, connection: conn, group };
	}

	function closeContextMenu() {
		contextMenu = null;
	}

	function handleConnect() {
		if (contextMenu?.connection) {
			connectionsStore.connect(contextMenu.connection.id);
		}
		closeContextMenu();
	}

	function handleDisconnect() {
		if (contextMenu?.connection) {
			connectionsStore.disconnect(contextMenu.connection.id);
		}
		closeContextMenu();
	}

	function handleEdit() {
		if (contextMenu?.connection) {
			connectionsStore.openDialog(contextMenu.connection.config);
		}
		closeContextMenu();
	}

	function handleDuplicate() {
		if (contextMenu?.connection) {
			const copy = {
				...contextMenu.connection.config,
				id: crypto.randomUUID(),
				name: `${contextMenu.connection.config.name} (copy)`
			};
			connectionsStore.openDialog(copy);
		}
		closeContextMenu();
	}

	function handleDelete() {
		if (contextMenu?.connection) {
			// Show confirmation dialog
			if (confirm(`Delete connection "${contextMenu.connection.config.name}"?`)) {
				connectionsStore.delete(contextMenu.connection.id);
			}
		}
		closeContextMenu();
	}

	function handleNewQuery() {
		if (contextMenu?.connection) {
			tabsStore.createQueryTab(contextMenu.connection.id);
		}
		closeContextMenu();
	}
</script>

<div class="connection-tree">
	<!-- Root level connections -->
	{#each tree.rootConnections as conn (conn.id)}
		<ConnectionNode
			connection={conn}
			isActive={conn.id === activeConnectionId}
			onclick={() => handleConnectionClick(conn)}
			ondblclick={() => handleConnectionDoubleClick(conn)}
			oncontextmenu={(e) => handleContextMenu(e, conn)}
		/>
	{/each}

	<!-- Groups -->
	{#each tree.groupNodes as node (node.group.id)}
		<GroupNode
			group={node.group}
			connections={node.connections}
			children={node.children}
			{activeConnectionId}
			onConnectionClick={handleConnectionClick}
			onConnectionDblClick={handleConnectionDoubleClick}
			onContextMenu={handleContextMenu}
		/>
	{/each}

	<!-- Empty state -->
	{#if tree.rootConnections.length === 0 && tree.groupNodes.length === 0}
		<div class="p-4 text-center text-gray-500 text-sm">
			{#if filter}
				No connections match "{filter}"
			{:else}
				No connections yet
			{/if}
		</div>
	{/if}
</div>

<!-- Context Menu -->
{#if contextMenu}
	<ContextMenu x={contextMenu.x} y={contextMenu.y} onClose={closeContextMenu}>
		{#if contextMenu.connection}
			{#if contextMenu.connection.status === 'connected'}
				<button onclick={handleDisconnect}>
					<Icon name="plug-off" size={14} />
					Disconnect
				</button>
			{:else}
				<button onclick={handleConnect}>
					<Icon name="plug" size={14} />
					Connect
				</button>
			{/if}
			<hr />
			<button onclick={handleNewQuery}>
				<Icon name="code" size={14} />
				New Query
			</button>
			<hr />
			<button onclick={handleEdit}>
				<Icon name="edit" size={14} />
				Edit
			</button>
			<button onclick={handleDuplicate}>
				<Icon name="copy" size={14} />
				Duplicate
			</button>
			<hr />
			<button onclick={handleDelete} class="text-red-600">
				<Icon name="trash" size={14} />
				Delete
			</button>
		{/if}
	</ContextMenu>
{/if}
```

### 3. Connection Node Component

```svelte
<!-- components/tree/ConnectionNode.svelte -->
<script lang="ts">
	import Icon from '$components/common/Icon.svelte';
	import type { ConnectionState } from '$stores/connections';

	interface Props {
		connection: ConnectionState;
		isActive: boolean;
		onclick: () => void;
		ondblclick: () => void;
		oncontextmenu: (e: MouseEvent) => void;
	}

	let { connection, isActive, onclick, ondblclick, oncontextmenu }: Props = $props();

	const statusColors = {
		disconnected: 'bg-gray-400',
		connecting: 'bg-yellow-400 animate-pulse',
		connected: 'bg-green-500',
		reconnecting: 'bg-yellow-400 animate-pulse',
		error: 'bg-red-500'
	};
</script>

<div
	class="connection-node flex items-center gap-2 px-2 py-1.5 cursor-pointer rounded text-sm"
	class:bg-blue-100={isActive}
	class:dark:bg-blue-900={isActive}
	class:hover:bg-gray-100={!isActive}
	class:dark:hover:bg-gray-800={!isActive}
	role="treeitem"
	tabindex="0"
	aria-selected={isActive}
	{onclick}
	{ondblclick}
	{oncontextmenu}
	onkeydown={(e) => e.key === 'Enter' && onclick()}
>
	<!-- Color indicator -->
	{#if connection.config.color}
		<span class="w-2 h-2 rounded flex-shrink-0" style="background-color: {connection.config.color}"
		></span>
	{/if}

	<!-- Status indicator -->
	<span
		class="w-2 h-2 rounded-full flex-shrink-0 {statusColors[connection.status]}"
		title={connection.status}
	></span>

	<!-- Database icon -->
	<Icon name="database" size={14} class="text-gray-500 flex-shrink-0" />

	<!-- Connection name -->
	<span class="truncate flex-1" title={connection.config.name}>
		{connection.config.name}
	</span>

	<!-- Host info on hover -->
	<span class="text-xs text-gray-400 hidden group-hover:block">
		{connection.config.host}:{connection.config.port}
	</span>
</div>
```

### 4. Group Node Component

```svelte
<!-- components/tree/GroupNode.svelte -->
<script lang="ts">
	import Icon from '$components/common/Icon.svelte';
	import ConnectionNode from './ConnectionNode.svelte';
	import type { ConnectionState } from '$stores/connections';
	import type { ConnectionGroup } from '$types/connection';

	interface GroupNodeData {
		group: ConnectionGroup;
		connections: ConnectionState[];
		children: GroupNodeData[];
	}

	interface Props {
		group: ConnectionGroup;
		connections: ConnectionState[];
		children: GroupNodeData[];
		activeConnectionId: string | null;
		onConnectionClick: (conn: ConnectionState) => void;
		onConnectionDblClick: (conn: ConnectionState) => void;
		onContextMenu: (e: MouseEvent, conn?: ConnectionState, group?: ConnectionGroup) => void;
	}

	let {
		group,
		connections,
		children,
		activeConnectionId,
		onConnectionClick,
		onConnectionDblClick,
		onContextMenu
	}: Props = $props();

	let isExpanded = $state(true);

	function toggleExpanded() {
		isExpanded = !isExpanded;
	}
</script>

<div class="group-node">
	<!-- Group header -->
	<div
		class="flex items-center gap-1 px-2 py-1 cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-800 rounded"
		onclick={toggleExpanded}
		oncontextmenu={(e) => onContextMenu(e, undefined, group)}
	>
		<Icon name={isExpanded ? 'chevron-down' : 'chevron-right'} size={14} class="text-gray-400" />
		<Icon name="folder" size={14} class="text-gray-500" />
		<span class="text-sm font-medium truncate">{group.name}</span>
		<span class="text-xs text-gray-400 ml-1">
			({connections.length + children.reduce((sum, c) => sum + c.connections.length, 0)})
		</span>
	</div>

	<!-- Children -->
	{#if isExpanded}
		<div class="pl-4">
			<!-- Nested groups -->
			{#each children as child (child.group.id)}
				<svelte:self
					group={child.group}
					connections={child.connections}
					children={child.children}
					{activeConnectionId}
					{onConnectionClick}
					{onConnectionDblClick}
					{onContextMenu}
				/>
			{/each}

			<!-- Connections in this group -->
			{#each connections as conn (conn.id)}
				<ConnectionNode
					connection={conn}
					isActive={conn.id === activeConnectionId}
					onclick={() => onConnectionClick(conn)}
					ondblclick={() => onConnectionDblClick(conn)}
					oncontextmenu={(e) => onContextMenu(e, conn)}
				/>
			{/each}
		</div>
	{/if}
</div>
```

### 5. Color Picker Component

```svelte
<!-- components/forms/ColorPicker.svelte -->
<script lang="ts">
	interface Props {
		value: string;
	}

	let { value = $bindable() }: Props = $props();

	const presetColors = [
		'#ef4444', // red
		'#f97316', // orange
		'#eab308', // yellow
		'#22c55e', // green
		'#06b6d4', // cyan
		'#3b82f6', // blue
		'#8b5cf6', // purple
		'#ec4899', // pink
		'#6b7280' // gray
	];

	let showPicker = $state(false);

	function selectColor(color: string) {
		value = color;
		showPicker = false;
	}
</script>

<div class="color-picker relative">
	<button
		type="button"
		class="w-8 h-8 rounded border-2 border-gray-300 dark:border-gray-600"
		style="background-color: {value}"
		onclick={() => (showPicker = !showPicker)}
		title="Choose color"
	></button>

	{#if showPicker}
		<div
			class="absolute top-full left-0 mt-1 p-2 bg-white dark:bg-gray-800 rounded shadow-lg border border-gray-200 dark:border-gray-700 z-50"
		>
			<div class="grid grid-cols-3 gap-1">
				{#each presetColors as color}
					<button
						type="button"
						class="w-6 h-6 rounded"
						class:ring-2={color === value}
						class:ring-blue-500={color === value}
						style="background-color: {color}"
						onclick={() => selectColor(color)}
					></button>
				{/each}
			</div>

			<div class="mt-2 pt-2 border-t border-gray-200 dark:border-gray-700">
				<input
					type="color"
					{value}
					oninput={(e) => (value = e.currentTarget.value)}
					class="w-full h-6 cursor-pointer"
				/>
			</div>
		</div>
	{/if}
</div>

<svelte:window
	onclick={(e) => {
		if (showPicker && !e.target?.closest('.color-picker')) {
			showPicker = false;
		}
	}}
/>
```

### 6. Context Menu Component

```svelte
<!-- components/common/ContextMenu.svelte -->
<script lang="ts">
	import { onMount } from 'svelte';

	interface Props {
		x: number;
		y: number;
		onClose: () => void;
		children: any;
	}

	let { x, y, onClose, children }: Props = $props();

	let menuRef: HTMLDivElement;

	onMount(() => {
		// Adjust position if menu would go off screen
		const rect = menuRef.getBoundingClientRect();
		if (x + rect.width > window.innerWidth) {
			x = window.innerWidth - rect.width - 8;
		}
		if (y + rect.height > window.innerHeight) {
			y = window.innerHeight - rect.height - 8;
		}
	});

	function handleKeyDown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			onClose();
		}
	}
</script>

<svelte:window onclick={onClose} onkeydown={handleKeyDown} />

<div
	bind:this={menuRef}
	class="context-menu fixed z-50 min-w-[160px] py-1 bg-white dark:bg-gray-800 rounded shadow-lg border border-gray-200 dark:border-gray-700"
	style="left: {x}px; top: {y}px"
	onclick|stopPropagation
>
	{@render children()}
</div>

<style>
	.context-menu :global(button) {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		width: 100%;
		padding: 0.375rem 0.75rem;
		text-align: left;
		font-size: 0.875rem;
	}

	.context-menu :global(button:hover) {
		background-color: #f3f4f6;
	}

	:global(.dark) .context-menu :global(button:hover) {
		background-color: #374151;
	}

	.context-menu :global(hr) {
		margin: 0.25rem 0;
		border-color: #e5e7eb;
	}

	:global(.dark) .context-menu :global(hr) {
		border-color: #374151;
	}
</style>
```

## Acceptance Criteria

1. [ ] Connection dialog opens for new/edit connections
2. [ ] All connection fields populate correctly
3. [ ] Test connection shows server version and latency
4. [ ] Validation prevents saving invalid connections
5. [ ] Password stored in keyring on save
6. [ ] Connection tree displays all connections
7. [ ] Groups expand/collapse correctly
8. [ ] Status indicators show correct colors
9. [ ] Double-click opens new query tab
10. [ ] Context menu shows correct actions
11. [ ] Disconnect/connect works from context menu
12. [ ] Edit opens dialog with connection data
13. [ ] Delete removes connection with confirmation
14. [ ] Search filters connections correctly

## Testing with MCP

```
1. Start app: npm run tauri dev
2. Connect: driver_session action=start
3. Open connection dialog: webview_click selector="[title='New Connection']"
4. Fill form: webview_fill_form fields=[...]
5. Test connection: webview_click selector="[text='Test Connection']"
6. Save: webview_click selector="[text='Save']"
7. Verify in tree: webview_dom_snapshot type=accessibility
8. Right-click: webview_interact action=click selector=".connection-node" button=right
9. Verify context menu: webview_dom_snapshot type=accessibility
```

## Dependencies on Other Features

- 06-settings-theming-credentials.md
- 07-connection-management.md
- 08-ssl-ssh-security.md

## Dependent Features

- 16-schema-browser.md
