# Feature 22: Role Management

## Overview

Role Management provides a comprehensive interface for managing PostgreSQL roles (users and groups), including creating, editing, and deleting roles, managing role memberships, and viewing/modifying object privileges.

## Goals

- List all roles with their attributes
- Create and edit roles with all PostgreSQL options
- Manage role memberships (role hierarchy)
- Display and modify object privileges
- Generate SQL for role operations
- Support password management

## Dependencies

- Feature 07: Connection Pool Management
- Feature 10: Schema Cache (for object lists)

## Technical Specification

### 22.1 Role Data Models

```typescript
// src/lib/types/roles.ts

export interface Role {
	oid: number;
	name: string;
	isSuperuser: boolean;
	canLogin: boolean;
	canCreateDb: boolean;
	canCreateRole: boolean;
	inheritPrivileges: boolean;
	isReplication: boolean;
	bypassRls: boolean;
	connectionLimit: number; // -1 = unlimited
	validUntil: Date | null;
	config: RoleConfig[];
	memberOf: string[];
	members: string[];
	comment: string | null;
}

export interface RoleConfig {
	name: string;
	value: string;
}

export interface RoleCreateOptions {
	name: string;
	password?: string;
	superuser: boolean;
	createdb: boolean;
	createrole: boolean;
	inherit: boolean;
	login: boolean;
	replication: boolean;
	bypassrls: boolean;
	connectionLimit: number;
	validUntil: Date | null;
	inRoles: string[];
	roles: string[];
	adminRoles: string[];
}

export interface RoleAlterOptions {
	name?: string;
	password?: string;
	superuser?: boolean;
	createdb?: boolean;
	createrole?: boolean;
	inherit?: boolean;
	login?: boolean;
	replication?: boolean;
	bypassrls?: boolean;
	connectionLimit?: number;
	validUntil?: Date | null;
}

export interface Privilege {
	grantee: string;
	objectType: PrivilegeObjectType;
	schema: string | null;
	objectName: string;
	privileges: PrivilegeType[];
	grantOption: boolean;
	grantor: string;
}

export type PrivilegeObjectType =
	| 'table'
	| 'view'
	| 'sequence'
	| 'function'
	| 'schema'
	| 'database'
	| 'tablespace'
	| 'type';

export type PrivilegeType =
	| 'SELECT'
	| 'INSERT'
	| 'UPDATE'
	| 'DELETE'
	| 'TRUNCATE'
	| 'REFERENCES'
	| 'TRIGGER'
	| 'USAGE'
	| 'CREATE'
	| 'CONNECT'
	| 'TEMPORARY'
	| 'EXECUTE'
	| 'ALL';

export interface DefaultPrivilege {
	role: string;
	schema: string | null;
	objectType: PrivilegeObjectType;
	grantee: string;
	privileges: PrivilegeType[];
}
```

### 22.2 Role Service (Rust)

```rust
// src-tauri/src/services/role.rs

use serde::{Deserialize, Serialize};
use tokio_postgres::Client;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Role {
    pub oid: i64,
    pub name: String,
    pub is_superuser: bool,
    pub can_login: bool,
    pub can_create_db: bool,
    pub can_create_role: bool,
    pub inherit_privileges: bool,
    pub is_replication: bool,
    pub bypass_rls: bool,
    pub connection_limit: i32,
    pub valid_until: Option<DateTime<Utc>>,
    pub config: Vec<RoleConfig>,
    pub member_of: Vec<String>,
    pub members: Vec<String>,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleConfig {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleCreateOptions {
    pub name: String,
    pub password: Option<String>,
    pub superuser: bool,
    pub createdb: bool,
    pub createrole: bool,
    pub inherit: bool,
    pub login: bool,
    pub replication: bool,
    pub bypassrls: bool,
    pub connection_limit: i32,
    pub valid_until: Option<DateTime<Utc>>,
    pub in_roles: Vec<String>,
    pub roles: Vec<String>,
    pub admin_roles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleAlterOptions {
    pub name: Option<String>,
    pub password: Option<String>,
    pub superuser: Option<bool>,
    pub createdb: Option<bool>,
    pub createrole: Option<bool>,
    pub inherit: Option<bool>,
    pub login: Option<bool>,
    pub replication: Option<bool>,
    pub bypassrls: Option<bool>,
    pub connection_limit: Option<i32>,
    pub valid_until: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Privilege {
    pub grantee: String,
    pub object_type: String,
    pub schema: Option<String>,
    pub object_name: String,
    pub privileges: Vec<String>,
    pub grant_option: bool,
    pub grantor: String,
}

pub struct RoleService;

impl RoleService {
    /// Get all roles
    pub async fn get_roles(client: &Client) -> Result<Vec<Role>, RoleError> {
        let rows = client
            .query(
                r#"
                SELECT
                    r.oid,
                    r.rolname AS name,
                    r.rolsuper AS is_superuser,
                    r.rolcanlogin AS can_login,
                    r.rolcreatedb AS can_create_db,
                    r.rolcreaterole AS can_create_role,
                    r.rolinherit AS inherit_privileges,
                    r.rolreplication AS is_replication,
                    r.rolbypassrls AS bypass_rls,
                    r.rolconnlimit AS connection_limit,
                    r.rolvaliduntil AS valid_until,
                    r.rolconfig AS config,
                    COALESCE(
                        (SELECT array_agg(g.rolname ORDER BY g.rolname)
                         FROM pg_roles g
                         JOIN pg_auth_members m ON m.roleid = g.oid
                         WHERE m.member = r.oid),
                        ARRAY[]::text[]
                    ) AS member_of,
                    COALESCE(
                        (SELECT array_agg(m.rolname ORDER BY m.rolname)
                         FROM pg_roles m
                         JOIN pg_auth_members am ON am.member = m.oid
                         WHERE am.roleid = r.oid),
                        ARRAY[]::text[]
                    ) AS members,
                    COALESCE(d.description, '') AS comment
                FROM pg_roles r
                LEFT JOIN pg_shdescription d ON d.objoid = r.oid
                WHERE r.rolname NOT LIKE 'pg_%'
                ORDER BY r.rolname
                "#,
                &[],
            )
            .await?;

        let roles: Vec<Role> = rows
            .iter()
            .map(|row| {
                let config_arr: Option<Vec<String>> = row.get("config");
                let config = config_arr
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|c| {
                        let parts: Vec<&str> = c.splitn(2, '=').collect();
                        if parts.len() == 2 {
                            Some(RoleConfig {
                                name: parts[0].to_string(),
                                value: parts[1].to_string(),
                            })
                        } else {
                            None
                        }
                    })
                    .collect();

                Role {
                    oid: row.get::<_, i64>("oid"),
                    name: row.get("name"),
                    is_superuser: row.get("is_superuser"),
                    can_login: row.get("can_login"),
                    can_create_db: row.get("can_create_db"),
                    can_create_role: row.get("can_create_role"),
                    inherit_privileges: row.get("inherit_privileges"),
                    is_replication: row.get("is_replication"),
                    bypass_rls: row.get("bypass_rls"),
                    connection_limit: row.get("connection_limit"),
                    valid_until: row.get("valid_until"),
                    config,
                    member_of: row.get("member_of"),
                    members: row.get("members"),
                    comment: {
                        let c: String = row.get("comment");
                        if c.is_empty() { None } else { Some(c) }
                    },
                }
            })
            .collect();

        Ok(roles)
    }

    /// Create a new role
    pub async fn create_role(
        client: &Client,
        options: &RoleCreateOptions,
    ) -> Result<(), RoleError> {
        let sql = Self::build_create_role_sql(options)?;
        client.execute(&sql, &[]).await?;
        Ok(())
    }

    /// Build CREATE ROLE SQL
    pub fn build_create_role_sql(options: &RoleCreateOptions) -> Result<String, RoleError> {
        let mut parts = vec![format!("CREATE ROLE {}", Self::quote_ident(&options.name))];
        let mut with_opts = Vec::new();

        if options.superuser {
            with_opts.push("SUPERUSER");
        } else {
            with_opts.push("NOSUPERUSER");
        }

        if options.createdb {
            with_opts.push("CREATEDB");
        } else {
            with_opts.push("NOCREATEDB");
        }

        if options.createrole {
            with_opts.push("CREATEROLE");
        } else {
            with_opts.push("NOCREATEROLE");
        }

        if options.inherit {
            with_opts.push("INHERIT");
        } else {
            with_opts.push("NOINHERIT");
        }

        if options.login {
            with_opts.push("LOGIN");
        } else {
            with_opts.push("NOLOGIN");
        }

        if options.replication {
            with_opts.push("REPLICATION");
        } else {
            with_opts.push("NOREPLICATION");
        }

        if options.bypassrls {
            with_opts.push("BYPASSRLS");
        } else {
            with_opts.push("NOBYPASSRLS");
        }

        if options.connection_limit >= 0 {
            with_opts.push(&format!("CONNECTION LIMIT {}", options.connection_limit));
        }

        if let Some(ref password) = options.password {
            // In production, use prepared statements for passwords
            with_opts.push(&format!("PASSWORD '{}'", Self::escape_string(password)));
        }

        if let Some(valid) = options.valid_until {
            with_opts.push(&format!("VALID UNTIL '{}'", valid.format("%Y-%m-%d %H:%M:%S %z")));
        }

        if !with_opts.is_empty() {
            parts.push(format!("WITH {}", with_opts.join(" ")));
        }

        if !options.in_roles.is_empty() {
            let roles: Vec<String> = options.in_roles.iter()
                .map(|r| Self::quote_ident(r))
                .collect();
            parts.push(format!("IN ROLE {}", roles.join(", ")));
        }

        if !options.roles.is_empty() {
            let roles: Vec<String> = options.roles.iter()
                .map(|r| Self::quote_ident(r))
                .collect();
            parts.push(format!("ROLE {}", roles.join(", ")));
        }

        if !options.admin_roles.is_empty() {
            let roles: Vec<String> = options.admin_roles.iter()
                .map(|r| Self::quote_ident(r))
                .collect();
            parts.push(format!("ADMIN {}", roles.join(", ")));
        }

        Ok(parts.join(" "))
    }

    /// Alter an existing role
    pub async fn alter_role(
        client: &Client,
        role_name: &str,
        options: &RoleAlterOptions,
    ) -> Result<(), RoleError> {
        let sql = Self::build_alter_role_sql(role_name, options)?;
        client.execute(&sql, &[]).await?;
        Ok(())
    }

    /// Build ALTER ROLE SQL
    pub fn build_alter_role_sql(
        role_name: &str,
        options: &RoleAlterOptions,
    ) -> Result<String, RoleError> {
        let mut parts = vec![format!("ALTER ROLE {}", Self::quote_ident(role_name))];
        let mut with_opts = Vec::new();

        if let Some(superuser) = options.superuser {
            with_opts.push(if superuser { "SUPERUSER" } else { "NOSUPERUSER" });
        }

        if let Some(createdb) = options.createdb {
            with_opts.push(if createdb { "CREATEDB" } else { "NOCREATEDB" });
        }

        if let Some(createrole) = options.createrole {
            with_opts.push(if createrole { "CREATEROLE" } else { "NOCREATEROLE" });
        }

        if let Some(inherit) = options.inherit {
            with_opts.push(if inherit { "INHERIT" } else { "NOINHERIT" });
        }

        if let Some(login) = options.login {
            with_opts.push(if login { "LOGIN" } else { "NOLOGIN" });
        }

        if let Some(replication) = options.replication {
            with_opts.push(if replication { "REPLICATION" } else { "NOREPLICATION" });
        }

        if let Some(bypassrls) = options.bypassrls {
            with_opts.push(if bypassrls { "BYPASSRLS" } else { "NOBYPASSRLS" });
        }

        if let Some(limit) = options.connection_limit {
            with_opts.push(&format!("CONNECTION LIMIT {}", limit));
        }

        if let Some(ref password) = options.password {
            with_opts.push(&format!("PASSWORD '{}'", Self::escape_string(password)));
        }

        if let Some(valid) = options.valid_until {
            with_opts.push(&format!("VALID UNTIL '{}'", valid.format("%Y-%m-%d %H:%M:%S %z")));
        }

        if !with_opts.is_empty() {
            parts.push(format!("WITH {}", with_opts.join(" ")));
        }

        // Rename must be separate statement
        if let Some(ref new_name) = options.name {
            // Would need to execute rename separately
            return Err(RoleError::InvalidOperation(
                "Rename must be done separately".to_string(),
            ));
        }

        Ok(parts.join(" "))
    }

    /// Drop a role
    pub async fn drop_role(client: &Client, role_name: &str) -> Result<(), RoleError> {
        let sql = format!("DROP ROLE {}", Self::quote_ident(role_name));
        client.execute(&sql, &[]).await?;
        Ok(())
    }

    /// Add role to another role (grant membership)
    pub async fn grant_role(
        client: &Client,
        role: &str,
        member: &str,
        with_admin: bool,
    ) -> Result<(), RoleError> {
        let sql = if with_admin {
            format!(
                "GRANT {} TO {} WITH ADMIN OPTION",
                Self::quote_ident(role),
                Self::quote_ident(member)
            )
        } else {
            format!(
                "GRANT {} TO {}",
                Self::quote_ident(role),
                Self::quote_ident(member)
            )
        };
        client.execute(&sql, &[]).await?;
        Ok(())
    }

    /// Remove role from another role (revoke membership)
    pub async fn revoke_role(
        client: &Client,
        role: &str,
        member: &str,
    ) -> Result<(), RoleError> {
        let sql = format!(
            "REVOKE {} FROM {}",
            Self::quote_ident(role),
            Self::quote_ident(member)
        );
        client.execute(&sql, &[]).await?;
        Ok(())
    }

    /// Get privileges for a role
    pub async fn get_privileges(
        client: &Client,
        role_name: &str,
    ) -> Result<Vec<Privilege>, RoleError> {
        // Query table privileges
        let rows = client
            .query(
                r#"
                SELECT
                    grantee,
                    'table' AS object_type,
                    table_schema AS schema,
                    table_name AS object_name,
                    array_agg(privilege_type ORDER BY privilege_type) AS privileges,
                    bool_or(is_grantable = 'YES') AS grant_option,
                    grantor
                FROM information_schema.table_privileges
                WHERE grantee = $1
                GROUP BY grantee, table_schema, table_name, grantor
                "#,
                &[&role_name],
            )
            .await?;

        let privileges: Vec<Privilege> = rows
            .iter()
            .map(|row| Privilege {
                grantee: row.get("grantee"),
                object_type: row.get("object_type"),
                schema: row.get("schema"),
                object_name: row.get("object_name"),
                privileges: row.get("privileges"),
                grant_option: row.get("grant_option"),
                grantor: row.get("grantor"),
            })
            .collect();

        Ok(privileges)
    }

    /// Grant privilege on object
    pub async fn grant_privilege(
        client: &Client,
        privilege: &str,
        object_type: &str,
        schema: Option<&str>,
        object_name: &str,
        role: &str,
        with_grant_option: bool,
    ) -> Result<(), RoleError> {
        let object = if let Some(s) = schema {
            format!("{}.{}", Self::quote_ident(s), Self::quote_ident(object_name))
        } else {
            Self::quote_ident(object_name)
        };

        let mut sql = format!(
            "GRANT {} ON {} {} TO {}",
            privilege,
            object_type.to_uppercase(),
            object,
            Self::quote_ident(role)
        );

        if with_grant_option {
            sql.push_str(" WITH GRANT OPTION");
        }

        client.execute(&sql, &[]).await?;
        Ok(())
    }

    /// Revoke privilege on object
    pub async fn revoke_privilege(
        client: &Client,
        privilege: &str,
        object_type: &str,
        schema: Option<&str>,
        object_name: &str,
        role: &str,
    ) -> Result<(), RoleError> {
        let object = if let Some(s) = schema {
            format!("{}.{}", Self::quote_ident(s), Self::quote_ident(object_name))
        } else {
            Self::quote_ident(object_name)
        };

        let sql = format!(
            "REVOKE {} ON {} {} FROM {}",
            privilege,
            object_type.to_uppercase(),
            object,
            Self::quote_ident(role)
        );

        client.execute(&sql, &[]).await?;
        Ok(())
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
pub enum RoleError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] tokio_postgres::Error),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    #[error("Role not found: {0}")]
    NotFound(String),
}
```

### 22.3 Tauri Commands

```rust
// src-tauri/src/commands/role.rs

use tauri::State;
use crate::services::role::{RoleService, Role, RoleCreateOptions, RoleAlterOptions, Privilege};
use crate::state::AppState;
use crate::error::Error;

#[tauri::command]
pub async fn get_roles(
    state: State<'_, AppState>,
    conn_id: String,
) -> Result<Vec<Role>, Error> {
    let pool = state.get_connection(&conn_id)?;
    let client = pool.get().await?;
    let roles = RoleService::get_roles(&client).await?;
    Ok(roles)
}

#[tauri::command]
pub async fn create_role(
    state: State<'_, AppState>,
    conn_id: String,
    options: RoleCreateOptions,
) -> Result<(), Error> {
    let pool = state.get_connection(&conn_id)?;
    let client = pool.get().await?;
    RoleService::create_role(&client, &options).await?;
    Ok(())
}

#[tauri::command]
pub async fn alter_role(
    state: State<'_, AppState>,
    conn_id: String,
    role_name: String,
    options: RoleAlterOptions,
) -> Result<(), Error> {
    let pool = state.get_connection(&conn_id)?;
    let client = pool.get().await?;
    RoleService::alter_role(&client, &role_name, &options).await?;
    Ok(())
}

#[tauri::command]
pub async fn drop_role(
    state: State<'_, AppState>,
    conn_id: String,
    role_name: String,
) -> Result<(), Error> {
    let pool = state.get_connection(&conn_id)?;
    let client = pool.get().await?;
    RoleService::drop_role(&client, &role_name).await?;
    Ok(())
}

#[tauri::command]
pub async fn grant_role_membership(
    state: State<'_, AppState>,
    conn_id: String,
    role: String,
    member: String,
    with_admin: bool,
) -> Result<(), Error> {
    let pool = state.get_connection(&conn_id)?;
    let client = pool.get().await?;
    RoleService::grant_role(&client, &role, &member, with_admin).await?;
    Ok(())
}

#[tauri::command]
pub async fn revoke_role_membership(
    state: State<'_, AppState>,
    conn_id: String,
    role: String,
    member: String,
) -> Result<(), Error> {
    let pool = state.get_connection(&conn_id)?;
    let client = pool.get().await?;
    RoleService::revoke_role(&client, &role, &member).await?;
    Ok(())
}

#[tauri::command]
pub async fn get_role_privileges(
    state: State<'_, AppState>,
    conn_id: String,
    role_name: String,
) -> Result<Vec<Privilege>, Error> {
    let pool = state.get_connection(&conn_id)?;
    let client = pool.get().await?;
    let privileges = RoleService::get_privileges(&client, &role_name).await?;
    Ok(privileges)
}

#[tauri::command]
pub fn generate_create_role_sql(options: RoleCreateOptions) -> Result<String, Error> {
    let sql = RoleService::build_create_role_sql(&options)?;
    Ok(sql)
}

#[tauri::command]
pub fn generate_alter_role_sql(
    role_name: String,
    options: RoleAlterOptions,
) -> Result<String, Error> {
    let sql = RoleService::build_alter_role_sql(&role_name, &options)?;
    Ok(sql)
}
```

### 22.4 Role Store (Svelte)

```typescript
// src/lib/stores/roleStore.svelte.ts

import { invoke } from '@tauri-apps/api/core';
import type { Role, RoleCreateOptions, RoleAlterOptions, Privilege } from '$lib/types/roles';

interface RoleState {
	roles: Role[];
	selectedRole: Role | null;
	privileges: Privilege[];
	loading: boolean;
	error: string | null;
}

export function createRoleStore() {
	let state = $state<RoleState>({
		roles: [],
		selectedRole: null,
		privileges: [],
		loading: false,
		error: null
	});

	async function loadRoles(connId: string) {
		state.loading = true;
		state.error = null;

		try {
			state.roles = await invoke<Role[]>('get_roles', { connId });
		} catch (err) {
			state.error = err instanceof Error ? err.message : String(err);
		} finally {
			state.loading = false;
		}
	}

	async function selectRole(connId: string, role: Role) {
		state.selectedRole = role;

		try {
			state.privileges = await invoke<Privilege[]>('get_role_privileges', {
				connId,
				roleName: role.name
			});
		} catch (err) {
			state.error = err instanceof Error ? err.message : String(err);
		}
	}

	async function createRole(connId: string, options: RoleCreateOptions) {
		try {
			await invoke('create_role', { connId, options });
			await loadRoles(connId);
		} catch (err) {
			throw err;
		}
	}

	async function alterRole(connId: string, roleName: string, options: RoleAlterOptions) {
		try {
			await invoke('alter_role', { connId, roleName, options });
			await loadRoles(connId);
		} catch (err) {
			throw err;
		}
	}

	async function dropRole(connId: string, roleName: string) {
		try {
			await invoke('drop_role', { connId, roleName });
			if (state.selectedRole?.name === roleName) {
				state.selectedRole = null;
				state.privileges = [];
			}
			await loadRoles(connId);
		} catch (err) {
			throw err;
		}
	}

	async function grantMembership(connId: string, role: string, member: string, withAdmin: boolean) {
		try {
			await invoke('grant_role_membership', { connId, role, member, withAdmin });
			await loadRoles(connId);
		} catch (err) {
			throw err;
		}
	}

	async function revokeMembership(connId: string, role: string, member: string) {
		try {
			await invoke('revoke_role_membership', { connId, role, member });
			await loadRoles(connId);
		} catch (err) {
			throw err;
		}
	}

	async function generateCreateSql(options: RoleCreateOptions): Promise<string> {
		return await invoke<string>('generate_create_role_sql', { options });
	}

	async function generateAlterSql(roleName: string, options: RoleAlterOptions): Promise<string> {
		return await invoke<string>('generate_alter_role_sql', { roleName, options });
	}

	function clearSelection() {
		state.selectedRole = null;
		state.privileges = [];
	}

	return {
		get roles() {
			return state.roles;
		},
		get selectedRole() {
			return state.selectedRole;
		},
		get privileges() {
			return state.privileges;
		},
		get loading() {
			return state.loading;
		},
		get error() {
			return state.error;
		},

		loadRoles,
		selectRole,
		createRole,
		alterRole,
		dropRole,
		grantMembership,
		revokeMembership,
		generateCreateSql,
		generateAlterSql,
		clearSelection
	};
}

export const roleStore = createRoleStore();
```

### 22.5 Role List Component

```svelte
<!-- src/lib/components/roles/RoleList.svelte -->
<script lang="ts">
	import type { Role } from '$lib/types/roles';
	import { roleStore } from '$lib/stores/roleStore.svelte';

	interface Props {
		connId: string;
		onEdit: (role: Role) => void;
		onCreate: () => void;
	}

	let { connId, onEdit, onCreate }: Props = $props();

	let filter = $state('');
	let showLoginOnly = $state(false);

	const filteredRoles = $derived(
		roleStore.roles.filter((r) => {
			if (filter && !r.name.toLowerCase().includes(filter.toLowerCase())) {
				return false;
			}
			if (showLoginOnly && !r.canLogin) {
				return false;
			}
			return true;
		})
	);

	function getBadges(role: Role): Array<{ label: string; color: string }> {
		const badges = [];

		if (role.isSuperuser) {
			badges.push({
				label: 'Superuser',
				color: 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400'
			});
		}
		if (role.canLogin) {
			badges.push({
				label: 'Login',
				color: 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400'
			});
		}
		if (role.canCreateDb) {
			badges.push({
				label: 'Create DB',
				color: 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400'
			});
		}
		if (role.canCreateRole) {
			badges.push({
				label: 'Create Role',
				color: 'bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-400'
			});
		}
		if (role.isReplication) {
			badges.push({
				label: 'Replication',
				color: 'bg-orange-100 text-orange-700 dark:bg-orange-900/30 dark:text-orange-400'
			});
		}

		return badges;
	}

	function handleSelect(role: Role) {
		roleStore.selectRole(connId, role);
	}
</script>

<div class="flex flex-col h-full">
	<!-- Toolbar -->
	<div class="flex items-center gap-2 p-4 border-b border-gray-200 dark:border-gray-700">
		<input
			type="text"
			bind:value={filter}
			placeholder="Filter roles..."
			class="flex-1 px-3 py-2 border border-gray-300 dark:border-gray-600 rounded
             bg-white dark:bg-gray-700 text-sm"
		/>
		<label class="flex items-center gap-2 text-sm">
			<input type="checkbox" bind:checked={showLoginOnly} class="rounded border-gray-300" />
			Login only
		</label>
		<button
			onclick={onCreate}
			class="px-3 py-2 text-sm bg-blue-600 text-white rounded hover:bg-blue-700"
		>
			+ New Role
		</button>
	</div>

	<!-- Role List -->
	<div class="flex-1 overflow-auto">
		<table class="min-w-full divide-y divide-gray-200 dark:divide-gray-700">
			<thead class="bg-gray-50 dark:bg-gray-900/50 sticky top-0">
				<tr>
					<th
						class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider"
					>
						Role
					</th>
					<th
						class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider"
					>
						Attributes
					</th>
					<th
						class="px-4 py-3 text-center text-xs font-medium text-gray-500 uppercase tracking-wider"
					>
						Connections
					</th>
					<th
						class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider"
					>
						Member Of
					</th>
					<th
						class="px-4 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider"
					>
						Actions
					</th>
				</tr>
			</thead>
			<tbody class="divide-y divide-gray-200 dark:divide-gray-700">
				{#each filteredRoles as role (role.oid)}
					<tr
						class="hover:bg-gray-50 dark:hover:bg-gray-700/50 cursor-pointer
                   {roleStore.selectedRole?.oid === role.oid
							? 'bg-blue-50 dark:bg-blue-900/20'
							: ''}"
						onclick={() => handleSelect(role)}
					>
						<td class="px-4 py-3">
							<div class="font-medium">{role.name}</div>
							{#if role.comment}
								<div class="text-xs text-gray-500 truncate max-w-xs">{role.comment}</div>
							{/if}
						</td>
						<td class="px-4 py-3">
							<div class="flex flex-wrap gap-1">
								{#each getBadges(role) as badge}
									<span class="inline-flex px-1.5 py-0.5 rounded text-xs font-medium {badge.color}">
										{badge.label}
									</span>
								{/each}
							</div>
						</td>
						<td class="px-4 py-3 text-center text-sm">
							{role.connectionLimit === -1 ? '∞' : role.connectionLimit}
						</td>
						<td class="px-4 py-3 text-sm">
							{#if role.memberOf.length > 0}
								<div class="flex flex-wrap gap-1">
									{#each role.memberOf.slice(0, 3) as memberRole}
										<span class="px-1.5 py-0.5 bg-gray-100 dark:bg-gray-700 rounded text-xs">
											{memberRole}
										</span>
									{/each}
									{#if role.memberOf.length > 3}
										<span class="text-xs text-gray-500">+{role.memberOf.length - 3}</span>
									{/if}
								</div>
							{:else}
								<span class="text-gray-400">-</span>
							{/if}
						</td>
						<td class="px-4 py-3 text-right">
							<button
								onclick={(e) => {
									e.stopPropagation();
									onEdit(role);
								}}
								class="text-blue-600 hover:text-blue-700 dark:text-blue-400
                       dark:hover:text-blue-300 text-sm"
							>
								Edit
							</button>
						</td>
					</tr>
				{:else}
					<tr>
						<td colspan="5" class="px-4 py-8 text-center text-gray-500">
							{filter ? 'No roles match the filter' : 'No roles found'}
						</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>
</div>
```

### 22.6 Role Editor Dialog

```svelte
<!-- src/lib/components/roles/RoleEditor.svelte -->
<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { Role, RoleCreateOptions, RoleAlterOptions } from '$lib/types/roles';
	import { roleStore } from '$lib/stores/roleStore.svelte';

	interface Props {
		open: boolean;
		connId: string;
		role?: Role; // undefined for create, Role for edit
		availableRoles: string[];
	}

	let { open = $bindable(), connId, role, availableRoles }: Props = $props();

	const dispatch = createEventDispatcher<{
		save: void;
		cancel: void;
	}>();

	const isEdit = $derived(!!role);

	// Form state
	let name = $state(role?.name ?? '');
	let password = $state('');
	let confirmPassword = $state('');
	let superuser = $state(role?.isSuperuser ?? false);
	let createdb = $state(role?.canCreateDb ?? false);
	let createrole = $state(role?.canCreateRole ?? false);
	let inherit = $state(role?.inheritPrivileges ?? true);
	let login = $state(role?.canLogin ?? true);
	let replication = $state(role?.isReplication ?? false);
	let bypassrls = $state(role?.bypassRls ?? false);
	let connectionLimit = $state(role?.connectionLimit ?? -1);
	let validUntil = $state<string>(
		role?.validUntil ? new Date(role.validUntil).toISOString().split('T')[0] : ''
	);
	let memberOf = $state<string[]>(role?.memberOf ?? []);

	let showSql = $state(false);
	let generatedSql = $state('');
	let saving = $state(false);
	let error = $state<string | null>(null);

	const passwordError = $derived(
		password && confirmPassword && password !== confirmPassword ? 'Passwords do not match' : null
	);

	const otherRoles = $derived(availableRoles.filter((r) => r !== role?.name));

	async function generateSql() {
		if (isEdit) {
			const options: RoleAlterOptions = {
				superuser,
				createdb,
				createrole,
				inherit,
				login,
				replication,
				bypassrls,
				connectionLimit,
				validUntil: validUntil ? new Date(validUntil) : undefined,
				password: password || undefined
			};
			generatedSql = await roleStore.generateAlterSql(role!.name, options);
		} else {
			const options: RoleCreateOptions = {
				name,
				password: password || undefined,
				superuser,
				createdb,
				createrole,
				inherit,
				login,
				replication,
				bypassrls,
				connectionLimit,
				validUntil: validUntil ? new Date(validUntil) : null,
				inRoles: memberOf,
				roles: [],
				adminRoles: []
			};
			generatedSql = await roleStore.generateCreateSql(options);
		}
		showSql = true;
	}

	async function handleSave() {
		if (passwordError) return;

		saving = true;
		error = null;

		try {
			if (isEdit) {
				const options: RoleAlterOptions = {
					superuser,
					createdb,
					createrole,
					inherit,
					login,
					replication,
					bypassrls,
					connectionLimit,
					validUntil: validUntil ? new Date(validUntil) : undefined,
					password: password || undefined
				};
				await roleStore.alterRole(connId, role!.name, options);

				// Handle membership changes
				const originalMemberOf = new Set(role!.memberOf);
				const newMemberOf = new Set(memberOf);

				// Revoke removed memberships
				for (const r of originalMemberOf) {
					if (!newMemberOf.has(r)) {
						await roleStore.revokeMembership(connId, r, role!.name);
					}
				}

				// Grant new memberships
				for (const r of newMemberOf) {
					if (!originalMemberOf.has(r)) {
						await roleStore.grantMembership(connId, r, role!.name, false);
					}
				}
			} else {
				const options: RoleCreateOptions = {
					name,
					password: password || undefined,
					superuser,
					createdb,
					createrole,
					inherit,
					login,
					replication,
					bypassrls,
					connectionLimit,
					validUntil: validUntil ? new Date(validUntil) : null,
					inRoles: memberOf,
					roles: [],
					adminRoles: []
				};
				await roleStore.createRole(connId, options);
			}

			dispatch('save');
			open = false;
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		} finally {
			saving = false;
		}
	}

	function handleCancel() {
		dispatch('cancel');
		open = false;
	}

	function toggleMembership(roleName: string) {
		if (memberOf.includes(roleName)) {
			memberOf = memberOf.filter((r) => r !== roleName);
		} else {
			memberOf = [...memberOf, roleName];
		}
	}
</script>

{#if open}
	<div
		class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
		role="dialog"
		aria-modal="true"
	>
		<div
			class="bg-white dark:bg-gray-800 rounded-lg shadow-xl w-[600px] max-h-[80vh] overflow-hidden"
		>
			<!-- Header -->
			<div class="px-4 py-3 border-b border-gray-200 dark:border-gray-700">
				<h2 class="text-lg font-semibold">
					{isEdit ? `Edit Role: ${role.name}` : 'Create New Role'}
				</h2>
			</div>

			<!-- Body -->
			<div class="p-4 space-y-4 overflow-y-auto max-h-[60vh]">
				{#if error}
					<div
						class="p-3 bg-red-50 dark:bg-red-900/20 border border-red-200
                      dark:border-red-800 rounded text-sm text-red-700 dark:text-red-400"
					>
						{error}
					</div>
				{/if}

				<!-- Name -->
				<div>
					<label class="block text-sm font-medium mb-1">Role Name</label>
					<input
						type="text"
						bind:value={name}
						disabled={isEdit}
						class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded
                   bg-white dark:bg-gray-700 text-sm disabled:opacity-50"
					/>
				</div>

				<!-- Password -->
				<div class="grid grid-cols-2 gap-4">
					<div>
						<label class="block text-sm font-medium mb-1">
							{isEdit ? 'New Password' : 'Password'}
						</label>
						<input
							type="password"
							bind:value={password}
							placeholder={isEdit ? 'Leave empty to keep current' : ''}
							class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded
                     bg-white dark:bg-gray-700 text-sm"
						/>
					</div>
					<div>
						<label class="block text-sm font-medium mb-1">Confirm Password</label>
						<input
							type="password"
							bind:value={confirmPassword}
							class="w-full px-3 py-2 border rounded text-sm
                     {passwordError
								? 'border-red-500 dark:border-red-500'
								: 'border-gray-300 dark:border-gray-600'}
                     bg-white dark:bg-gray-700"
						/>
						{#if passwordError}
							<p class="text-xs text-red-500 mt-1">{passwordError}</p>
						{/if}
					</div>
				</div>

				<!-- Privileges -->
				<div>
					<label class="block text-sm font-medium mb-2">Privileges</label>
					<div class="grid grid-cols-2 gap-3">
						<label class="flex items-center gap-2 cursor-pointer">
							<input type="checkbox" bind:checked={login} class="rounded" />
							<span class="text-sm">Can login</span>
						</label>
						<label class="flex items-center gap-2 cursor-pointer">
							<input type="checkbox" bind:checked={superuser} class="rounded" />
							<span class="text-sm">Superuser</span>
						</label>
						<label class="flex items-center gap-2 cursor-pointer">
							<input type="checkbox" bind:checked={createdb} class="rounded" />
							<span class="text-sm">Create databases</span>
						</label>
						<label class="flex items-center gap-2 cursor-pointer">
							<input type="checkbox" bind:checked={createrole} class="rounded" />
							<span class="text-sm">Create roles</span>
						</label>
						<label class="flex items-center gap-2 cursor-pointer">
							<input type="checkbox" bind:checked={inherit} class="rounded" />
							<span class="text-sm">Inherit privileges</span>
						</label>
						<label class="flex items-center gap-2 cursor-pointer">
							<input type="checkbox" bind:checked={replication} class="rounded" />
							<span class="text-sm">Replication</span>
						</label>
						<label class="flex items-center gap-2 cursor-pointer">
							<input type="checkbox" bind:checked={bypassrls} class="rounded" />
							<span class="text-sm">Bypass RLS</span>
						</label>
					</div>
				</div>

				<!-- Limits -->
				<div class="grid grid-cols-2 gap-4">
					<div>
						<label class="block text-sm font-medium mb-1">Connection Limit</label>
						<input
							type="number"
							bind:value={connectionLimit}
							min="-1"
							class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded
                     bg-white dark:bg-gray-700 text-sm"
						/>
						<p class="text-xs text-gray-500 mt-1">-1 = unlimited</p>
					</div>
					<div>
						<label class="block text-sm font-medium mb-1">Valid Until</label>
						<input
							type="date"
							bind:value={validUntil}
							class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded
                     bg-white dark:bg-gray-700 text-sm"
						/>
						<p class="text-xs text-gray-500 mt-1">Leave empty for no expiration</p>
					</div>
				</div>

				<!-- Membership -->
				<div>
					<label class="block text-sm font-medium mb-2">Member Of</label>
					<div
						class="max-h-32 overflow-y-auto border border-gray-200 dark:border-gray-700
                      rounded p-2 space-y-1"
					>
						{#each otherRoles as r}
							<label class="flex items-center gap-2 cursor-pointer text-sm">
								<input
									type="checkbox"
									checked={memberOf.includes(r)}
									onchange={() => toggleMembership(r)}
									class="rounded"
								/>
								<span>{r}</span>
							</label>
						{:else}
							<p class="text-sm text-gray-500 py-2 text-center">No other roles available</p>
						{/each}
					</div>
				</div>

				<!-- SQL Preview -->
				{#if showSql}
					<div>
						<label class="block text-sm font-medium mb-2">Generated SQL</label>
						<pre class="p-3 bg-gray-100 dark:bg-gray-900 rounded font-mono text-xs overflow-auto">
{generatedSql}
            </pre>
					</div>
				{/if}
			</div>

			<!-- Footer -->
			<div class="px-4 py-3 border-t border-gray-200 dark:border-gray-700 flex justify-between">
				<button
					onclick={generateSql}
					class="px-3 py-2 text-sm text-gray-600 dark:text-gray-400
                 hover:text-gray-900 dark:hover:text-gray-100"
				>
					View SQL
				</button>
				<div class="flex gap-2">
					<button
						onclick={handleCancel}
						class="px-4 py-2 text-sm text-gray-700 dark:text-gray-300
                   hover:bg-gray-100 dark:hover:bg-gray-700 rounded"
					>
						Cancel
					</button>
					<button
						onclick={handleSave}
						disabled={saving || !!passwordError || (!isEdit && !name)}
						class="px-4 py-2 text-sm bg-blue-600 text-white rounded hover:bg-blue-700
                   disabled:opacity-50 disabled:cursor-not-allowed"
					>
						{saving ? 'Saving...' : isEdit ? 'Save Changes' : 'Create Role'}
					</button>
				</div>
			</div>
		</div>
	</div>
{/if}
```

## Acceptance Criteria

1. **Role Listing**
   - [ ] Display all roles with attributes
   - [ ] Show role badges (superuser, login, etc.)
   - [ ] Filter by name and login capability
   - [ ] Display role memberships

2. **Role Creation**
   - [ ] Create roles with all PostgreSQL options
   - [ ] Set password with confirmation
   - [ ] Configure all privilege flags
   - [ ] Set connection limit and expiration
   - [ ] Assign initial role memberships
   - [ ] Preview generated SQL

3. **Role Editing**
   - [ ] Modify all role attributes
   - [ ] Change password optionally
   - [ ] Add/remove role memberships
   - [ ] Preview ALTER statements

4. **Role Deletion**
   - [ ] Confirm before deleting
   - [ ] Handle dependent objects warning

5. **Privilege Management**
   - [ ] View privileges for selected role
   - [ ] Display object-level permissions

## MCP Testing Instructions

### Tauri MCP Testing

```typescript
// Load roles
await mcp___hypothesi_tauri_mcp_server__ipc_execute_command({
	command: 'get_roles',
	args: { connId: 'test-conn' }
});

// Create a new role
await mcp___hypothesi_tauri_mcp_server__ipc_execute_command({
	command: 'create_role',
	args: {
		connId: 'test-conn',
		options: {
			name: 'test_user',
			password: 'secure_password',
			superuser: false,
			createdb: false,
			createrole: false,
			inherit: true,
			login: true,
			replication: false,
			bypassrls: false,
			connectionLimit: 10,
			validUntil: null,
			inRoles: [],
			roles: [],
			adminRoles: []
		}
	}
});

// Grant role membership
await mcp___hypothesi_tauri_mcp_server__ipc_execute_command({
	command: 'grant_role_membership',
	args: {
		connId: 'test-conn',
		role: 'readonly',
		member: 'test_user',
		withAdmin: false
	}
});
```

### Playwright MCP Testing

```typescript
// Navigate to roles
await mcp__playwright__browser_navigate({
	url: 'http://localhost:1420/roles'
});

// Click create button
await mcp__playwright__browser_click({
	element: 'New Role button',
	ref: 'button:has-text("New Role")'
});

// Fill form
await mcp__playwright__browser_fill_form({
	fields: [
		{ name: 'Role Name', type: 'textbox', ref: 'input[name="name"]', value: 'test_user' },
		{
			name: 'Password',
			type: 'textbox',
			ref: 'input[type="password"]:first',
			value: 'password123'
		},
		{
			name: 'Confirm Password',
			type: 'textbox',
			ref: 'input[type="password"]:last',
			value: 'password123'
		}
	]
});

// Check login option
await mcp__playwright__browser_click({
	element: 'Can login checkbox',
	ref: 'input[type="checkbox"]:near(:text("Can login"))'
});

// Take screenshot
await mcp__playwright__browser_take_screenshot({
	filename: 'role-editor.png'
});
```
