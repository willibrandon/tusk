# Feature 23: Extension Manager

## Overview

The Extension Manager provides a GUI for managing PostgreSQL extensions, including viewing installed extensions, installing new extensions, upgrading to newer versions, and viewing extension details and dependencies.

## Goals

- List all available and installed extensions
- Install extensions with schema selection
- Upgrade extensions to newer versions
- Uninstall extensions (with CASCADE option)
- Display extension details, dependencies, and objects
- Generate SQL for extension operations

## Dependencies

- Feature 07: Connection Pool Management
- Feature 10: Schema Cache (for schema list)

## Technical Specification

### 23.1 Extension Data Models

```typescript
// src/lib/types/extensions.ts

export interface Extension {
	name: string;
	installedVersion: string | null;
	defaultVersion: string;
	availableVersions: string[];
	schema: string | null;
	relocatable: boolean;
	comment: string | null;
	requires: string[];
	isInstalled: boolean;
}

export interface ExtensionDetail {
	name: string;
	version: string;
	schema: string;
	description: string;
	requires: string[];
	objects: ExtensionObject[];
	config: ExtensionConfig[];
}

export interface ExtensionObject {
	type: string; // 'function', 'type', 'operator', 'table', etc.
	schema: string;
	name: string;
	identity: string; // Full qualified name with signature
}

export interface ExtensionConfig {
	name: string;
	value: string;
	description: string;
	unit: string | null;
	vartype: string;
	enumVals: string[] | null;
	minVal: string | null;
	maxVal: string | null;
}

export interface InstallExtensionOptions {
	name: string;
	version?: string;
	schema?: string;
	cascade: boolean;
}

export interface UpgradeExtensionOptions {
	name: string;
	targetVersion?: string;
}
```

### 23.2 Extension Service (Rust)

```rust
// src-tauri/src/services/extension.rs

use serde::{Deserialize, Serialize};
use tokio_postgres::Client;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Extension {
    pub name: String,
    pub installed_version: Option<String>,
    pub default_version: String,
    pub available_versions: Vec<String>,
    pub schema: Option<String>,
    pub relocatable: bool,
    pub comment: Option<String>,
    pub requires: Vec<String>,
    pub is_installed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionDetail {
    pub name: String,
    pub version: String,
    pub schema: String,
    pub description: String,
    pub requires: Vec<String>,
    pub objects: Vec<ExtensionObject>,
    pub config: Vec<ExtensionConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionObject {
    pub object_type: String,
    pub schema: String,
    pub name: String,
    pub identity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionConfig {
    pub name: String,
    pub value: String,
    pub description: String,
    pub unit: Option<String>,
    pub vartype: String,
    pub enum_vals: Option<Vec<String>>,
    pub min_val: Option<String>,
    pub max_val: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallExtensionOptions {
    pub name: String,
    pub version: Option<String>,
    pub schema: Option<String>,
    pub cascade: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpgradeExtensionOptions {
    pub name: String,
    pub target_version: Option<String>,
}

pub struct ExtensionService;

impl ExtensionService {
    /// Get all available and installed extensions
    pub async fn get_extensions(client: &Client) -> Result<Vec<Extension>, ExtensionError> {
        let rows = client
            .query(
                r#"
                SELECT
                    a.name,
                    e.extversion AS installed_version,
                    a.default_version,
                    n.nspname AS schema,
                    a.relocatable,
                    a.comment,
                    COALESCE(a.requires, ARRAY[]::name[]) AS requires,
                    e.oid IS NOT NULL AS is_installed
                FROM pg_available_extensions a
                LEFT JOIN pg_extension e ON e.extname = a.name
                LEFT JOIN pg_namespace n ON n.oid = e.extnamespace
                ORDER BY a.name
                "#,
                &[],
            )
            .await?;

        let extensions: Vec<Extension> = rows
            .iter()
            .map(|row| {
                let requires: Vec<String> = row
                    .get::<_, Vec<String>>("requires");

                Extension {
                    name: row.get("name"),
                    installed_version: row.get("installed_version"),
                    default_version: row.get("default_version"),
                    available_versions: vec![], // Will be populated separately
                    schema: row.get("schema"),
                    relocatable: row.get("relocatable"),
                    comment: row.get("comment"),
                    requires,
                    is_installed: row.get("is_installed"),
                }
            })
            .collect();

        Ok(extensions)
    }

    /// Get available versions for an extension
    pub async fn get_available_versions(
        client: &Client,
        extension_name: &str,
    ) -> Result<Vec<String>, ExtensionError> {
        let rows = client
            .query(
                r#"
                SELECT version
                FROM pg_available_extension_versions
                WHERE name = $1
                ORDER BY version DESC
                "#,
                &[&extension_name],
            )
            .await?;

        let versions: Vec<String> = rows.iter().map(|row| row.get("version")).collect();
        Ok(versions)
    }

    /// Get detailed information about an installed extension
    pub async fn get_extension_detail(
        client: &Client,
        extension_name: &str,
    ) -> Result<ExtensionDetail, ExtensionError> {
        // Get basic info
        let info_row = client
            .query_one(
                r#"
                SELECT
                    e.extname AS name,
                    e.extversion AS version,
                    n.nspname AS schema,
                    COALESCE(a.comment, '') AS description,
                    COALESCE(a.requires, ARRAY[]::name[]) AS requires
                FROM pg_extension e
                JOIN pg_namespace n ON n.oid = e.extnamespace
                LEFT JOIN pg_available_extensions a ON a.name = e.extname
                WHERE e.extname = $1
                "#,
                &[&extension_name],
            )
            .await?;

        let name: String = info_row.get("name");
        let version: String = info_row.get("version");
        let schema: String = info_row.get("schema");
        let description: String = info_row.get("description");
        let requires: Vec<String> = info_row.get("requires");

        // Get objects created by extension
        let object_rows = client
            .query(
                r#"
                SELECT
                    CASE classid
                        WHEN 'pg_proc'::regclass THEN 'function'
                        WHEN 'pg_type'::regclass THEN 'type'
                        WHEN 'pg_operator'::regclass THEN 'operator'
                        WHEN 'pg_class'::regclass THEN
                            CASE (SELECT relkind FROM pg_class WHERE oid = objid)
                                WHEN 'r' THEN 'table'
                                WHEN 'i' THEN 'index'
                                WHEN 'S' THEN 'sequence'
                                WHEN 'v' THEN 'view'
                                WHEN 'm' THEN 'materialized view'
                                ELSE 'relation'
                            END
                        WHEN 'pg_cast'::regclass THEN 'cast'
                        WHEN 'pg_opclass'::regclass THEN 'operator class'
                        WHEN 'pg_opfamily'::regclass THEN 'operator family'
                        WHEN 'pg_am'::regclass THEN 'access method'
                        ELSE 'other'
                    END AS object_type,
                    COALESCE(n.nspname, '') AS schema,
                    pg_describe_object(classid, objid, objsubid) AS identity
                FROM pg_depend d
                JOIN pg_extension e ON e.oid = d.refobjid
                LEFT JOIN pg_class c ON c.oid = d.objid
                LEFT JOIN pg_namespace n ON n.oid = c.relnamespace
                WHERE e.extname = $1
                  AND d.deptype = 'e'
                ORDER BY object_type, identity
                "#,
                &[&extension_name],
            )
            .await?;

        let objects: Vec<ExtensionObject> = object_rows
            .iter()
            .map(|row| {
                let identity: String = row.get("identity");
                let name = identity.split('.').last().unwrap_or(&identity).to_string();

                ExtensionObject {
                    object_type: row.get("object_type"),
                    schema: row.get("schema"),
                    name,
                    identity,
                }
            })
            .collect();

        // Get extension configuration parameters
        let config = Self::get_extension_config(client, &name).await.unwrap_or_default();

        Ok(ExtensionDetail {
            name,
            version,
            schema,
            description,
            requires,
            objects,
            config,
        })
    }

    async fn get_extension_config(
        client: &Client,
        extension_name: &str,
    ) -> Result<Vec<ExtensionConfig>, ExtensionError> {
        // Query for extension-specific GUCs
        let rows = client
            .query(
                r#"
                SELECT
                    name,
                    setting AS value,
                    short_desc AS description,
                    unit,
                    vartype,
                    enumvals AS enum_vals,
                    min_val,
                    max_val
                FROM pg_settings
                WHERE name LIKE $1 || '.%'
                ORDER BY name
                "#,
                &[&extension_name],
            )
            .await?;

        let config: Vec<ExtensionConfig> = rows
            .iter()
            .map(|row| ExtensionConfig {
                name: row.get("name"),
                value: row.get("value"),
                description: row.get::<_, Option<String>>("description").unwrap_or_default(),
                unit: row.get("unit"),
                vartype: row.get("vartype"),
                enum_vals: row.get("enum_vals"),
                min_val: row.get("min_val"),
                max_val: row.get("max_val"),
            })
            .collect();

        Ok(config)
    }

    /// Install an extension
    pub async fn install_extension(
        client: &Client,
        options: &InstallExtensionOptions,
    ) -> Result<(), ExtensionError> {
        let sql = Self::build_install_sql(options);
        client.execute(&sql, &[]).await?;
        Ok(())
    }

    /// Build CREATE EXTENSION SQL
    pub fn build_install_sql(options: &InstallExtensionOptions) -> String {
        let mut sql = format!("CREATE EXTENSION IF NOT EXISTS {}", Self::quote_ident(&options.name));

        if let Some(ref schema) = options.schema {
            sql.push_str(&format!(" SCHEMA {}", Self::quote_ident(schema)));
        }

        if let Some(ref version) = options.version {
            sql.push_str(&format!(" VERSION '{}'", Self::escape_string(version)));
        }

        if options.cascade {
            sql.push_str(" CASCADE");
        }

        sql
    }

    /// Upgrade an extension
    pub async fn upgrade_extension(
        client: &Client,
        options: &UpgradeExtensionOptions,
    ) -> Result<(), ExtensionError> {
        let sql = Self::build_upgrade_sql(options);
        client.execute(&sql, &[]).await?;
        Ok(())
    }

    /// Build ALTER EXTENSION UPDATE SQL
    pub fn build_upgrade_sql(options: &UpgradeExtensionOptions) -> String {
        let mut sql = format!("ALTER EXTENSION {} UPDATE", Self::quote_ident(&options.name));

        if let Some(ref version) = options.target_version {
            sql.push_str(&format!(" TO '{}'", Self::escape_string(version)));
        }

        sql
    }

    /// Uninstall an extension
    pub async fn uninstall_extension(
        client: &Client,
        extension_name: &str,
        cascade: bool,
    ) -> Result<(), ExtensionError> {
        let sql = Self::build_uninstall_sql(extension_name, cascade);
        client.execute(&sql, &[]).await?;
        Ok(())
    }

    /// Build DROP EXTENSION SQL
    pub fn build_uninstall_sql(extension_name: &str, cascade: bool) -> String {
        let mut sql = format!("DROP EXTENSION IF EXISTS {}", Self::quote_ident(extension_name));

        if cascade {
            sql.push_str(" CASCADE");
        }

        sql
    }

    fn quote_ident(s: &str) -> String {
        if s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') {
            s.to_string()
        } else {
            format!("\"{}\"", s.replace('"', "\"\""))
        }
    }

    fn escape_string(s: &str) -> String {
        s.replace('\'', "''")
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ExtensionError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] tokio_postgres::Error),

    #[error("Extension not found: {0}")]
    NotFound(String),

    #[error("Extension not installed: {0}")]
    NotInstalled(String),
}
```

### 23.3 Tauri Commands

```rust
// src-tauri/src/commands/extension.rs

use tauri::State;
use crate::services::extension::{
    ExtensionService, Extension, ExtensionDetail,
    InstallExtensionOptions, UpgradeExtensionOptions,
};
use crate::state::AppState;
use crate::error::Error;

#[tauri::command]
pub async fn get_extensions(
    state: State<'_, AppState>,
    conn_id: String,
) -> Result<Vec<Extension>, Error> {
    let pool = state.get_connection(&conn_id)?;
    let client = pool.get().await?;
    let extensions = ExtensionService::get_extensions(&client).await?;
    Ok(extensions)
}

#[tauri::command]
pub async fn get_extension_versions(
    state: State<'_, AppState>,
    conn_id: String,
    extension_name: String,
) -> Result<Vec<String>, Error> {
    let pool = state.get_connection(&conn_id)?;
    let client = pool.get().await?;
    let versions = ExtensionService::get_available_versions(&client, &extension_name).await?;
    Ok(versions)
}

#[tauri::command]
pub async fn get_extension_detail(
    state: State<'_, AppState>,
    conn_id: String,
    extension_name: String,
) -> Result<ExtensionDetail, Error> {
    let pool = state.get_connection(&conn_id)?;
    let client = pool.get().await?;
    let detail = ExtensionService::get_extension_detail(&client, &extension_name).await?;
    Ok(detail)
}

#[tauri::command]
pub async fn install_extension(
    state: State<'_, AppState>,
    conn_id: String,
    options: InstallExtensionOptions,
) -> Result<(), Error> {
    let pool = state.get_connection(&conn_id)?;
    let client = pool.get().await?;
    ExtensionService::install_extension(&client, &options).await?;
    Ok(())
}

#[tauri::command]
pub async fn upgrade_extension(
    state: State<'_, AppState>,
    conn_id: String,
    options: UpgradeExtensionOptions,
) -> Result<(), Error> {
    let pool = state.get_connection(&conn_id)?;
    let client = pool.get().await?;
    ExtensionService::upgrade_extension(&client, &options).await?;
    Ok(())
}

#[tauri::command]
pub async fn uninstall_extension(
    state: State<'_, AppState>,
    conn_id: String,
    extension_name: String,
    cascade: bool,
) -> Result<(), Error> {
    let pool = state.get_connection(&conn_id)?;
    let client = pool.get().await?;
    ExtensionService::uninstall_extension(&client, &extension_name, cascade).await?;
    Ok(())
}

#[tauri::command]
pub fn generate_install_extension_sql(options: InstallExtensionOptions) -> String {
    ExtensionService::build_install_sql(&options)
}

#[tauri::command]
pub fn generate_upgrade_extension_sql(options: UpgradeExtensionOptions) -> String {
    ExtensionService::build_upgrade_sql(&options)
}

#[tauri::command]
pub fn generate_uninstall_extension_sql(extension_name: String, cascade: bool) -> String {
    ExtensionService::build_uninstall_sql(&extension_name, cascade)
}
```

### 23.4 Extension Store (Svelte)

```typescript
// src/lib/stores/extensionStore.svelte.ts

import { invoke } from '@tauri-apps/api/core';
import type {
	Extension,
	ExtensionDetail,
	InstallExtensionOptions,
	UpgradeExtensionOptions
} from '$lib/types/extensions';

interface ExtensionState {
	extensions: Extension[];
	selectedExtension: ExtensionDetail | null;
	loading: boolean;
	error: string | null;
	filter: string;
	showInstalledOnly: boolean;
}

export function createExtensionStore() {
	let state = $state<ExtensionState>({
		extensions: [],
		selectedExtension: null,
		loading: false,
		error: null,
		filter: '',
		showInstalledOnly: false
	});

	async function loadExtensions(connId: string) {
		state.loading = true;
		state.error = null;

		try {
			state.extensions = await invoke<Extension[]>('get_extensions', { connId });
		} catch (err) {
			state.error = err instanceof Error ? err.message : String(err);
		} finally {
			state.loading = false;
		}
	}

	async function loadExtensionDetail(connId: string, extensionName: string) {
		if (!state.extensions.find((e) => e.name === extensionName)?.isInstalled) {
			state.selectedExtension = null;
			return;
		}

		try {
			state.selectedExtension = await invoke<ExtensionDetail>('get_extension_detail', {
				connId,
				extensionName
			});
		} catch (err) {
			state.error = err instanceof Error ? err.message : String(err);
		}
	}

	async function installExtension(connId: string, options: InstallExtensionOptions) {
		try {
			await invoke('install_extension', { connId, options });
			await loadExtensions(connId);
		} catch (err) {
			throw err;
		}
	}

	async function upgradeExtension(connId: string, options: UpgradeExtensionOptions) {
		try {
			await invoke('upgrade_extension', { connId, options });
			await loadExtensions(connId);
			if (state.selectedExtension?.name === options.name) {
				await loadExtensionDetail(connId, options.name);
			}
		} catch (err) {
			throw err;
		}
	}

	async function uninstallExtension(connId: string, extensionName: string, cascade: boolean) {
		try {
			await invoke('uninstall_extension', { connId, extensionName, cascade });
			if (state.selectedExtension?.name === extensionName) {
				state.selectedExtension = null;
			}
			await loadExtensions(connId);
		} catch (err) {
			throw err;
		}
	}

	function setFilter(filter: string) {
		state.filter = filter;
	}

	function setShowInstalledOnly(value: boolean) {
		state.showInstalledOnly = value;
	}

	function clearSelection() {
		state.selectedExtension = null;
	}

	// Derived: filtered extensions
	const filteredExtensions = $derived(
		state.extensions.filter((ext) => {
			if (state.showInstalledOnly && !ext.isInstalled) return false;
			if (state.filter) {
				const search = state.filter.toLowerCase();
				return (
					ext.name.toLowerCase().includes(search) ||
					(ext.comment?.toLowerCase().includes(search) ?? false)
				);
			}
			return true;
		})
	);

	return {
		get extensions() {
			return state.extensions;
		},
		get filteredExtensions() {
			return filteredExtensions;
		},
		get selectedExtension() {
			return state.selectedExtension;
		},
		get loading() {
			return state.loading;
		},
		get error() {
			return state.error;
		},
		get filter() {
			return state.filter;
		},
		get showInstalledOnly() {
			return state.showInstalledOnly;
		},

		loadExtensions,
		loadExtensionDetail,
		installExtension,
		upgradeExtension,
		uninstallExtension,
		setFilter,
		setShowInstalledOnly,
		clearSelection
	};
}

export const extensionStore = createExtensionStore();
```

### 23.5 Extension List Component

```svelte
<!-- src/lib/components/extensions/ExtensionList.svelte -->
<script lang="ts">
	import type { Extension } from '$lib/types/extensions';
	import { extensionStore } from '$lib/stores/extensionStore.svelte';

	interface Props {
		connId: string;
		onSelect: (ext: Extension) => void;
		onInstall: (ext: Extension) => void;
	}

	let { connId, onSelect, onInstall }: Props = $props();
</script>

<div class="flex flex-col h-full">
	<!-- Toolbar -->
	<div class="flex items-center gap-2 p-4 border-b border-gray-200 dark:border-gray-700">
		<input
			type="text"
			value={extensionStore.filter}
			oninput={(e) => extensionStore.setFilter(e.currentTarget.value)}
			placeholder="Search extensions..."
			class="flex-1 px-3 py-2 border border-gray-300 dark:border-gray-600 rounded
             bg-white dark:bg-gray-700 text-sm"
		/>
		<label class="flex items-center gap-2 text-sm">
			<input
				type="checkbox"
				checked={extensionStore.showInstalledOnly}
				onchange={(e) => extensionStore.setShowInstalledOnly(e.currentTarget.checked)}
				class="rounded border-gray-300"
			/>
			Installed only
		</label>
		<button
			onclick={() => extensionStore.loadExtensions(connId)}
			class="px-3 py-2 text-sm text-gray-600 dark:text-gray-400
             hover:text-gray-900 dark:hover:text-gray-100"
		>
			Refresh
		</button>
	</div>

	<!-- Extension List -->
	<div class="flex-1 overflow-auto">
		{#if extensionStore.loading}
			<div class="flex items-center justify-center h-full">
				<div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
			</div>
		{:else if extensionStore.error}
			<div class="p-4 text-center text-red-500">{extensionStore.error}</div>
		{:else}
			<table class="min-w-full divide-y divide-gray-200 dark:divide-gray-700">
				<thead class="bg-gray-50 dark:bg-gray-900/50 sticky top-0">
					<tr>
						<th
							class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider"
						>
							Extension
						</th>
						<th
							class="px-4 py-3 text-center text-xs font-medium text-gray-500 uppercase tracking-wider"
						>
							Version
						</th>
						<th
							class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider"
						>
							Schema
						</th>
						<th
							class="px-4 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider"
						>
							Actions
						</th>
					</tr>
				</thead>
				<tbody class="divide-y divide-gray-200 dark:divide-gray-700">
					{#each extensionStore.filteredExtensions as ext (ext.name)}
						<tr
							class="hover:bg-gray-50 dark:hover:bg-gray-700/50 cursor-pointer
                     {extensionStore.selectedExtension?.name === ext.name
								? 'bg-blue-50 dark:bg-blue-900/20'
								: ''}"
							onclick={() => onSelect(ext)}
						>
							<td class="px-4 py-3">
								<div class="flex items-center gap-2">
									{#if ext.isInstalled}
										<span class="w-2 h-2 rounded-full bg-green-500" title="Installed"></span>
									{:else}
										<span
											class="w-2 h-2 rounded-full bg-gray-300 dark:bg-gray-600"
											title="Not installed"
										></span>
									{/if}
									<div>
										<div class="font-medium">{ext.name}</div>
										{#if ext.comment}
											<div class="text-xs text-gray-500 dark:text-gray-400 truncate max-w-md">
												{ext.comment}
											</div>
										{/if}
									</div>
								</div>
							</td>
							<td class="px-4 py-3 text-center text-sm">
								{#if ext.isInstalled}
									<span class="font-mono">{ext.installedVersion}</span>
									{#if ext.installedVersion !== ext.defaultVersion}
										<span
											class="ml-1 text-xs text-amber-600 dark:text-amber-400"
											title="Upgrade available"
										>
											→ {ext.defaultVersion}
										</span>
									{/if}
								{:else}
									<span class="text-gray-400">{ext.defaultVersion}</span>
								{/if}
							</td>
							<td class="px-4 py-3 text-sm">
								{#if ext.schema}
									<span
										class="font-mono text-xs bg-gray-100 dark:bg-gray-700 px-1.5 py-0.5 rounded"
									>
										{ext.schema}
									</span>
								{:else}
									<span class="text-gray-400">-</span>
								{/if}
							</td>
							<td class="px-4 py-3 text-right">
								{#if ext.isInstalled}
									<button
										onclick={(e) => {
											e.stopPropagation(); /* uninstall */
										}}
										class="text-red-600 hover:text-red-700 dark:text-red-400
                           dark:hover:text-red-300 text-sm"
									>
										Uninstall
									</button>
								{:else}
									<button
										onclick={(e) => {
											e.stopPropagation();
											onInstall(ext);
										}}
										class="text-blue-600 hover:text-blue-700 dark:text-blue-400
                           dark:hover:text-blue-300 text-sm"
									>
										Install
									</button>
								{/if}
							</td>
						</tr>
					{:else}
						<tr>
							<td colspan="4" class="px-4 py-8 text-center text-gray-500">
								{extensionStore.filter ? 'No extensions match the filter' : 'No extensions found'}
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		{/if}
	</div>
</div>
```

### 23.6 Extension Detail Panel

```svelte
<!-- src/lib/components/extensions/ExtensionDetail.svelte -->
<script lang="ts">
	import type { ExtensionDetail } from '$lib/types/extensions';

	interface Props {
		detail: ExtensionDetail;
		onUpgrade: () => void;
		onUninstall: () => void;
	}

	let { detail, onUpgrade, onUninstall }: Props = $props();

	let activeTab = $state<'objects' | 'config'>('objects');

	// Group objects by type
	const objectsByType = $derived(() => {
		const groups: Record<string, typeof detail.objects> = {};
		for (const obj of detail.objects) {
			if (!groups[obj.objectType]) {
				groups[obj.objectType] = [];
			}
			groups[obj.objectType].push(obj);
		}
		return groups;
	});

	const objectTypes = $derived(Object.keys(objectsByType()).sort());
</script>

<div class="flex flex-col h-full">
	<!-- Header -->
	<div class="p-4 border-b border-gray-200 dark:border-gray-700">
		<div class="flex items-center justify-between">
			<div>
				<h2 class="text-lg font-semibold">{detail.name}</h2>
				<p class="text-sm text-gray-500 dark:text-gray-400">{detail.description}</p>
			</div>
			<div class="flex items-center gap-2">
				<button
					onclick={onUpgrade}
					class="px-3 py-1.5 text-sm bg-blue-100 text-blue-700 dark:bg-blue-900/30
                 dark:text-blue-400 rounded hover:bg-blue-200 dark:hover:bg-blue-900/50"
				>
					Upgrade
				</button>
				<button
					onclick={onUninstall}
					class="px-3 py-1.5 text-sm bg-red-100 text-red-700 dark:bg-red-900/30
                 dark:text-red-400 rounded hover:bg-red-200 dark:hover:bg-red-900/50"
				>
					Uninstall
				</button>
			</div>
		</div>

		<!-- Info Row -->
		<div class="flex items-center gap-6 mt-3 text-sm">
			<div>
				<span class="text-gray-500">Version:</span>
				<span class="ml-1 font-mono">{detail.version}</span>
			</div>
			<div>
				<span class="text-gray-500">Schema:</span>
				<span class="ml-1 font-mono">{detail.schema}</span>
			</div>
			{#if detail.requires.length > 0}
				<div>
					<span class="text-gray-500">Requires:</span>
					<span class="ml-1">{detail.requires.join(', ')}</span>
				</div>
			{/if}
		</div>
	</div>

	<!-- Tabs -->
	<div class="flex border-b border-gray-200 dark:border-gray-700">
		<button
			onclick={() => (activeTab = 'objects')}
			class="px-4 py-2 text-sm font-medium transition-colors
             {activeTab === 'objects'
				? 'border-b-2 border-blue-500 text-blue-600 dark:text-blue-400'
				: 'text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300'}"
		>
			Objects ({detail.objects.length})
		</button>
		<button
			onclick={() => (activeTab = 'config')}
			class="px-4 py-2 text-sm font-medium transition-colors
             {activeTab === 'config'
				? 'border-b-2 border-blue-500 text-blue-600 dark:text-blue-400'
				: 'text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300'}"
		>
			Configuration ({detail.config.length})
		</button>
	</div>

	<!-- Content -->
	<div class="flex-1 overflow-auto p-4">
		{#if activeTab === 'objects'}
			{#if detail.objects.length === 0}
				<p class="text-gray-500 text-center py-8">No objects found</p>
			{:else}
				<div class="space-y-4">
					{#each objectTypes as objType}
						<div>
							<h3 class="text-sm font-medium text-gray-700 dark:text-gray-300 mb-2 capitalize">
								{objType}s ({objectsByType()[objType].length})
							</h3>
							<div class="bg-gray-50 dark:bg-gray-900/50 rounded p-2 space-y-1">
								{#each objectsByType()[objType] as obj}
									<div
										class="text-sm font-mono py-1 px-2 hover:bg-gray-100 dark:hover:bg-gray-800 rounded"
									>
										{obj.identity}
									</div>
								{/each}
							</div>
						</div>
					{/each}
				</div>
			{/if}
		{:else if activeTab === 'config'}
			{#if detail.config.length === 0}
				<p class="text-gray-500 text-center py-8">No configuration parameters</p>
			{:else}
				<div class="space-y-3">
					{#each detail.config as param}
						<div class="bg-gray-50 dark:bg-gray-900/50 rounded p-3">
							<div class="flex items-center justify-between">
								<span class="font-mono text-sm">{param.name}</span>
								<span class="font-mono text-sm text-blue-600 dark:text-blue-400">
									{param.value}
									{#if param.unit}
										<span class="text-gray-500">{param.unit}</span>
									{/if}
								</span>
							</div>
							{#if param.description}
								<p class="text-xs text-gray-500 mt-1">{param.description}</p>
							{/if}
						</div>
					{/each}
				</div>
			{/if}
		{/if}
	</div>
</div>
```

### 23.7 Install Extension Dialog

```svelte
<!-- src/lib/components/extensions/InstallExtensionDialog.svelte -->
<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { Extension, InstallExtensionOptions } from '$lib/types/extensions';
	import { invoke } from '@tauri-apps/api/core';

	interface Props {
		open: boolean;
		connId: string;
		extension: Extension;
		schemas: string[];
	}

	let { open = $bindable(), connId, extension, schemas }: Props = $props();

	const dispatch = createEventDispatcher<{
		install: InstallExtensionOptions;
		cancel: void;
	}>();

	let selectedVersion = $state(extension.defaultVersion);
	let selectedSchema = $state('public');
	let cascade = $state(true);
	let availableVersions = $state<string[]>([]);
	let loading = $state(false);
	let generatedSql = $state('');

	// Load available versions
	$effect(() => {
		if (open) {
			loadVersions();
			updateSql();
		}
	});

	async function loadVersions() {
		try {
			availableVersions = await invoke<string[]>('get_extension_versions', {
				connId,
				extensionName: extension.name
			});
		} catch {
			availableVersions = [extension.defaultVersion];
		}
	}

	function updateSql() {
		const options: InstallExtensionOptions = {
			name: extension.name,
			version: selectedVersion !== extension.defaultVersion ? selectedVersion : undefined,
			schema: selectedSchema !== 'public' ? selectedSchema : undefined,
			cascade
		};

		generatedSql = `CREATE EXTENSION IF NOT EXISTS ${extension.name}`;
		if (options.schema) {
			generatedSql += ` SCHEMA ${options.schema}`;
		}
		if (options.version) {
			generatedSql += ` VERSION '${options.version}'`;
		}
		if (options.cascade) {
			generatedSql += ` CASCADE`;
		}
		generatedSql += ';';
	}

	// Update SQL when options change
	$effect(() => {
		selectedVersion;
		selectedSchema;
		cascade;
		updateSql();
	});

	function handleInstall() {
		dispatch('install', {
			name: extension.name,
			version: selectedVersion !== extension.defaultVersion ? selectedVersion : undefined,
			schema: selectedSchema !== 'public' ? selectedSchema : undefined,
			cascade
		});
		open = false;
	}

	function handleCancel() {
		dispatch('cancel');
		open = false;
	}
</script>

{#if open}
	<div
		class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
		role="dialog"
		aria-modal="true"
	>
		<div
			class="bg-white dark:bg-gray-800 rounded-lg shadow-xl w-[500px] max-h-[80vh] overflow-hidden"
		>
			<!-- Header -->
			<div class="px-4 py-3 border-b border-gray-200 dark:border-gray-700">
				<h2 class="text-lg font-semibold">Install Extension: {extension.name}</h2>
			</div>

			<!-- Body -->
			<div class="p-4 space-y-4">
				{#if extension.comment}
					<p class="text-sm text-gray-600 dark:text-gray-400">{extension.comment}</p>
				{/if}

				<!-- Dependencies -->
				{#if extension.requires.length > 0}
					<div
						class="p-3 bg-blue-50 dark:bg-blue-900/20 border border-blue-200
                      dark:border-blue-800 rounded text-sm"
					>
						<strong>Requires:</strong>
						<span class="ml-1">{extension.requires.join(', ')}</span>
					</div>
				{/if}

				<!-- Version -->
				<div>
					<label class="block text-sm font-medium mb-1">Version</label>
					<select
						bind:value={selectedVersion}
						class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded
                   bg-white dark:bg-gray-700 text-sm"
					>
						{#each availableVersions as version}
							<option value={version}>
								{version}
								{version === extension.defaultVersion ? '(default)' : ''}
							</option>
						{/each}
					</select>
				</div>

				<!-- Schema -->
				<div>
					<label class="block text-sm font-medium mb-1">Schema</label>
					<select
						bind:value={selectedSchema}
						disabled={!extension.relocatable}
						class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded
                   bg-white dark:bg-gray-700 text-sm disabled:opacity-50"
					>
						{#each schemas as schema}
							<option value={schema}>{schema}</option>
						{/each}
					</select>
					{#if !extension.relocatable}
						<p class="text-xs text-gray-500 mt-1">
							This extension is not relocatable and will be installed in its default schema.
						</p>
					{/if}
				</div>

				<!-- Cascade -->
				<label class="flex items-start gap-3 cursor-pointer">
					<input
						type="checkbox"
						bind:checked={cascade}
						class="mt-1 rounded border-gray-300 dark:border-gray-600"
					/>
					<div>
						<span class="font-medium text-sm">CASCADE</span>
						<p class="text-xs text-gray-500 dark:text-gray-400">
							Automatically install required dependencies
						</p>
					</div>
				</label>

				<!-- SQL Preview -->
				<div>
					<label class="block text-sm font-medium mb-1">Generated SQL</label>
					<pre class="p-3 bg-gray-100 dark:bg-gray-900 rounded font-mono text-xs overflow-auto">
{generatedSql}
          </pre>
				</div>
			</div>

			<!-- Footer -->
			<div class="px-4 py-3 border-t border-gray-200 dark:border-gray-700 flex justify-end gap-2">
				<button
					onclick={handleCancel}
					class="px-4 py-2 text-sm text-gray-700 dark:text-gray-300
                 hover:bg-gray-100 dark:hover:bg-gray-700 rounded"
				>
					Cancel
				</button>
				<button
					onclick={handleInstall}
					class="px-4 py-2 text-sm bg-blue-600 text-white rounded hover:bg-blue-700"
				>
					Install
				</button>
			</div>
		</div>
	</div>
{/if}
```

## Acceptance Criteria

1. **Extension Listing**
   - [ ] Display all available extensions
   - [ ] Show installed status and version
   - [ ] Indicate upgrade availability
   - [ ] Filter by name and installed status
   - [ ] Show extension description

2. **Extension Installation**
   - [ ] Select version to install
   - [ ] Choose target schema (if relocatable)
   - [ ] CASCADE option for dependencies
   - [ ] Preview generated SQL
   - [ ] Handle installation errors

3. **Extension Details**
   - [ ] Show installed extension info
   - [ ] List all objects created by extension
   - [ ] Display configuration parameters
   - [ ] Show required dependencies

4. **Extension Upgrade**
   - [ ] Select target version
   - [ ] Preview upgrade SQL
   - [ ] Handle upgrade errors

5. **Extension Removal**
   - [ ] Confirm before uninstall
   - [ ] CASCADE option for dependent objects
   - [ ] Handle removal errors

## MCP Testing Instructions

### Tauri MCP Testing

```typescript
// List extensions
await mcp___hypothesi_tauri_mcp_server__ipc_execute_command({
	command: 'get_extensions',
	args: { connId: 'test-conn' }
});

// Install extension
await mcp___hypothesi_tauri_mcp_server__ipc_execute_command({
	command: 'install_extension',
	args: {
		connId: 'test-conn',
		options: {
			name: 'uuid-ossp',
			schema: 'public',
			cascade: true
		}
	}
});

// Get extension detail
await mcp___hypothesi_tauri_mcp_server__ipc_execute_command({
	command: 'get_extension_detail',
	args: {
		connId: 'test-conn',
		extensionName: 'uuid-ossp'
	}
});
```

### Playwright MCP Testing

```typescript
// Navigate to extensions
await mcp__playwright__browser_navigate({
	url: 'http://localhost:1420/extensions'
});

// Search for extension
await mcp__playwright__browser_type({
	element: 'Search input',
	ref: 'input[placeholder*="Search"]',
	text: 'uuid'
});

// Click install button
await mcp__playwright__browser_click({
	element: 'Install button',
	ref: 'button:has-text("Install"):first'
});

// Take screenshot of install dialog
await mcp__playwright__browser_take_screenshot({
	filename: 'extension-install-dialog.png'
});
```
