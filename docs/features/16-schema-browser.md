# Feature 16: Schema Browser

## Overview

The schema browser provides a hierarchical tree view of all database objects, enabling users to navigate schemas, tables, views, functions, and other objects. It includes context menus for common operations, DDL generation, and a global object search (command palette).

## Goals

- Display database objects in a navigable tree structure
- Support lazy loading for large schemas
- Provide context menus with object-specific actions
- Generate DDL for any object (CREATE, DROP, ALTER)
- Enable fuzzy search across all objects (command palette)
- Show object details on selection

## Dependencies

- Feature 10: Schema Introspection (schema metadata)
- Feature 07: Connection Management (connection state)
- Feature 13: Tabs Management (opening objects in tabs)

## Technical Specification

### 16.1 Schema Tree Component

```svelte
<!-- src/lib/components/sidebar/SchemaTree.svelte -->
<script lang="ts">
	import { onMount } from 'svelte';
	import {
		ChevronRight,
		ChevronDown,
		RefreshCw,
		Search,
		Table2,
		Eye,
		Columns,
		Key,
		ListTree,
		Zap,
		Hash,
		Database,
		Lock,
		GitBranch,
		Box,
		Users,
		Server,
		HardDrive
	} from 'lucide-svelte';
	import { schemaStore, type SchemaTreeNode } from '$lib/stores/schema.svelte';
	import { connectionsStore } from '$lib/stores/connections.svelte';
	import { tabStore } from '$lib/stores/tabs.svelte';
	import ContextMenu from '$lib/components/common/ContextMenu.svelte';
	import type { ContextMenuItem } from '$lib/components/common/ContextMenu.svelte';

	interface Props {
		connectionId: string;
	}

	let { connectionId }: Props = $props();

	let expandedNodes = $state<Set<string>>(new Set());
	let selectedNodeId = $state<string | null>(null);
	let contextMenu: { x: number; y: number; node: SchemaTreeNode } | null = $state(null);
	let searchQuery = $state('');

	const connection = $derived($connectionsStore.connections.find((c) => c.id === connectionId));

	const treeData = $derived($schemaStore.getTreeForConnection(connectionId));

	onMount(() => {
		if (!$schemaStore.hasSchema(connectionId)) {
			schemaStore.loadSchema(connectionId);
		}
	});

	function toggleExpand(nodeId: string) {
		const newExpanded = new Set(expandedNodes);
		if (newExpanded.has(nodeId)) {
			newExpanded.delete(nodeId);
		} else {
			newExpanded.add(nodeId);
		}
		expandedNodes = newExpanded;
	}

	function selectNode(node: SchemaTreeNode) {
		selectedNodeId = node.id;
	}

	function handleDoubleClick(node: SchemaTreeNode) {
		switch (node.type) {
			case 'table':
			case 'view':
			case 'materialized_view':
				tabStore.createTableTab(connectionId, node.schema!, node.name);
				break;
			case 'function':
				// Open function editor
				break;
		}
	}

	function handleContextMenu(e: MouseEvent, node: SchemaTreeNode) {
		e.preventDefault();
		contextMenu = { x: e.clientX, y: e.clientY, node };
	}

	function getContextMenuItems(node: SchemaTreeNode): ContextMenuItem[] {
		switch (node.type) {
			case 'table':
				return [
					{ label: 'View Data', action: () => openTableData(node) },
					{ label: 'New Query', action: () => newQueryForTable(node) },
					{ type: 'separator' },
					{ label: 'Edit Table', action: () => editTable(node) },
					{ label: 'Create Similar', action: () => createSimilar(node) },
					{ type: 'separator' },
					{ label: 'Copy Name', action: () => copyName(node) },
					{ label: 'Copy Qualified Name', action: () => copyQualifiedName(node) },
					{ type: 'separator' },
					{ label: 'View DDL', action: () => viewDdl(node) },
					{ type: 'separator' },
					{ label: 'Truncate...', action: () => truncateTable(node), danger: true },
					{ label: 'Drop...', action: () => dropObject(node), danger: true },
					{ type: 'separator' },
					{ label: 'Refresh', action: () => refreshNode(node) }
				];

			case 'view':
				return [
					{ label: 'View Data', action: () => openTableData(node) },
					{ label: 'New Query', action: () => newQueryForTable(node) },
					{ type: 'separator' },
					{ label: 'Edit View', action: () => editView(node) },
					{ label: 'View DDL', action: () => viewDdl(node) },
					{ type: 'separator' },
					{ label: 'Copy Name', action: () => copyName(node) },
					{ type: 'separator' },
					{ label: 'Drop...', action: () => dropObject(node), danger: true }
				];

			case 'materialized_view':
				return [
					{ label: 'View Data', action: () => openTableData(node) },
					{ label: 'Refresh View', action: () => refreshMatView(node) },
					{ type: 'separator' },
					{ label: 'View DDL', action: () => viewDdl(node) },
					{ label: 'Drop...', action: () => dropObject(node), danger: true }
				];

			case 'function':
				return [
					{ label: 'Open', action: () => openFunction(node) },
					{ label: 'Execute...', action: () => executeFunction(node) },
					{ type: 'separator' },
					{ label: 'View DDL', action: () => viewDdl(node) },
					{ label: 'Drop...', action: () => dropObject(node), danger: true }
				];

			case 'index':
				return [
					{ label: 'View DDL', action: () => viewDdl(node) },
					{ label: 'Reindex', action: () => reindex(node) },
					{ label: 'Drop...', action: () => dropObject(node), danger: true }
				];

			case 'column':
				return [
					{ label: 'Add to Query', action: () => addColumnToQuery(node) },
					{ label: 'Filter by Value...', action: () => filterByColumn(node) },
					{ label: 'Copy Name', action: () => copyName(node) }
				];

			case 'schema':
				return [
					{ label: 'New Table...', action: () => createTable(node) },
					{ label: 'New View...', action: () => createView(node) },
					{ label: 'New Function...', action: () => createFunction(node) },
					{ type: 'separator' },
					{ label: 'Drop Schema...', action: () => dropObject(node), danger: true }
				];

			default:
				return [{ label: 'Refresh', action: () => refreshNode(node) }];
		}
	}

	function getNodeIcon(node: SchemaTreeNode) {
		switch (node.type) {
			case 'connection':
				return Database;
			case 'schemas_folder':
				return ListTree;
			case 'schema':
				return Box;
			case 'tables_folder':
				return Table2;
			case 'table':
				return Table2;
			case 'views_folder':
				return Eye;
			case 'view':
				return Eye;
			case 'materialized_view':
				return Eye;
			case 'functions_folder':
				return Zap;
			case 'function':
				return Zap;
			case 'sequences_folder':
				return Hash;
			case 'sequence':
				return Hash;
			case 'columns_folder':
				return Columns;
			case 'column':
				return Columns;
			case 'indexes_folder':
				return Key;
			case 'index':
				return Key;
			case 'foreign_keys_folder':
				return GitBranch;
			case 'foreign_key':
				return GitBranch;
			case 'triggers_folder':
				return Zap;
			case 'trigger':
				return Zap;
			case 'policies_folder':
				return Lock;
			case 'policy':
				return Lock;
			case 'roles_folder':
				return Users;
			case 'role':
				return Users;
			case 'extensions_folder':
				return Box;
			case 'extension':
				return Box;
			case 'tablespaces_folder':
				return HardDrive;
			case 'tablespace':
				return HardDrive;
			default:
				return Box;
		}
	}

	// Action implementations
	function openTableData(node: SchemaTreeNode) {
		tabStore.createTableTab(connectionId, node.schema!, node.name);
		contextMenu = null;
	}

	function newQueryForTable(node: SchemaTreeNode) {
		const qualifiedName = node.schema ? `"${node.schema}"."${node.name}"` : `"${node.name}"`;
		tabStore.createQueryTab(connectionId, undefined, `SELECT * FROM ${qualifiedName} LIMIT 100;`);
		contextMenu = null;
	}

	function copyName(node: SchemaTreeNode) {
		navigator.clipboard.writeText(node.name);
		contextMenu = null;
	}

	function copyQualifiedName(node: SchemaTreeNode) {
		const name = node.schema ? `"${node.schema}"."${node.name}"` : `"${node.name}"`;
		navigator.clipboard.writeText(name);
		contextMenu = null;
	}

	async function viewDdl(node: SchemaTreeNode) {
		const ddl = await schemaStore.generateDdl(connectionId, node);
		tabStore.createQueryTab(connectionId, `${node.name} DDL`, ddl);
		contextMenu = null;
	}

	async function dropObject(node: SchemaTreeNode) {
		// Show confirmation dialog
		contextMenu = null;
	}

	async function truncateTable(node: SchemaTreeNode) {
		// Show confirmation dialog
		contextMenu = null;
	}

	async function refreshMatView(node: SchemaTreeNode) {
		// Execute REFRESH MATERIALIZED VIEW
		contextMenu = null;
	}

	function refreshNode(node: SchemaTreeNode) {
		schemaStore.loadSchema(connectionId);
		contextMenu = null;
	}

	function editTable(node: SchemaTreeNode) {
		contextMenu = null;
	}
	function createSimilar(node: SchemaTreeNode) {
		contextMenu = null;
	}
	function editView(node: SchemaTreeNode) {
		contextMenu = null;
	}
	function openFunction(node: SchemaTreeNode) {
		contextMenu = null;
	}
	function executeFunction(node: SchemaTreeNode) {
		contextMenu = null;
	}
	function reindex(node: SchemaTreeNode) {
		contextMenu = null;
	}
	function addColumnToQuery(node: SchemaTreeNode) {
		contextMenu = null;
	}
	function filterByColumn(node: SchemaTreeNode) {
		contextMenu = null;
	}
	function createTable(node: SchemaTreeNode) {
		contextMenu = null;
	}
	function createView(node: SchemaTreeNode) {
		contextMenu = null;
	}
	function createFunction(node: SchemaTreeNode) {
		contextMenu = null;
	}
</script>

<div class="schema-tree">
	<div class="tree-header">
		<div class="search-box">
			<Search size={14} />
			<input type="text" placeholder="Search objects..." bind:value={searchQuery} />
		</div>
		<button
			class="refresh-btn"
			onclick={() => schemaStore.loadSchema(connectionId)}
			title="Refresh schema"
		>
			<RefreshCw size={14} />
		</button>
	</div>

	<div class="tree-content">
		{#if $schemaStore.isLoading}
			<div class="loading">Loading schema...</div>
		{:else if treeData}
			{#each treeData.children || [] as node}
				<svelte:self
					{node}
					depth={0}
					{expandedNodes}
					{selectedNodeId}
					onToggle={toggleExpand}
					onSelect={selectNode}
					onDoubleClick={handleDoubleClick}
					onContextMenu={handleContextMenu}
					getIcon={getNodeIcon}
				/>
			{/each}
		{:else}
			<div class="empty">No schema loaded</div>
		{/if}
	</div>
</div>

{#if contextMenu}
	<ContextMenu
		x={contextMenu.x}
		y={contextMenu.y}
		items={getContextMenuItems(contextMenu.node)}
		onClose={() => (contextMenu = null)}
	/>
{/if}

<style>
	.schema-tree {
		display: flex;
		flex-direction: column;
		height: 100%;
		background: var(--surface-color);
	}

	.tree-header {
		display: flex;
		gap: 0.25rem;
		padding: 0.5rem;
		border-bottom: 1px solid var(--border-color);
	}

	.search-box {
		flex: 1;
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.375rem 0.5rem;
		border: 1px solid var(--border-color);
		border-radius: 0.375rem;
		background: var(--background-color);
	}

	.search-box input {
		flex: 1;
		border: none;
		background: none;
		font-size: 0.8125rem;
		outline: none;
	}

	.refresh-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 28px;
		height: 28px;
		border: 1px solid var(--border-color);
		border-radius: 0.375rem;
		background: none;
		color: var(--text-muted);
		cursor: pointer;
	}

	.refresh-btn:hover {
		background: var(--hover-color);
		color: var(--text-color);
	}

	.tree-content {
		flex: 1;
		overflow: auto;
		padding: 0.25rem 0;
	}

	.loading,
	.empty {
		padding: 1rem;
		text-align: center;
		color: var(--text-muted);
		font-size: 0.875rem;
	}
</style>
```

### 16.2 Tree Node Component

```svelte
<!-- src/lib/components/sidebar/TreeNode.svelte -->
<script lang="ts">
	import { ChevronRight, ChevronDown } from 'lucide-svelte';
	import type { SchemaTreeNode } from '$lib/stores/schema.svelte';
	import type { Component } from 'svelte';

	interface Props {
		node: SchemaTreeNode;
		depth: number;
		expandedNodes: Set<string>;
		selectedNodeId: string | null;
		onToggle: (nodeId: string) => void;
		onSelect: (node: SchemaTreeNode) => void;
		onDoubleClick: (node: SchemaTreeNode) => void;
		onContextMenu: (e: MouseEvent, node: SchemaTreeNode) => void;
		getIcon: (node: SchemaTreeNode) => Component;
	}

	let {
		node,
		depth,
		expandedNodes,
		selectedNodeId,
		onToggle,
		onSelect,
		onDoubleClick,
		onContextMenu,
		getIcon
	}: Props = $props();

	const isExpanded = $derived(expandedNodes.has(node.id));
	const isSelected = $derived(selectedNodeId === node.id);
	const hasChildren = $derived((node.children && node.children.length > 0) || node.hasLazyChildren);
	const Icon = $derived(getIcon(node));

	function handleClick(e: MouseEvent) {
		e.stopPropagation();
		onSelect(node);
		if (hasChildren) {
			onToggle(node.id);
		}
	}

	function handleDoubleClick(e: MouseEvent) {
		e.stopPropagation();
		onDoubleClick(node);
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' || e.key === ' ') {
			e.preventDefault();
			onSelect(node);
			if (hasChildren) {
				onToggle(node.id);
			}
		}
	}
</script>

<div class="tree-node">
	<div
		class="node-content"
		class:selected={isSelected}
		style:padding-left="{depth * 16 + 4}px"
		onclick={handleClick}
		ondblclick={handleDoubleClick}
		oncontextmenu={(e) => onContextMenu(e, node)}
		onkeydown={handleKeydown}
		role="treeitem"
		tabindex="0"
		aria-selected={isSelected}
		aria-expanded={hasChildren ? isExpanded : undefined}
	>
		<span class="expand-icon">
			{#if hasChildren}
				{#if isExpanded}
					<ChevronDown size={14} />
				{:else}
					<ChevronRight size={14} />
				{/if}
			{/if}
		</span>

		<span class="node-icon">
			<svelte:component this={Icon} size={14} />
		</span>

		<span class="node-label" title={node.tooltip || node.name}>
			{node.label || node.name}
			{#if node.badge}
				<span class="node-badge">({node.badge})</span>
			{/if}
		</span>

		{#if node.extra}
			<span class="node-extra">{node.extra}</span>
		{/if}
	</div>

	{#if isExpanded && hasChildren}
		<div class="node-children" role="group">
			{#each node.children || [] as child}
				<svelte:self
					node={child}
					depth={depth + 1}
					{expandedNodes}
					{selectedNodeId}
					{onToggle}
					{onSelect}
					{onDoubleClick}
					{onContextMenu}
					{getIcon}
				/>
			{/each}
		</div>
	{/if}
</div>

<style>
	.tree-node {
		user-select: none;
	}

	.node-content {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		padding: 0.25rem 0.5rem;
		cursor: pointer;
		border-radius: 0.25rem;
		margin: 0 0.25rem;
	}

	.node-content:hover {
		background: var(--hover-color);
	}

	.node-content.selected {
		background: var(--selected-color);
	}

	.node-content:focus {
		outline: 2px solid var(--primary-color);
		outline-offset: -2px;
	}

	.expand-icon {
		width: 14px;
		height: 14px;
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--text-muted);
	}

	.node-icon {
		display: flex;
		color: var(--text-muted);
	}

	.node-label {
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: 0.8125rem;
	}

	.node-badge {
		color: var(--text-muted);
		font-size: 0.75rem;
		margin-left: 0.25rem;
	}

	.node-extra {
		color: var(--text-muted);
		font-size: 0.6875rem;
		font-family: var(--font-mono);
	}
</style>
```

### 16.3 Schema Store with Tree Building

```typescript
// src/lib/stores/schema.svelte.ts

import { invoke } from '@tauri-apps/api/core';
import type { Schema, Table, Column, Index, ForeignKey } from '$lib/services/schema';

export interface SchemaTreeNode {
	id: string;
	name: string;
	label?: string;
	type: NodeType;
	schema?: string;
	tooltip?: string;
	badge?: string;
	extra?: string;
	children?: SchemaTreeNode[];
	hasLazyChildren?: boolean;
	data?: any;
}

export type NodeType =
	| 'connection'
	| 'schemas_folder'
	| 'schema'
	| 'tables_folder'
	| 'table'
	| 'views_folder'
	| 'view'
	| 'materialized_view'
	| 'functions_folder'
	| 'function'
	| 'sequences_folder'
	| 'sequence'
	| 'types_folder'
	| 'type'
	| 'columns_folder'
	| 'column'
	| 'indexes_folder'
	| 'index'
	| 'foreign_keys_folder'
	| 'foreign_key'
	| 'triggers_folder'
	| 'trigger'
	| 'policies_folder'
	| 'policy'
	| 'extensions_folder'
	| 'extension'
	| 'roles_folder'
	| 'role'
	| 'tablespaces_folder'
	| 'tablespace';

class SchemaStore {
	private schemas = $state<Map<string, Schema[]>>(new Map());
	private trees = $state<Map<string, SchemaTreeNode>>(new Map());
	isLoading = $state(false);
	error = $state<string | null>(null);

	hasSchema(connectionId: string): boolean {
		return this.schemas.has(connectionId);
	}

	getTreeForConnection(connectionId: string): SchemaTreeNode | null {
		return this.trees.get(connectionId) ?? null;
	}

	async loadSchema(connectionId: string): Promise<void> {
		this.isLoading = true;
		this.error = null;

		try {
			const schemas = await invoke<Schema[]>('get_schema', {
				connId: connectionId
			});

			this.schemas.set(connectionId, schemas);
			this.trees.set(connectionId, this.buildTree(connectionId, schemas));
		} catch (err) {
			this.error = String(err);
		} finally {
			this.isLoading = false;
		}
	}

	private buildTree(connectionId: string, schemas: Schema[]): SchemaTreeNode {
		const schemaNodes: SchemaTreeNode[] = schemas.map((schema) => ({
			id: `${connectionId}:schema:${schema.name}`,
			name: schema.name,
			type: 'schema',
			schema: schema.name,
			children: [
				this.buildTablesFolder(connectionId, schema),
				this.buildViewsFolder(connectionId, schema),
				this.buildFunctionsFolder(connectionId, schema),
				this.buildSequencesFolder(connectionId, schema),
				this.buildTypesFolder(connectionId, schema)
			].filter((node) => node.children && node.children.length > 0)
		}));

		return {
			id: `${connectionId}:root`,
			name: 'Schemas',
			type: 'schemas_folder',
			children: [
				{
					id: `${connectionId}:schemas`,
					name: 'Schemas',
					type: 'schemas_folder',
					badge: String(schemas.length),
					children: schemaNodes
				}
				// Extensions, Roles, Tablespaces would go here
			]
		};
	}

	private buildTablesFolder(connectionId: string, schema: Schema): SchemaTreeNode {
		return {
			id: `${connectionId}:${schema.name}:tables`,
			name: 'Tables',
			type: 'tables_folder',
			schema: schema.name,
			badge: String(schema.tables.length),
			children: schema.tables.map((table) => this.buildTableNode(connectionId, schema.name, table))
		};
	}

	private buildTableNode(connectionId: string, schemaName: string, table: Table): SchemaTreeNode {
		const sizeStr = this.formatSize(table.size_bytes);

		return {
			id: `${connectionId}:${schemaName}:table:${table.name}`,
			name: table.name,
			type: 'table',
			schema: schemaName,
			tooltip: `${table.row_count_estimate?.toLocaleString() ?? '?'} rows, ${sizeStr}`,
			extra: sizeStr,
			data: table,
			children: [
				{
					id: `${connectionId}:${schemaName}:${table.name}:columns`,
					name: 'Columns',
					type: 'columns_folder',
					badge: String(table.columns.length),
					children: table.columns.map((col) => ({
						id: `${connectionId}:${schemaName}:${table.name}:col:${col.name}`,
						name: col.name,
						label: `${col.name}${col.nullable ? '' : ' *'}`,
						type: 'column' as NodeType,
						schema: schemaName,
						extra: col.type,
						tooltip: this.buildColumnTooltip(col),
						data: col
					}))
				},
				{
					id: `${connectionId}:${schemaName}:${table.name}:indexes`,
					name: 'Indexes',
					type: 'indexes_folder',
					badge: String(table.indexes.length),
					children: table.indexes.map((idx) => ({
						id: `${connectionId}:${schemaName}:${table.name}:idx:${idx.name}`,
						name: idx.name,
						type: 'index' as NodeType,
						schema: schemaName,
						extra: idx.method,
						tooltip: `${idx.is_unique ? 'UNIQUE ' : ''}${idx.method.toUpperCase()} on (${idx.columns.join(', ')})`,
						data: idx
					}))
				},
				{
					id: `${connectionId}:${schemaName}:${table.name}:fks`,
					name: 'Foreign Keys',
					type: 'foreign_keys_folder',
					badge: String(table.foreign_keys.length),
					children: table.foreign_keys.map((fk) => ({
						id: `${connectionId}:${schemaName}:${table.name}:fk:${fk.name}`,
						name: fk.name,
						type: 'foreign_key' as NodeType,
						schema: schemaName,
						tooltip: `(${fk.columns.join(', ')}) → ${fk.referenced_table}(${fk.referenced_columns.join(', ')})`,
						data: fk
					}))
				},
				{
					id: `${connectionId}:${schemaName}:${table.name}:triggers`,
					name: 'Triggers',
					type: 'triggers_folder',
					badge: String(table.triggers?.length ?? 0),
					children: (table.triggers ?? []).map((tr) => ({
						id: `${connectionId}:${schemaName}:${table.name}:trigger:${tr.name}`,
						name: tr.name,
						type: 'trigger' as NodeType,
						schema: schemaName,
						data: tr
					}))
				},
				{
					id: `${connectionId}:${schemaName}:${table.name}:policies`,
					name: 'Policies',
					type: 'policies_folder',
					badge: String(table.policies?.length ?? 0),
					children: (table.policies ?? []).map((pol) => ({
						id: `${connectionId}:${schemaName}:${table.name}:policy:${pol.name}`,
						name: pol.name,
						type: 'policy' as NodeType,
						schema: schemaName,
						data: pol
					}))
				}
			].filter((node) => !node.badge || node.badge !== '0')
		};
	}

	private buildViewsFolder(connectionId: string, schema: Schema): SchemaTreeNode {
		const views = [
			...schema.views.map((v) => ({ ...v, viewType: 'view' as const })),
			...schema.materialized_views.map((v) => ({ ...v, viewType: 'materialized_view' as const }))
		];

		return {
			id: `${connectionId}:${schema.name}:views`,
			name: 'Views',
			type: 'views_folder',
			schema: schema.name,
			badge: String(views.length),
			children: views.map((view) => ({
				id: `${connectionId}:${schema.name}:view:${view.name}`,
				name: view.name,
				type: view.viewType,
				schema: schema.name,
				data: view
			}))
		};
	}

	private buildFunctionsFolder(connectionId: string, schema: Schema): SchemaTreeNode {
		return {
			id: `${connectionId}:${schema.name}:functions`,
			name: 'Functions',
			type: 'functions_folder',
			schema: schema.name,
			badge: String(schema.functions.length),
			children: schema.functions.map((fn) => ({
				id: `${connectionId}:${schema.name}:fn:${fn.name}:${fn.oid}`,
				name: fn.name,
				type: 'function' as NodeType,
				schema: schema.name,
				extra: fn.return_type,
				tooltip: `${fn.name}(${fn.arguments}) → ${fn.return_type}`,
				data: fn
			}))
		};
	}

	private buildSequencesFolder(connectionId: string, schema: Schema): SchemaTreeNode {
		return {
			id: `${connectionId}:${schema.name}:sequences`,
			name: 'Sequences',
			type: 'sequences_folder',
			schema: schema.name,
			badge: String(schema.sequences.length),
			children: schema.sequences.map((seq) => ({
				id: `${connectionId}:${schema.name}:seq:${seq.name}`,
				name: seq.name,
				type: 'sequence' as NodeType,
				schema: schema.name,
				data: seq
			}))
		};
	}

	private buildTypesFolder(connectionId: string, schema: Schema): SchemaTreeNode {
		return {
			id: `${connectionId}:${schema.name}:types`,
			name: 'Types',
			type: 'types_folder',
			schema: schema.name,
			badge: String(schema.types.length),
			children: schema.types.map((t) => ({
				id: `${connectionId}:${schema.name}:type:${t.name}`,
				name: t.name,
				type: 'type' as NodeType,
				schema: schema.name,
				data: t
			}))
		};
	}

	private buildColumnTooltip(col: Column): string {
		const parts = [col.type];
		if (!col.nullable) parts.push('NOT NULL');
		if (col.default) parts.push(`DEFAULT ${col.default}`);
		if (col.is_identity) parts.push(`IDENTITY ${col.identity_generation}`);
		if (col.comment) parts.push(`-- ${col.comment}`);
		return parts.join(' ');
	}

	private formatSize(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
		if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
		return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
	}

	async generateDdl(connectionId: string, node: SchemaTreeNode): Promise<string> {
		return invoke<string>('generate_ddl', {
			connId: connectionId,
			objectType: node.type,
			schema: node.schema,
			name: node.name
		});
	}
}

export const schemaStore = new SchemaStore();
```

### 16.4 Object Search (Command Palette)

```svelte
<!-- src/lib/components/dialogs/ObjectSearch.svelte -->
<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { Search, Table2, Eye, Zap, Columns, Box } from 'lucide-svelte';
	import { schemaStore } from '$lib/stores/schema.svelte';
	import { tabStore } from '$lib/stores/tabs.svelte';

	interface Props {
		connectionId: string;
		onClose: () => void;
	}

	let { connectionId, onClose }: Props = $props();

	let query = $state('');
	let selectedIndex = $state(0);
	let inputRef: HTMLInputElement;

	interface SearchResult {
		type: 'table' | 'view' | 'function' | 'column' | 'schema';
		schema: string;
		name: string;
		parentName?: string;
		icon: any;
	}

	const results = $derived(searchObjects(query));

	function searchObjects(q: string): SearchResult[] {
		if (!q.trim()) return [];

		const term = q.toLowerCase();
		const results: SearchResult[] = [];
		const schemas = schemaStore.schemas.get(connectionId) ?? [];

		for (const schema of schemas) {
			// Search tables
			for (const table of schema.tables) {
				if (fuzzyMatch(table.name, term)) {
					results.push({
						type: 'table',
						schema: schema.name,
						name: table.name,
						icon: Table2
					});
				}

				// Search columns
				for (const col of table.columns) {
					if (fuzzyMatch(col.name, term)) {
						results.push({
							type: 'column',
							schema: schema.name,
							name: col.name,
							parentName: table.name,
							icon: Columns
						});
					}
				}
			}

			// Search views
			for (const view of schema.views) {
				if (fuzzyMatch(view.name, term)) {
					results.push({
						type: 'view',
						schema: schema.name,
						name: view.name,
						icon: Eye
					});
				}
			}

			// Search functions
			for (const fn of schema.functions) {
				if (fuzzyMatch(fn.name, term)) {
					results.push({
						type: 'function',
						schema: schema.name,
						name: fn.name,
						icon: Zap
					});
				}
			}
		}

		// Sort by relevance
		results.sort((a, b) => {
			const aScore = getMatchScore(a.name, term);
			const bScore = getMatchScore(b.name, term);
			return bScore - aScore;
		});

		return results.slice(0, 50);
	}

	function fuzzyMatch(text: string, pattern: string): boolean {
		const textLower = text.toLowerCase();
		let patternIdx = 0;

		for (const char of textLower) {
			if (char === pattern[patternIdx]) {
				patternIdx++;
				if (patternIdx === pattern.length) return true;
			}
		}

		return patternIdx === pattern.length;
	}

	function getMatchScore(text: string, pattern: string): number {
		const textLower = text.toLowerCase();

		// Exact match gets highest score
		if (textLower === pattern) return 100;

		// Prefix match
		if (textLower.startsWith(pattern)) return 90;

		// Contains match
		if (textLower.includes(pattern)) return 80;

		// Fuzzy match - count consecutive matches
		let score = 0;
		let consecutive = 0;
		let patternIdx = 0;

		for (const char of textLower) {
			if (char === pattern[patternIdx]) {
				consecutive++;
				score += consecutive;
				patternIdx++;
			} else {
				consecutive = 0;
			}
		}

		return score;
	}

	function handleKeydown(e: KeyboardEvent) {
		switch (e.key) {
			case 'ArrowDown':
				e.preventDefault();
				selectedIndex = Math.min(selectedIndex + 1, results.length - 1);
				break;

			case 'ArrowUp':
				e.preventDefault();
				selectedIndex = Math.max(selectedIndex - 1, 0);
				break;

			case 'Enter':
				e.preventDefault();
				if (results[selectedIndex]) {
					selectResult(results[selectedIndex]);
				}
				break;

			case 'Escape':
				e.preventDefault();
				onClose();
				break;
		}
	}

	function selectResult(result: SearchResult) {
		switch (result.type) {
			case 'table':
			case 'view':
				tabStore.createTableTab(connectionId, result.schema, result.name);
				break;
			case 'function':
				// Open function
				break;
			case 'column':
				// Navigate to table and highlight column
				tabStore.createTableTab(connectionId, result.schema, result.parentName!);
				break;
		}
		onClose();
	}

	onMount(() => {
		inputRef?.focus();
	});

	$effect(() => {
		// Reset selection when results change
		selectedIndex = 0;
	});
</script>

<div class="search-overlay" onclick={onClose}>
	<div class="search-dialog" onclick={(e) => e.stopPropagation()}>
		<div class="search-input-wrapper">
			<Search size={20} />
			<input
				bind:this={inputRef}
				type="text"
				placeholder="Search tables, views, functions..."
				bind:value={query}
				onkeydown={handleKeydown}
			/>
		</div>

		{#if results.length > 0}
			<div class="search-results">
				{#each results as result, i}
					<button
						class="result-item"
						class:selected={i === selectedIndex}
						onclick={() => selectResult(result)}
						onmouseenter={() => (selectedIndex = i)}
					>
						<svelte:component this={result.icon} size={16} />
						<span class="result-name">
							{result.name}
							{#if result.parentName}
								<span class="result-parent">in {result.parentName}</span>
							{/if}
						</span>
						<span class="result-path">{result.schema}</span>
						<span class="result-type">{result.type}</span>
					</button>
				{/each}
			</div>
		{:else if query.trim()}
			<div class="no-results">No objects found</div>
		{:else}
			<div class="search-hint">Type to search across all schemas</div>
		{/if}
	</div>
</div>

<style>
	.search-overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.5);
		display: flex;
		justify-content: center;
		padding-top: 100px;
		z-index: 100;
	}

	.search-dialog {
		background: var(--surface-color);
		border-radius: 0.5rem;
		box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.1);
		width: 600px;
		max-height: 500px;
		overflow: hidden;
	}

	.search-input-wrapper {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 1rem;
		border-bottom: 1px solid var(--border-color);
	}

	.search-input-wrapper input {
		flex: 1;
		border: none;
		background: none;
		font-size: 1.125rem;
		outline: none;
	}

	.search-results {
		max-height: 400px;
		overflow-y: auto;
	}

	.result-item {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		width: 100%;
		padding: 0.625rem 1rem;
		border: none;
		background: none;
		text-align: left;
		cursor: pointer;
		transition: background 0.1s;
	}

	.result-item:hover,
	.result-item.selected {
		background: var(--hover-color);
	}

	.result-name {
		flex: 1;
		font-weight: 500;
	}

	.result-parent {
		font-weight: normal;
		color: var(--text-muted);
		font-size: 0.875rem;
	}

	.result-path {
		color: var(--text-muted);
		font-size: 0.8125rem;
	}

	.result-type {
		color: var(--text-muted);
		font-size: 0.75rem;
		text-transform: uppercase;
		padding: 0.125rem 0.375rem;
		background: var(--surface-secondary);
		border-radius: 0.25rem;
	}

	.no-results,
	.search-hint {
		padding: 2rem;
		text-align: center;
		color: var(--text-muted);
	}
</style>
```

### 16.5 DDL Generation (Rust)

```rust
// src-tauri/src/services/ddl.rs

use crate::error::Result;
use crate::models::schema::{Table, View, Function, Index, ForeignKey, Column};

pub struct DdlGenerator;

impl DdlGenerator {
    pub fn generate_create_table(table: &Table) -> String {
        let mut ddl = format!(
            "CREATE TABLE \"{}\".\"{}\" (\n",
            table.schema, table.name
        );

        // Columns
        let column_defs: Vec<String> = table.columns.iter()
            .map(Self::generate_column_def)
            .collect();
        ddl.push_str(&column_defs.join(",\n"));

        // Primary key
        if let Some(pk) = &table.primary_key {
            ddl.push_str(&format!(
                ",\n  CONSTRAINT \"{}\" PRIMARY KEY ({})",
                pk.name,
                pk.columns.iter()
                    .map(|c| format!("\"{}\"", c))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }

        // Unique constraints
        for uc in &table.unique_constraints {
            ddl.push_str(&format!(
                ",\n  CONSTRAINT \"{}\" UNIQUE ({})",
                uc.name,
                uc.columns.iter()
                    .map(|c| format!("\"{}\"", c))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }

        // Check constraints
        for cc in &table.check_constraints {
            ddl.push_str(&format!(
                ",\n  CONSTRAINT \"{}\" CHECK ({})",
                cc.name, cc.expression
            ));
        }

        // Foreign keys
        for fk in &table.foreign_keys {
            ddl.push_str(&Self::generate_fk_constraint(fk));
        }

        ddl.push_str("\n);\n\n");

        // Indexes (non-primary, non-unique constraint)
        for index in &table.indexes {
            if !index.is_primary && !index.is_unique {
                ddl.push_str(&Self::generate_create_index(index, &table.schema, &table.name));
                ddl.push_str("\n");
            }
        }

        // Table comment
        if let Some(comment) = &table.comment {
            ddl.push_str(&format!(
                "\nCOMMENT ON TABLE \"{}\".\"{}\" IS {};\n",
                table.schema, table.name, Self::quote_string(comment)
            ));
        }

        // Column comments
        for col in &table.columns {
            if let Some(comment) = &col.comment {
                ddl.push_str(&format!(
                    "COMMENT ON COLUMN \"{}\".\"{}\".\"{}\" IS {};\n",
                    table.schema, table.name, col.name, Self::quote_string(comment)
                ));
            }
        }

        ddl
    }

    fn generate_column_def(col: &Column) -> String {
        let mut def = format!("  \"{}\" {}", col.name, col.type_name);

        // Collation for text types
        // if let Some(collation) = &col.collation {
        //     def.push_str(&format!(" COLLATE \"{}\"", collation));
        // }

        if col.is_identity {
            let gen = col.identity_generation.as_deref().unwrap_or("BY DEFAULT");
            def.push_str(&format!(" GENERATED {} AS IDENTITY", gen));
        } else if col.is_generated {
            if let Some(expr) = &col.generation_expression {
                def.push_str(&format!(" GENERATED ALWAYS AS ({}) STORED", expr));
            }
        } else if let Some(default) = &col.default {
            def.push_str(&format!(" DEFAULT {}", default));
        }

        if !col.nullable {
            def.push_str(" NOT NULL");
        }

        def
    }

    fn generate_fk_constraint(fk: &ForeignKey) -> String {
        format!(
            ",\n  CONSTRAINT \"{}\" FOREIGN KEY ({}) REFERENCES \"{}\".\"{}\".({}) ON DELETE {} ON UPDATE {}{}",
            fk.name,
            fk.columns.iter().map(|c| format!("\"{}\"", c)).collect::<Vec<_>>().join(", "),
            fk.referenced_schema,
            fk.referenced_table,
            fk.referenced_columns.iter().map(|c| format!("\"{}\"", c)).collect::<Vec<_>>().join(", "),
            fk.on_delete,
            fk.on_update,
            if fk.deferrable {
                format!(" DEFERRABLE{}", if fk.initially_deferred { " INITIALLY DEFERRED" } else { "" })
            } else {
                String::new()
            }
        )
    }

    pub fn generate_create_index(index: &Index, schema: &str, table: &str) -> String {
        let mut ddl = String::from("CREATE ");

        if index.is_unique {
            ddl.push_str("UNIQUE ");
        }

        ddl.push_str(&format!(
            "INDEX \"{}\" ON \"{}\".\"{}\" USING {} ({})",
            index.name,
            schema,
            table,
            index.method,
            index.columns.iter().map(|c| format!("\"{}\"", c)).collect::<Vec<_>>().join(", ")
        ));

        if !index.include_columns.is_empty() {
            ddl.push_str(&format!(
                " INCLUDE ({})",
                index.include_columns.iter().map(|c| format!("\"{}\"", c)).collect::<Vec<_>>().join(", ")
            ));
        }

        if let Some(pred) = &index.predicate {
            ddl.push_str(&format!(" WHERE {}", pred));
        }

        ddl.push(';');
        ddl
    }

    pub fn generate_create_view(view: &View) -> String {
        format!(
            "CREATE OR REPLACE VIEW \"{}\".\"{}\" AS\n{};\n",
            view.schema, view.name, view.definition
        )
    }

    pub fn generate_create_function(func: &Function) -> String {
        let mut ddl = format!(
            "CREATE OR REPLACE FUNCTION \"{}\".\"{}\"({})\n",
            func.schema, func.name, func.arguments
        );

        ddl.push_str(&format!("RETURNS {}\n", func.return_type));
        ddl.push_str(&format!("LANGUAGE {}\n", func.language));

        match func.volatility.as_str() {
            "i" => ddl.push_str("IMMUTABLE\n"),
            "s" => ddl.push_str("STABLE\n"),
            "v" => ddl.push_str("VOLATILE\n"),
            _ => {}
        }

        if func.is_strict {
            ddl.push_str("STRICT\n");
        }

        if func.is_security_definer {
            ddl.push_str("SECURITY DEFINER\n");
        }

        ddl.push_str("AS $function$\n");
        ddl.push_str(&func.source);
        ddl.push_str("\n$function$;\n");

        if let Some(comment) = &func.comment {
            ddl.push_str(&format!(
                "\nCOMMENT ON FUNCTION \"{}\".\"{}\"({}) IS {};\n",
                func.schema, func.name, func.arguments, Self::quote_string(comment)
            ));
        }

        ddl
    }

    pub fn generate_drop_statement(object_type: &str, schema: &str, name: &str, cascade: bool) -> String {
        let cascade_str = if cascade { " CASCADE" } else { "" };
        format!(
            "DROP {} IF EXISTS \"{}\".\"{}\"{};",
            object_type.to_uppercase(),
            schema,
            name,
            cascade_str
        )
    }

    fn quote_string(s: &str) -> String {
        format!("'{}'", s.replace('\'', "''"))
    }
}
```

## Acceptance Criteria

1. **Tree Display**
   - Show all database objects in hierarchical tree
   - Support lazy loading for large schemas
   - Display counts for folders (e.g., "Tables (42)")
   - Show size/row count for tables

2. **Tree Navigation**
   - Expand/collapse nodes
   - Single-click to select
   - Double-click to open
   - Keyboard navigation (arrows, enter)

3. **Context Menus**
   - Table: View Data, New Query, Edit, DDL, Drop, Truncate
   - View: View Data, Edit, DDL, Drop
   - Function: Open, Execute, DDL, Drop
   - Index: DDL, Reindex, Drop
   - Column: Add to Query, Copy Name

4. **DDL Generation**
   - Generate complete CREATE statements
   - Include comments, constraints, indexes
   - Support DROP with CASCADE option

5. **Object Search**
   - Fuzzy search across all objects
   - Keyboard navigation (up/down/enter)
   - Show object type and schema
   - Open selected object in tab

6. **Refresh**
   - Manual refresh button
   - Refresh specific nodes
   - Clear and reload full schema

## MCP Testing Instructions

### Using Tauri MCP

```typescript
// Connect to app
await mcp.driver_session({ action: 'start' });

// Verify schema tree renders
const snapshot = await mcp.webview_dom_snapshot({ type: 'accessibility' });
assert(snapshot.includes('Schemas'));

// Expand a schema
await mcp.webview_click({
	selector: '[data-type="schema"]:first-child',
	element: 'Schema node'
});

// Right-click a table for context menu
await mcp.webview_interact({
	action: 'click',
	selector: '[data-type="table"]:first-child',
	button: 'right'
});

// Verify context menu appears
await mcp.browser_wait_for({ text: 'View Data' });

// Test object search
await mcp.browser_press_key({ key: 'p', modifiers: ['Meta', 'Shift'] });
await mcp.webview_type({
	selector: '.search-input',
	text: 'users'
});

// Verify search results
await mcp.browser_wait_for({ text: 'public.users' });
```

## Dependencies

- Feature 10: Schema Introspection
- Feature 07: Connection Management
- Feature 13: Tab Management
