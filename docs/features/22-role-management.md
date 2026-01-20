# Feature 22: Role Management

## Overview

Role Management provides a comprehensive interface for managing PostgreSQL roles (users and groups), including creating, editing, and deleting roles, managing role memberships, and viewing/modifying object privileges. Built with GPUI for native performance and cross-platform support.

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

```rust
// src/models/role.rs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A PostgreSQL role (user or group)
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub connection_limit: i32, // -1 = unlimited
    pub valid_until: Option<DateTime<Utc>>,
    pub config: Vec<RoleConfig>,
    pub member_of: Vec<String>,
    pub members: Vec<String>,
    pub comment: Option<String>,
}

/// A role configuration parameter (SET variable)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleConfig {
    pub name: String,
    pub value: String,
}

/// Options for creating a new role
#[derive(Debug, Clone, Default)]
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
    pub connection_limit: i32, // -1 = unlimited
    pub valid_until: Option<DateTime<Utc>>,
    pub in_roles: Vec<String>,      // Roles this role is member of
    pub roles: Vec<String>,         // Roles that are members of this role
    pub admin_roles: Vec<String>,   // Roles with admin option on this role
}

impl RoleCreateOptions {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            inherit: true, // PostgreSQL default
            connection_limit: -1, // Unlimited
            ..Default::default()
        }
    }

    pub fn with_login(mut self) -> Self {
        self.login = true;
        self
    }

    pub fn with_password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }

    pub fn with_superuser(mut self) -> Self {
        self.superuser = true;
        self
    }

    pub fn in_role(mut self, role: impl Into<String>) -> Self {
        self.in_roles.push(role.into());
        self
    }
}

/// Options for altering an existing role
#[derive(Debug, Clone, Default)]
pub struct RoleAlterOptions {
    pub new_name: Option<String>,
    pub password: Option<String>,
    pub superuser: Option<bool>,
    pub createdb: Option<bool>,
    pub createrole: Option<bool>,
    pub inherit: Option<bool>,
    pub login: Option<bool>,
    pub replication: Option<bool>,
    pub bypassrls: Option<bool>,
    pub connection_limit: Option<i32>,
    pub valid_until: Option<Option<DateTime<Utc>>>, // Some(None) clears expiration
}

/// A privilege grant on a database object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Privilege {
    pub grantee: String,
    pub object_type: PrivilegeObjectType,
    pub schema: Option<String>,
    pub object_name: String,
    pub privileges: Vec<PrivilegeType>,
    pub grant_option: bool,
    pub grantor: String,
}

/// Types of database objects that can have privileges
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrivilegeObjectType {
    Table,
    View,
    Sequence,
    Function,
    Schema,
    Database,
    Tablespace,
    Type,
}

impl PrivilegeObjectType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Table => "TABLE",
            Self::View => "VIEW",
            Self::Sequence => "SEQUENCE",
            Self::Function => "FUNCTION",
            Self::Schema => "SCHEMA",
            Self::Database => "DATABASE",
            Self::Tablespace => "TABLESPACE",
            Self::Type => "TYPE",
        }
    }

    pub fn available_privileges(&self) -> &'static [PrivilegeType] {
        use PrivilegeType::*;
        match self {
            Self::Table | Self::View => &[Select, Insert, Update, Delete, Truncate, References, Trigger],
            Self::Sequence => &[Usage, Select, Update],
            Self::Function => &[Execute],
            Self::Schema => &[Usage, Create],
            Self::Database => &[Create, Connect, Temporary],
            Self::Tablespace => &[Create],
            Self::Type => &[Usage],
        }
    }
}

/// Types of privileges that can be granted
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PrivilegeType {
    Select,
    Insert,
    Update,
    Delete,
    Truncate,
    References,
    Trigger,
    Usage,
    Create,
    Connect,
    Temporary,
    Execute,
    All,
}

impl PrivilegeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Select => "SELECT",
            Self::Insert => "INSERT",
            Self::Update => "UPDATE",
            Self::Delete => "DELETE",
            Self::Truncate => "TRUNCATE",
            Self::References => "REFERENCES",
            Self::Trigger => "TRIGGER",
            Self::Usage => "USAGE",
            Self::Create => "CREATE",
            Self::Connect => "CONNECT",
            Self::Temporary => "TEMPORARY",
            Self::Execute => "EXECUTE",
            Self::All => "ALL",
        }
    }
}

/// Default privilege configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultPrivilege {
    pub role: String,
    pub schema: Option<String>,
    pub object_type: PrivilegeObjectType,
    pub grantee: String,
    pub privileges: Vec<PrivilegeType>,
}

/// Role membership with admin flag
#[derive(Debug, Clone)]
pub struct RoleMembership {
    pub role: String,
    pub member: String,
    pub admin_option: bool,
    pub grantor: String,
}
```

### 22.2 Role Service

```rust
// src/services/role.rs

use crate::models::role::{
    DefaultPrivilege, Privilege, PrivilegeObjectType, PrivilegeType,
    Role, RoleAlterOptions, RoleConfig, RoleCreateOptions, RoleMembership,
};
use chrono::{DateTime, Utc};
use deadpool_postgres::Pool;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RoleError {
    #[error("Database error: {0}")]
    Database(#[from] tokio_postgres::Error),

    #[error("Pool error: {0}")]
    Pool(#[from] deadpool_postgres::PoolError),

    #[error("Role not found: {0}")]
    NotFound(String),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Role has dependent objects")]
    HasDependentObjects,
}

pub struct RoleService;

impl RoleService {
    /// Get all roles (excluding system roles starting with pg_)
    pub async fn get_roles(pool: &Pool) -> Result<Vec<Role>, RoleError> {
        let client = pool.get().await?;

        let rows = client
            .query(
                r#"
                SELECT
                    r.oid::bigint,
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
                    d.description AS comment
                FROM pg_roles r
                LEFT JOIN pg_shdescription d ON d.objoid = r.oid
                WHERE r.rolname NOT LIKE 'pg_%'
                ORDER BY r.rolname
                "#,
                &[],
            )
            .await?;

        let roles = rows
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
                    oid: row.get("oid"),
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
                    comment: row.get("comment"),
                }
            })
            .collect();

        Ok(roles)
    }

    /// Get a single role by name
    pub async fn get_role(pool: &Pool, name: &str) -> Result<Role, RoleError> {
        let roles = Self::get_roles(pool).await?;
        roles
            .into_iter()
            .find(|r| r.name == name)
            .ok_or_else(|| RoleError::NotFound(name.to_string()))
    }

    /// Create a new role
    pub async fn create_role(pool: &Pool, options: &RoleCreateOptions) -> Result<(), RoleError> {
        let client = pool.get().await?;
        let sql = Self::build_create_role_sql(options);
        client.execute(&sql, &[]).await?;
        Ok(())
    }

    /// Build CREATE ROLE SQL statement
    pub fn build_create_role_sql(options: &RoleCreateOptions) -> String {
        let mut parts = vec![format!("CREATE ROLE {}", quote_ident(&options.name))];
        let mut with_opts = Vec::new();

        // Boolean options
        with_opts.push(if options.superuser { "SUPERUSER" } else { "NOSUPERUSER" });
        with_opts.push(if options.createdb { "CREATEDB" } else { "NOCREATEDB" });
        with_opts.push(if options.createrole { "CREATEROLE" } else { "NOCREATEROLE" });
        with_opts.push(if options.inherit { "INHERIT" } else { "NOINHERIT" });
        with_opts.push(if options.login { "LOGIN" } else { "NOLOGIN" });
        with_opts.push(if options.replication { "REPLICATION" } else { "NOREPLICATION" });
        with_opts.push(if options.bypassrls { "BYPASSRLS" } else { "NOBYPASSRLS" });

        // Connection limit
        if options.connection_limit >= 0 {
            with_opts.push(&format!("CONNECTION LIMIT {}", options.connection_limit));
        }

        // Password (use ENCRYPTED by default)
        if let Some(ref password) = options.password {
            with_opts.push(&format!("ENCRYPTED PASSWORD '{}'", escape_string(password)));
        }

        // Valid until
        if let Some(valid) = options.valid_until {
            with_opts.push(&format!(
                "VALID UNTIL '{}'",
                valid.format("%Y-%m-%d %H:%M:%S%:z")
            ));
        }

        if !with_opts.is_empty() {
            parts.push(format!("WITH {}", with_opts.join(" ")));
        }

        // IN ROLE clause
        if !options.in_roles.is_empty() {
            let roles: Vec<String> = options.in_roles.iter().map(|r| quote_ident(r)).collect();
            parts.push(format!("IN ROLE {}", roles.join(", ")));
        }

        // ROLE clause (members)
        if !options.roles.is_empty() {
            let roles: Vec<String> = options.roles.iter().map(|r| quote_ident(r)).collect();
            parts.push(format!("ROLE {}", roles.join(", ")));
        }

        // ADMIN clause (members with admin option)
        if !options.admin_roles.is_empty() {
            let roles: Vec<String> = options.admin_roles.iter().map(|r| quote_ident(r)).collect();
            parts.push(format!("ADMIN {}", roles.join(", ")));
        }

        parts.join(" ")
    }

    /// Alter an existing role
    pub async fn alter_role(
        pool: &Pool,
        role_name: &str,
        options: &RoleAlterOptions,
    ) -> Result<(), RoleError> {
        let client = pool.get().await?;

        // Handle rename separately if needed
        if let Some(ref new_name) = options.new_name {
            let rename_sql = format!(
                "ALTER ROLE {} RENAME TO {}",
                quote_ident(role_name),
                quote_ident(new_name)
            );
            client.execute(&rename_sql, &[]).await?;
        }

        // Build and execute ALTER for other options
        if let Some(sql) = Self::build_alter_role_sql(role_name, options) {
            client.execute(&sql, &[]).await?;
        }

        Ok(())
    }

    /// Build ALTER ROLE SQL statement (returns None if no changes)
    pub fn build_alter_role_sql(role_name: &str, options: &RoleAlterOptions) -> Option<String> {
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
            with_opts.push(&format!("ENCRYPTED PASSWORD '{}'", escape_string(password)));
        }

        if let Some(valid_opt) = &options.valid_until {
            match valid_opt {
                Some(valid) => {
                    with_opts.push(&format!(
                        "VALID UNTIL '{}'",
                        valid.format("%Y-%m-%d %H:%M:%S%:z")
                    ));
                }
                None => {
                    with_opts.push("VALID UNTIL 'infinity'");
                }
            }
        }

        if with_opts.is_empty() {
            return None;
        }

        Some(format!(
            "ALTER ROLE {} WITH {}",
            quote_ident(role_name),
            with_opts.join(" ")
        ))
    }

    /// Drop a role
    pub async fn drop_role(pool: &Pool, role_name: &str) -> Result<(), RoleError> {
        let client = pool.get().await?;
        let sql = format!("DROP ROLE {}", quote_ident(role_name));
        client.execute(&sql, &[]).await?;
        Ok(())
    }

    /// Drop a role with CASCADE (reassign owned objects first)
    pub async fn drop_role_cascade(
        pool: &Pool,
        role_name: &str,
        reassign_to: &str,
    ) -> Result<(), RoleError> {
        let client = pool.get().await?;

        // Reassign owned objects
        let reassign_sql = format!(
            "REASSIGN OWNED BY {} TO {}",
            quote_ident(role_name),
            quote_ident(reassign_to)
        );
        client.execute(&reassign_sql, &[]).await?;

        // Drop owned objects (privileges)
        let drop_owned_sql = format!("DROP OWNED BY {}", quote_ident(role_name));
        client.execute(&drop_owned_sql, &[]).await?;

        // Drop the role
        Self::drop_role(pool, role_name).await
    }

    /// Grant role membership
    pub async fn grant_role(
        pool: &Pool,
        role: &str,
        member: &str,
        with_admin: bool,
    ) -> Result<(), RoleError> {
        let client = pool.get().await?;

        let sql = if with_admin {
            format!(
                "GRANT {} TO {} WITH ADMIN OPTION",
                quote_ident(role),
                quote_ident(member)
            )
        } else {
            format!("GRANT {} TO {}", quote_ident(role), quote_ident(member))
        };

        client.execute(&sql, &[]).await?;
        Ok(())
    }

    /// Revoke role membership
    pub async fn revoke_role(pool: &Pool, role: &str, member: &str) -> Result<(), RoleError> {
        let client = pool.get().await?;
        let sql = format!(
            "REVOKE {} FROM {}",
            quote_ident(role),
            quote_ident(member)
        );
        client.execute(&sql, &[]).await?;
        Ok(())
    }

    /// Get role memberships with details
    pub async fn get_role_memberships(pool: &Pool) -> Result<Vec<RoleMembership>, RoleError> {
        let client = pool.get().await?;

        let rows = client
            .query(
                r#"
                SELECT
                    r.rolname AS role,
                    m.rolname AS member,
                    am.admin_option,
                    g.rolname AS grantor
                FROM pg_auth_members am
                JOIN pg_roles r ON r.oid = am.roleid
                JOIN pg_roles m ON m.oid = am.member
                JOIN pg_roles g ON g.oid = am.grantor
                WHERE r.rolname NOT LIKE 'pg_%'
                  AND m.rolname NOT LIKE 'pg_%'
                ORDER BY r.rolname, m.rolname
                "#,
                &[],
            )
            .await?;

        let memberships = rows
            .iter()
            .map(|row| RoleMembership {
                role: row.get("role"),
                member: row.get("member"),
                admin_option: row.get("admin_option"),
                grantor: row.get("grantor"),
            })
            .collect();

        Ok(memberships)
    }

    /// Get privileges granted to a role
    pub async fn get_role_privileges(
        pool: &Pool,
        role_name: &str,
    ) -> Result<Vec<Privilege>, RoleError> {
        let client = pool.get().await?;

        // Query table/view privileges
        let table_rows = client
            .query(
                r#"
                SELECT
                    grantee,
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

        let mut privileges: Vec<Privilege> = table_rows
            .iter()
            .map(|row| {
                let priv_strings: Vec<String> = row.get("privileges");
                let privileges = priv_strings
                    .iter()
                    .filter_map(|s| parse_privilege_type(s))
                    .collect();

                Privilege {
                    grantee: row.get("grantee"),
                    object_type: PrivilegeObjectType::Table,
                    schema: row.get("schema"),
                    object_name: row.get("object_name"),
                    privileges,
                    grant_option: row.get("grant_option"),
                    grantor: row.get("grantor"),
                }
            })
            .collect();

        // Query schema privileges
        let schema_rows = client
            .query(
                r#"
                SELECT
                    grantee,
                    nspname AS schema_name,
                    array_agg(
                        CASE
                            WHEN has_schema_privilege(grantee, nspname, 'USAGE') THEN 'USAGE'
                            WHEN has_schema_privilege(grantee, nspname, 'CREATE') THEN 'CREATE'
                        END
                    ) FILTER (WHERE has_schema_privilege(grantee, nspname, 'USAGE')
                                 OR has_schema_privilege(grantee, nspname, 'CREATE')) AS privileges
                FROM pg_namespace n
                CROSS JOIN (SELECT $1::text AS grantee) g
                WHERE n.nspname NOT LIKE 'pg_%'
                  AND n.nspname != 'information_schema'
                  AND (has_schema_privilege(grantee, nspname, 'USAGE')
                       OR has_schema_privilege(grantee, nspname, 'CREATE'))
                GROUP BY grantee, nspname
                "#,
                &[&role_name],
            )
            .await?;

        for row in schema_rows {
            let priv_strings: Vec<String> = row.get("privileges");
            let privs = priv_strings
                .iter()
                .filter_map(|s| parse_privilege_type(s))
                .collect();

            privileges.push(Privilege {
                grantee: row.get("grantee"),
                object_type: PrivilegeObjectType::Schema,
                schema: None,
                object_name: row.get("schema_name"),
                privileges: privs,
                grant_option: false,
                grantor: String::new(),
            });
        }

        Ok(privileges)
    }

    /// Grant privilege on an object
    pub async fn grant_privilege(
        pool: &Pool,
        privilege: PrivilegeType,
        object_type: PrivilegeObjectType,
        schema: Option<&str>,
        object_name: &str,
        role: &str,
        with_grant_option: bool,
    ) -> Result<(), RoleError> {
        let client = pool.get().await?;

        let object = if let Some(s) = schema {
            format!("{}.{}", quote_ident(s), quote_ident(object_name))
        } else {
            quote_ident(object_name)
        };

        let mut sql = format!(
            "GRANT {} ON {} {} TO {}",
            privilege.as_str(),
            object_type.as_str(),
            object,
            quote_ident(role)
        );

        if with_grant_option {
            sql.push_str(" WITH GRANT OPTION");
        }

        client.execute(&sql, &[]).await?;
        Ok(())
    }

    /// Revoke privilege on an object
    pub async fn revoke_privilege(
        pool: &Pool,
        privilege: PrivilegeType,
        object_type: PrivilegeObjectType,
        schema: Option<&str>,
        object_name: &str,
        role: &str,
    ) -> Result<(), RoleError> {
        let client = pool.get().await?;

        let object = if let Some(s) = schema {
            format!("{}.{}", quote_ident(s), quote_ident(object_name))
        } else {
            quote_ident(object_name)
        };

        let sql = format!(
            "REVOKE {} ON {} {} FROM {}",
            privilege.as_str(),
            object_type.as_str(),
            object,
            quote_ident(role)
        );

        client.execute(&sql, &[]).await?;
        Ok(())
    }

    /// Get default privileges for a role
    pub async fn get_default_privileges(
        pool: &Pool,
        role_name: &str,
    ) -> Result<Vec<DefaultPrivilege>, RoleError> {
        let client = pool.get().await?;

        let rows = client
            .query(
                r#"
                SELECT
                    r.rolname AS role,
                    n.nspname AS schema,
                    CASE d.objtype
                        WHEN 'r' THEN 'TABLE'
                        WHEN 'S' THEN 'SEQUENCE'
                        WHEN 'f' THEN 'FUNCTION'
                        WHEN 'T' THEN 'TYPE'
                        WHEN 'n' THEN 'SCHEMA'
                    END AS object_type,
                    a.rolname AS grantee,
                    d.defaclacl AS acl
                FROM pg_default_acl d
                JOIN pg_roles r ON r.oid = d.defaclrole
                LEFT JOIN pg_namespace n ON n.oid = d.defaclnamespace
                JOIN pg_roles a ON a.oid = ANY(d.defaclacl::text[]::oid[])
                WHERE r.rolname = $1
                "#,
                &[&role_name],
            )
            .await?;

        // Parse ACL entries (simplified - full implementation would parse PostgreSQL ACL format)
        let defaults = rows
            .iter()
            .filter_map(|row| {
                let obj_type_str: String = row.get("object_type");
                let object_type = match obj_type_str.as_str() {
                    "TABLE" => PrivilegeObjectType::Table,
                    "SEQUENCE" => PrivilegeObjectType::Sequence,
                    "FUNCTION" => PrivilegeObjectType::Function,
                    "TYPE" => PrivilegeObjectType::Type,
                    "SCHEMA" => PrivilegeObjectType::Schema,
                    _ => return None,
                };

                Some(DefaultPrivilege {
                    role: row.get("role"),
                    schema: row.get("schema"),
                    object_type,
                    grantee: row.get("grantee"),
                    privileges: vec![], // Would need ACL parsing
                })
            })
            .collect();

        Ok(defaults)
    }

    /// Set role comment
    pub async fn set_role_comment(
        pool: &Pool,
        role_name: &str,
        comment: Option<&str>,
    ) -> Result<(), RoleError> {
        let client = pool.get().await?;

        let sql = match comment {
            Some(c) => format!(
                "COMMENT ON ROLE {} IS '{}'",
                quote_ident(role_name),
                escape_string(c)
            ),
            None => format!("COMMENT ON ROLE {} IS NULL", quote_ident(role_name)),
        };

        client.execute(&sql, &[]).await?;
        Ok(())
    }
}

/// Quote an identifier for safe use in SQL
fn quote_ident(s: &str) -> String {
    if s.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        && !s.is_empty()
        && !s.chars().next().unwrap().is_ascii_digit()
    {
        s.to_string()
    } else {
        format!("\"{}\"", s.replace('"', "\"\""))
    }
}

/// Escape a string for safe use in SQL
fn escape_string(s: &str) -> String {
    s.replace('\'', "''")
}

/// Parse privilege type from string
fn parse_privilege_type(s: &str) -> Option<PrivilegeType> {
    match s.to_uppercase().as_str() {
        "SELECT" => Some(PrivilegeType::Select),
        "INSERT" => Some(PrivilegeType::Insert),
        "UPDATE" => Some(PrivilegeType::Update),
        "DELETE" => Some(PrivilegeType::Delete),
        "TRUNCATE" => Some(PrivilegeType::Truncate),
        "REFERENCES" => Some(PrivilegeType::References),
        "TRIGGER" => Some(PrivilegeType::Trigger),
        "USAGE" => Some(PrivilegeType::Usage),
        "CREATE" => Some(PrivilegeType::Create),
        "CONNECT" => Some(PrivilegeType::Connect),
        "TEMPORARY" | "TEMP" => Some(PrivilegeType::Temporary),
        "EXECUTE" => Some(PrivilegeType::Execute),
        "ALL" | "ALL PRIVILEGES" => Some(PrivilegeType::All),
        _ => None,
    }
}
```

### 22.3 Role State (GPUI Global)

```rust
// src/state/role_state.rs

use crate::models::role::{Privilege, Role, RoleAlterOptions, RoleCreateOptions, RoleMembership};
use crate::services::role::{RoleError, RoleService};
use deadpool_postgres::Pool;
use gpui::Global;
use parking_lot::RwLock;
use std::sync::Arc;

/// Application-wide role management state
pub struct RoleState {
    inner: Arc<RwLock<RoleStateInner>>,
}

struct RoleStateInner {
    /// All roles from the current connection
    roles: Vec<Role>,

    /// Currently selected role for editing/viewing
    selected_role: Option<String>,

    /// Privileges for the selected role
    privileges: Vec<Privilege>,

    /// Role memberships
    memberships: Vec<RoleMembership>,

    /// Loading state
    loading: bool,

    /// Error message
    error: Option<String>,

    /// Connection pool reference
    pool: Option<Pool>,
}

impl Global for RoleState {}

impl RoleState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(RoleStateInner {
                roles: Vec::new(),
                selected_role: None,
                privileges: Vec::new(),
                memberships: Vec::new(),
                loading: false,
                error: None,
                pool: None,
            })),
        }
    }

    /// Set the connection pool
    pub fn set_pool(&self, pool: Pool) {
        self.inner.write().pool = Some(pool);
    }

    /// Get all roles
    pub fn roles(&self) -> Vec<Role> {
        self.inner.read().roles.clone()
    }

    /// Get role names for membership selection
    pub fn role_names(&self) -> Vec<String> {
        self.inner.read().roles.iter().map(|r| r.name.clone()).collect()
    }

    /// Get selected role
    pub fn selected_role(&self) -> Option<Role> {
        let inner = self.inner.read();
        let name = inner.selected_role.as_ref()?;
        inner.roles.iter().find(|r| &r.name == name).cloned()
    }

    /// Get privileges for selected role
    pub fn privileges(&self) -> Vec<Privilege> {
        self.inner.read().privileges.clone()
    }

    /// Get role memberships
    pub fn memberships(&self) -> Vec<RoleMembership> {
        self.inner.read().memberships.clone()
    }

    /// Check if loading
    pub fn is_loading(&self) -> bool {
        self.inner.read().loading
    }

    /// Get error message
    pub fn error(&self) -> Option<String> {
        self.inner.read().error.clone()
    }

    /// Clear error
    pub fn clear_error(&self) {
        self.inner.write().error = None;
    }

    /// Load all roles from the database
    pub async fn load_roles(&self) -> Result<(), RoleError> {
        let pool = {
            let inner = self.inner.read();
            inner.pool.clone().ok_or_else(|| {
                RoleError::InvalidOperation("No connection pool".to_string())
            })?
        };

        self.inner.write().loading = true;
        self.inner.write().error = None;

        match RoleService::get_roles(&pool).await {
            Ok(roles) => {
                let mut inner = self.inner.write();
                inner.roles = roles;
                inner.loading = false;
                Ok(())
            }
            Err(e) => {
                let mut inner = self.inner.write();
                inner.loading = false;
                inner.error = Some(e.to_string());
                Err(e)
            }
        }
    }

    /// Load role memberships
    pub async fn load_memberships(&self) -> Result<(), RoleError> {
        let pool = {
            let inner = self.inner.read();
            inner.pool.clone().ok_or_else(|| {
                RoleError::InvalidOperation("No connection pool".to_string())
            })?
        };

        match RoleService::get_role_memberships(&pool).await {
            Ok(memberships) => {
                self.inner.write().memberships = memberships;
                Ok(())
            }
            Err(e) => {
                self.inner.write().error = Some(e.to_string());
                Err(e)
            }
        }
    }

    /// Select a role for viewing/editing
    pub async fn select_role(&self, name: &str) -> Result<(), RoleError> {
        self.inner.write().selected_role = Some(name.to_string());

        // Load privileges for the selected role
        let pool = {
            let inner = self.inner.read();
            inner.pool.clone().ok_or_else(|| {
                RoleError::InvalidOperation("No connection pool".to_string())
            })?
        };

        match RoleService::get_role_privileges(&pool, name).await {
            Ok(privileges) => {
                self.inner.write().privileges = privileges;
                Ok(())
            }
            Err(e) => {
                self.inner.write().error = Some(e.to_string());
                Err(e)
            }
        }
    }

    /// Clear selection
    pub fn clear_selection(&self) {
        let mut inner = self.inner.write();
        inner.selected_role = None;
        inner.privileges.clear();
    }

    /// Create a new role
    pub async fn create_role(&self, options: &RoleCreateOptions) -> Result<(), RoleError> {
        let pool = {
            let inner = self.inner.read();
            inner.pool.clone().ok_or_else(|| {
                RoleError::InvalidOperation("No connection pool".to_string())
            })?
        };

        RoleService::create_role(&pool, options).await?;
        self.load_roles().await?;
        Ok(())
    }

    /// Alter an existing role
    pub async fn alter_role(
        &self,
        role_name: &str,
        options: &RoleAlterOptions,
    ) -> Result<(), RoleError> {
        let pool = {
            let inner = self.inner.read();
            inner.pool.clone().ok_or_else(|| {
                RoleError::InvalidOperation("No connection pool".to_string())
            })?
        };

        RoleService::alter_role(&pool, role_name, options).await?;
        self.load_roles().await?;
        Ok(())
    }

    /// Drop a role
    pub async fn drop_role(&self, role_name: &str) -> Result<(), RoleError> {
        let pool = {
            let inner = self.inner.read();
            inner.pool.clone().ok_or_else(|| {
                RoleError::InvalidOperation("No connection pool".to_string())
            })?
        };

        // Clear selection if dropping selected role
        {
            let inner = self.inner.read();
            if inner.selected_role.as_deref() == Some(role_name) {
                drop(inner);
                self.clear_selection();
            }
        }

        RoleService::drop_role(&pool, role_name).await?;
        self.load_roles().await?;
        Ok(())
    }

    /// Grant role membership
    pub async fn grant_membership(
        &self,
        role: &str,
        member: &str,
        with_admin: bool,
    ) -> Result<(), RoleError> {
        let pool = {
            let inner = self.inner.read();
            inner.pool.clone().ok_or_else(|| {
                RoleError::InvalidOperation("No connection pool".to_string())
            })?
        };

        RoleService::grant_role(&pool, role, member, with_admin).await?;
        self.load_roles().await?;
        self.load_memberships().await?;
        Ok(())
    }

    /// Revoke role membership
    pub async fn revoke_membership(&self, role: &str, member: &str) -> Result<(), RoleError> {
        let pool = {
            let inner = self.inner.read();
            inner.pool.clone().ok_or_else(|| {
                RoleError::InvalidOperation("No connection pool".to_string())
            })?
        };

        RoleService::revoke_role(&pool, role, member).await?;
        self.load_roles().await?;
        self.load_memberships().await?;
        Ok(())
    }

    /// Generate CREATE ROLE SQL
    pub fn generate_create_sql(&self, options: &RoleCreateOptions) -> String {
        RoleService::build_create_role_sql(options)
    }

    /// Generate ALTER ROLE SQL
    pub fn generate_alter_sql(&self, role_name: &str, options: &RoleAlterOptions) -> Option<String> {
        RoleService::build_alter_role_sql(role_name, options)
    }
}

impl Default for RoleState {
    fn default() -> Self {
        Self::new()
    }
}
```

### 22.4 Role List View Component

```rust
// src/components/roles/role_list.rs

use crate::models::role::Role;
use crate::state::role_state::RoleState;
use crate::ui::{
    Button, ButtonVariant, Checkbox, Icon, IconName, Input, ScrollView, Table,
    TableColumn, TableRow, Tooltip,
};
use gpui::{
    div, px, AppContext, Context, Element, EventEmitter, FocusHandle, FocusableView,
    InteractiveElement, IntoElement, Model, ParentElement, Render, SharedString,
    Styled, View, ViewContext, VisualContext, WindowContext,
};

/// Events emitted by the role list
pub enum RoleListEvent {
    CreateRole,
    EditRole(String),
    DeleteRole(String),
    SelectRole(String),
}

pub struct RoleListView {
    focus_handle: FocusHandle,
    filter_text: String,
    show_login_only: bool,
    filtered_roles: Vec<Role>,
}

impl EventEmitter<RoleListEvent> for RoleListView {}

impl RoleListView {
    pub fn new(cx: &mut ViewContext<Self>) -> Self {
        let mut view = Self {
            focus_handle: cx.focus_handle(),
            filter_text: String::new(),
            show_login_only: false,
            filtered_roles: Vec::new(),
        };

        view.refresh_filter(cx);
        view
    }

    fn refresh_filter(&mut self, cx: &mut ViewContext<Self>) {
        let role_state = cx.global::<RoleState>();
        let all_roles = role_state.roles();

        self.filtered_roles = all_roles
            .into_iter()
            .filter(|role| {
                // Filter by search text
                if !self.filter_text.is_empty()
                    && !role
                        .name
                        .to_lowercase()
                        .contains(&self.filter_text.to_lowercase())
                {
                    return false;
                }

                // Filter by login capability
                if self.show_login_only && !role.can_login {
                    return false;
                }

                true
            })
            .collect();
    }

    fn set_filter(&mut self, text: String, cx: &mut ViewContext<Self>) {
        self.filter_text = text;
        self.refresh_filter(cx);
        cx.notify();
    }

    fn toggle_login_filter(&mut self, cx: &mut ViewContext<Self>) {
        self.show_login_only = !self.show_login_only;
        self.refresh_filter(cx);
        cx.notify();
    }

    fn handle_create(&mut self, cx: &mut ViewContext<Self>) {
        cx.emit(RoleListEvent::CreateRole);
    }

    fn handle_edit(&mut self, role_name: String, cx: &mut ViewContext<Self>) {
        cx.emit(RoleListEvent::EditRole(role_name));
    }

    fn handle_delete(&mut self, role_name: String, cx: &mut ViewContext<Self>) {
        cx.emit(RoleListEvent::DeleteRole(role_name));
    }

    fn handle_select(&mut self, role_name: String, cx: &mut ViewContext<Self>) {
        cx.emit(RoleListEvent::SelectRole(role_name.clone()));

        let role_state = cx.global::<RoleState>().clone();
        cx.spawn(|_, _| async move {
            let _ = role_state.select_role(&role_name).await;
        })
        .detach();
    }

    fn render_role_badges(&self, role: &Role) -> impl IntoElement {
        let mut badges = Vec::new();

        if role.is_superuser {
            badges.push(("Superuser", "bg-red-100 text-red-700"));
        }
        if role.can_login {
            badges.push(("Login", "bg-green-100 text-green-700"));
        }
        if role.can_create_db {
            badges.push(("Create DB", "bg-blue-100 text-blue-700"));
        }
        if role.can_create_role {
            badges.push(("Create Role", "bg-purple-100 text-purple-700"));
        }
        if role.is_replication {
            badges.push(("Replication", "bg-orange-100 text-orange-700"));
        }
        if role.bypass_rls {
            badges.push(("Bypass RLS", "bg-yellow-100 text-yellow-700"));
        }

        div()
            .flex()
            .gap_1()
            .children(badges.into_iter().map(|(label, class)| {
                div()
                    .px_1()
                    .py_px()
                    .rounded(px(4.0))
                    .text_xs()
                    .class(class)
                    .child(label)
            }))
    }

    fn render_membership_badges(&self, role: &Role) -> impl IntoElement {
        let visible_count = 3;
        let member_of = &role.member_of;

        div().flex().gap_1().children(
            member_of
                .iter()
                .take(visible_count)
                .map(|name| {
                    div()
                        .px_1()
                        .py_px()
                        .bg(gpui::rgb(0xf3f4f6))
                        .rounded(px(4.0))
                        .text_xs()
                        .child(name.clone())
                })
                .chain(if member_of.len() > visible_count {
                    Some(
                        div()
                            .text_xs()
                            .text_color(gpui::rgb(0x6b7280))
                            .child(format!("+{}", member_of.len() - visible_count)),
                    )
                } else {
                    None
                }),
        )
    }
}

impl FocusableView for RoleListView {
    fn focus_handle(&self, _cx: &AppContext) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for RoleListView {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let role_state = cx.global::<RoleState>();
        let selected_role = role_state.selected_role();
        let is_loading = role_state.is_loading();

        div()
            .flex()
            .flex_col()
            .size_full()
            .track_focus(&self.focus_handle)
            .child(
                // Toolbar
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .p_4()
                    .border_b_1()
                    .border_color(gpui::rgb(0xe5e7eb))
                    .child(
                        Input::new("filter-roles")
                            .placeholder("Filter roles...")
                            .value(self.filter_text.clone())
                            .on_change(cx.listener(|this, text: &String, cx| {
                                this.set_filter(text.clone(), cx);
                            }))
                            .flex_1(),
                    )
                    .child(
                        Checkbox::new("login-only")
                            .checked(self.show_login_only)
                            .label("Login only")
                            .on_toggle(cx.listener(|this, _, cx| {
                                this.toggle_login_filter(cx);
                            })),
                    )
                    .child(
                        Button::new("create-role")
                            .variant(ButtonVariant::Primary)
                            .icon(IconName::Plus)
                            .label("New Role")
                            .on_click(cx.listener(|this, _, cx| {
                                this.handle_create(cx);
                            })),
                    ),
            )
            .child(
                // Role table
                ScrollView::new("role-list-scroll").child(
                    Table::new("roles-table")
                        .header(vec![
                            TableColumn::new("Role").width(px(200.0)),
                            TableColumn::new("Attributes").flex(1.0),
                            TableColumn::new("Connections").width(px(100.0)).center(),
                            TableColumn::new("Member Of").width(px(200.0)),
                            TableColumn::new("Actions").width(px(100.0)).right(),
                        ])
                        .loading(is_loading)
                        .empty_message("No roles found")
                        .children(self.filtered_roles.iter().map(|role| {
                            let role_name = role.name.clone();
                            let is_selected = selected_role
                                .as_ref()
                                .map(|r| r.name == role.name)
                                .unwrap_or(false);

                            let name_for_select = role_name.clone();
                            let name_for_edit = role_name.clone();
                            let name_for_delete = role_name.clone();

                            TableRow::new(format!("role-{}", role.oid))
                                .selected(is_selected)
                                .on_click(cx.listener(move |this, _, cx| {
                                    this.handle_select(name_for_select.clone(), cx);
                                }))
                                .child(
                                    // Role name column
                                    div()
                                        .flex()
                                        .flex_col()
                                        .child(
                                            div()
                                                .font_weight(gpui::FontWeight::MEDIUM)
                                                .child(role.name.clone()),
                                        )
                                        .when_some(role.comment.as_ref(), |this, comment| {
                                            this.child(
                                                div()
                                                    .text_xs()
                                                    .text_color(gpui::rgb(0x6b7280))
                                                    .truncate()
                                                    .max_w(px(180.0))
                                                    .child(comment.clone()),
                                            )
                                        }),
                                )
                                .child(
                                    // Attributes column
                                    self.render_role_badges(role),
                                )
                                .child(
                                    // Connection limit column
                                    div().text_center().child(if role.connection_limit < 0 {
                                        SharedString::from("∞")
                                    } else {
                                        SharedString::from(role.connection_limit.to_string())
                                    }),
                                )
                                .child(
                                    // Member of column
                                    if role.member_of.is_empty() {
                                        div()
                                            .text_color(gpui::rgb(0x9ca3af))
                                            .child("-")
                                    } else {
                                        self.render_membership_badges(role)
                                    },
                                )
                                .child(
                                    // Actions column
                                    div()
                                        .flex()
                                        .justify_end()
                                        .gap_1()
                                        .child(
                                            Tooltip::new("Edit role").child(
                                                Button::new(format!("edit-{}", role_name))
                                                    .icon(IconName::Pencil)
                                                    .variant(ButtonVariant::Ghost)
                                                    .small()
                                                    .on_click(cx.listener(
                                                        move |this, _, cx| {
                                                            this.handle_edit(
                                                                name_for_edit.clone(),
                                                                cx,
                                                            );
                                                        },
                                                    )),
                                            ),
                                        )
                                        .child(
                                            Tooltip::new("Delete role").child(
                                                Button::new(format!("delete-{}", role_name))
                                                    .icon(IconName::Trash)
                                                    .variant(ButtonVariant::Ghost)
                                                    .small()
                                                    .danger()
                                                    .on_click(cx.listener(
                                                        move |this, _, cx| {
                                                            this.handle_delete(
                                                                name_for_delete.clone(),
                                                                cx,
                                                            );
                                                        },
                                                    )),
                                            ),
                                        ),
                                )
                        })),
                ),
            )
    }
}
```

### 22.5 Role Editor Dialog

```rust
// src/components/roles/role_editor.rs

use crate::models::role::{Role, RoleAlterOptions, RoleCreateOptions};
use crate::state::role_state::RoleState;
use crate::ui::{
    Button, ButtonVariant, Checkbox, DatePicker, Input, Modal, ModalFooter,
    NumberInput, ScrollView, Section, Select, SelectOption, TextArea,
};
use chrono::{DateTime, NaiveDate, Utc};
use gpui::{
    div, px, AppContext, Context, Element, EventEmitter, FocusHandle, FocusableView,
    InteractiveElement, IntoElement, ParentElement, Render, SharedString, Styled,
    View, ViewContext, VisualContext,
};

/// Events emitted by the role editor
pub enum RoleEditorEvent {
    Saved,
    Cancelled,
}

/// Mode for the role editor
#[derive(Clone)]
pub enum RoleEditorMode {
    Create,
    Edit(Role),
}

pub struct RoleEditorDialog {
    focus_handle: FocusHandle,
    mode: RoleEditorMode,

    // Form fields
    name: String,
    password: String,
    confirm_password: String,
    superuser: bool,
    createdb: bool,
    createrole: bool,
    inherit: bool,
    login: bool,
    replication: bool,
    bypassrls: bool,
    connection_limit: i32,
    valid_until: Option<NaiveDate>,
    member_of: Vec<String>,
    comment: String,

    // UI state
    show_sql: bool,
    generated_sql: String,
    saving: bool,
    error: Option<String>,

    // Available roles for membership
    available_roles: Vec<String>,
}

impl EventEmitter<RoleEditorEvent> for RoleEditorDialog {}

impl RoleEditorDialog {
    pub fn new_create(cx: &mut ViewContext<Self>) -> Self {
        let role_state = cx.global::<RoleState>();
        let available_roles = role_state.role_names();

        Self {
            focus_handle: cx.focus_handle(),
            mode: RoleEditorMode::Create,
            name: String::new(),
            password: String::new(),
            confirm_password: String::new(),
            superuser: false,
            createdb: false,
            createrole: false,
            inherit: true, // PostgreSQL default
            login: true,   // Common default for users
            replication: false,
            bypassrls: false,
            connection_limit: -1, // Unlimited
            valid_until: None,
            member_of: Vec::new(),
            comment: String::new(),
            show_sql: false,
            generated_sql: String::new(),
            saving: false,
            error: None,
            available_roles,
        }
    }

    pub fn new_edit(role: Role, cx: &mut ViewContext<Self>) -> Self {
        let role_state = cx.global::<RoleState>();
        let available_roles: Vec<String> = role_state
            .role_names()
            .into_iter()
            .filter(|r| r != &role.name)
            .collect();

        let valid_until = role.valid_until.map(|dt| dt.date_naive());

        Self {
            focus_handle: cx.focus_handle(),
            mode: RoleEditorMode::Edit(role.clone()),
            name: role.name.clone(),
            password: String::new(),
            confirm_password: String::new(),
            superuser: role.is_superuser,
            createdb: role.can_create_db,
            createrole: role.can_create_role,
            inherit: role.inherit_privileges,
            login: role.can_login,
            replication: role.is_replication,
            bypassrls: role.bypass_rls,
            connection_limit: role.connection_limit,
            valid_until,
            member_of: role.member_of.clone(),
            comment: role.comment.unwrap_or_default(),
            show_sql: false,
            generated_sql: String::new(),
            saving: false,
            error: None,
            available_roles,
        }
    }

    fn is_edit_mode(&self) -> bool {
        matches!(self.mode, RoleEditorMode::Edit(_))
    }

    fn original_role(&self) -> Option<&Role> {
        match &self.mode {
            RoleEditorMode::Edit(role) => Some(role),
            RoleEditorMode::Create => None,
        }
    }

    fn password_error(&self) -> Option<&'static str> {
        if !self.password.is_empty()
            && !self.confirm_password.is_empty()
            && self.password != self.confirm_password
        {
            Some("Passwords do not match")
        } else {
            None
        }
    }

    fn can_save(&self) -> bool {
        if self.saving {
            return false;
        }

        if self.password_error().is_some() {
            return false;
        }

        if !self.is_edit_mode() && self.name.is_empty() {
            return false;
        }

        true
    }

    fn build_create_options(&self) -> RoleCreateOptions {
        let valid_until = self.valid_until.map(|date| {
            DateTime::<Utc>::from_naive_utc_and_offset(
                date.and_hms_opt(23, 59, 59).unwrap(),
                Utc,
            )
        });

        RoleCreateOptions {
            name: self.name.clone(),
            password: if self.password.is_empty() {
                None
            } else {
                Some(self.password.clone())
            },
            superuser: self.superuser,
            createdb: self.createdb,
            createrole: self.createrole,
            inherit: self.inherit,
            login: self.login,
            replication: self.replication,
            bypassrls: self.bypassrls,
            connection_limit: self.connection_limit,
            valid_until,
            in_roles: self.member_of.clone(),
            roles: Vec::new(),
            admin_roles: Vec::new(),
        }
    }

    fn build_alter_options(&self) -> RoleAlterOptions {
        let original = self.original_role();

        // Only include changed values
        let valid_until = match (original.and_then(|r| r.valid_until), self.valid_until) {
            (Some(_), None) => Some(None), // Clear expiration
            (_, Some(date)) => Some(Some(DateTime::<Utc>::from_naive_utc_and_offset(
                date.and_hms_opt(23, 59, 59).unwrap(),
                Utc,
            ))),
            _ => None,
        };

        RoleAlterOptions {
            new_name: None, // Handle rename separately
            password: if self.password.is_empty() {
                None
            } else {
                Some(self.password.clone())
            },
            superuser: original
                .filter(|r| r.is_superuser != self.superuser)
                .map(|_| self.superuser),
            createdb: original
                .filter(|r| r.can_create_db != self.createdb)
                .map(|_| self.createdb),
            createrole: original
                .filter(|r| r.can_create_role != self.createrole)
                .map(|_| self.createrole),
            inherit: original
                .filter(|r| r.inherit_privileges != self.inherit)
                .map(|_| self.inherit),
            login: original
                .filter(|r| r.can_login != self.login)
                .map(|_| self.login),
            replication: original
                .filter(|r| r.is_replication != self.replication)
                .map(|_| self.replication),
            bypassrls: original
                .filter(|r| r.bypass_rls != self.bypassrls)
                .map(|_| self.bypassrls),
            connection_limit: original
                .filter(|r| r.connection_limit != self.connection_limit)
                .map(|_| self.connection_limit),
            valid_until,
        }
    }

    fn generate_sql(&mut self, cx: &mut ViewContext<Self>) {
        let role_state = cx.global::<RoleState>();

        self.generated_sql = if self.is_edit_mode() {
            let options = self.build_alter_options();
            role_state
                .generate_alter_sql(&self.name, &options)
                .unwrap_or_else(|| "-- No changes".to_string())
        } else {
            let options = self.build_create_options();
            role_state.generate_create_sql(&options)
        };

        self.show_sql = true;
        cx.notify();
    }

    fn handle_save(&mut self, cx: &mut ViewContext<Self>) {
        if !self.can_save() {
            return;
        }

        self.saving = true;
        self.error = None;
        cx.notify();

        let role_state = cx.global::<RoleState>().clone();
        let is_edit = self.is_edit_mode();
        let name = self.name.clone();
        let original_member_of = self
            .original_role()
            .map(|r| r.member_of.clone())
            .unwrap_or_default();
        let new_member_of = self.member_of.clone();

        if is_edit {
            let options = self.build_alter_options();

            cx.spawn(|this, mut cx| async move {
                // Alter role attributes
                if let Err(e) = role_state.alter_role(&name, &options).await {
                    this.update(&mut cx, |this, cx| {
                        this.saving = false;
                        this.error = Some(e.to_string());
                        cx.notify();
                    })
                    .ok();
                    return;
                }

                // Handle membership changes
                let original_set: std::collections::HashSet<_> =
                    original_member_of.iter().collect();
                let new_set: std::collections::HashSet<_> = new_member_of.iter().collect();

                // Revoke removed memberships
                for role in original_set.difference(&new_set) {
                    if let Err(e) = role_state.revoke_membership(role, &name).await {
                        this.update(&mut cx, |this, cx| {
                            this.error = Some(format!("Failed to revoke {}: {}", role, e));
                            cx.notify();
                        })
                        .ok();
                    }
                }

                // Grant new memberships
                for role in new_set.difference(&original_set) {
                    if let Err(e) = role_state.grant_membership(role, &name, false).await {
                        this.update(&mut cx, |this, cx| {
                            this.error = Some(format!("Failed to grant {}: {}", role, e));
                            cx.notify();
                        })
                        .ok();
                    }
                }

                this.update(&mut cx, |this, cx| {
                    this.saving = false;
                    cx.emit(RoleEditorEvent::Saved);
                })
                .ok();
            })
            .detach();
        } else {
            let options = self.build_create_options();

            cx.spawn(|this, mut cx| async move {
                match role_state.create_role(&options).await {
                    Ok(()) => {
                        this.update(&mut cx, |this, cx| {
                            this.saving = false;
                            cx.emit(RoleEditorEvent::Saved);
                        })
                        .ok();
                    }
                    Err(e) => {
                        this.update(&mut cx, |this, cx| {
                            this.saving = false;
                            this.error = Some(e.to_string());
                            cx.notify();
                        })
                        .ok();
                    }
                }
            })
            .detach();
        }
    }

    fn handle_cancel(&mut self, cx: &mut ViewContext<Self>) {
        cx.emit(RoleEditorEvent::Cancelled);
    }

    fn toggle_membership(&mut self, role_name: String, cx: &mut ViewContext<Self>) {
        if let Some(pos) = self.member_of.iter().position(|r| r == &role_name) {
            self.member_of.remove(pos);
        } else {
            self.member_of.push(role_name);
        }
        cx.notify();
    }
}

impl FocusableView for RoleEditorDialog {
    fn focus_handle(&self, _cx: &AppContext) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for RoleEditorDialog {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let title = if self.is_edit_mode() {
            format!("Edit Role: {}", self.name)
        } else {
            "Create New Role".to_string()
        };

        let password_error = self.password_error();
        let can_save = self.can_save();

        Modal::new("role-editor")
            .title(title)
            .width(px(600.0))
            .child(
                ScrollView::new("role-editor-scroll")
                    .max_h(px(500.0))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_4()
                            .p_4()
                            // Error message
                            .when_some(self.error.clone(), |this, error| {
                                this.child(
                                    div()
                                        .p_3()
                                        .bg(gpui::rgb(0xfef2f2))
                                        .border_1()
                                        .border_color(gpui::rgb(0xfecaca))
                                        .rounded(px(6.0))
                                        .text_sm()
                                        .text_color(gpui::rgb(0xb91c1c))
                                        .child(error),
                                )
                            })
                            // Role name
                            .child(
                                Section::new("role-name")
                                    .label("Role Name")
                                    .child(
                                        Input::new("name")
                                            .value(self.name.clone())
                                            .disabled(self.is_edit_mode())
                                            .on_change(cx.listener(|this, value: &String, cx| {
                                                this.name = value.clone();
                                                cx.notify();
                                            })),
                                    ),
                            )
                            // Password section
                            .child(
                                div()
                                    .flex()
                                    .gap_4()
                                    .child(
                                        Section::new("password")
                                            .label(if self.is_edit_mode() {
                                                "New Password"
                                            } else {
                                                "Password"
                                            })
                                            .flex_1()
                                            .child(
                                                Input::new("password")
                                                    .password()
                                                    .value(self.password.clone())
                                                    .placeholder(if self.is_edit_mode() {
                                                        "Leave empty to keep current"
                                                    } else {
                                                        ""
                                                    })
                                                    .on_change(cx.listener(
                                                        |this, value: &String, cx| {
                                                            this.password = value.clone();
                                                            cx.notify();
                                                        },
                                                    )),
                                            ),
                                    )
                                    .child(
                                        Section::new("confirm-password")
                                            .label("Confirm Password")
                                            .flex_1()
                                            .error(password_error.map(|s| s.to_string()))
                                            .child(
                                                Input::new("confirm-password")
                                                    .password()
                                                    .value(self.confirm_password.clone())
                                                    .on_change(cx.listener(
                                                        |this, value: &String, cx| {
                                                            this.confirm_password = value.clone();
                                                            cx.notify();
                                                        },
                                                    )),
                                            ),
                                    ),
                            )
                            // Privileges section
                            .child(
                                Section::new("privileges")
                                    .label("Privileges")
                                    .child(
                                        div()
                                            .grid()
                                            .grid_cols_2()
                                            .gap_3()
                                            .child(
                                                Checkbox::new("login")
                                                    .checked(self.login)
                                                    .label("Can login")
                                                    .on_toggle(cx.listener(|this, _, cx| {
                                                        this.login = !this.login;
                                                        cx.notify();
                                                    })),
                                            )
                                            .child(
                                                Checkbox::new("superuser")
                                                    .checked(self.superuser)
                                                    .label("Superuser")
                                                    .on_toggle(cx.listener(|this, _, cx| {
                                                        this.superuser = !this.superuser;
                                                        cx.notify();
                                                    })),
                                            )
                                            .child(
                                                Checkbox::new("createdb")
                                                    .checked(self.createdb)
                                                    .label("Create databases")
                                                    .on_toggle(cx.listener(|this, _, cx| {
                                                        this.createdb = !this.createdb;
                                                        cx.notify();
                                                    })),
                                            )
                                            .child(
                                                Checkbox::new("createrole")
                                                    .checked(self.createrole)
                                                    .label("Create roles")
                                                    .on_toggle(cx.listener(|this, _, cx| {
                                                        this.createrole = !this.createrole;
                                                        cx.notify();
                                                    })),
                                            )
                                            .child(
                                                Checkbox::new("inherit")
                                                    .checked(self.inherit)
                                                    .label("Inherit privileges")
                                                    .on_toggle(cx.listener(|this, _, cx| {
                                                        this.inherit = !this.inherit;
                                                        cx.notify();
                                                    })),
                                            )
                                            .child(
                                                Checkbox::new("replication")
                                                    .checked(self.replication)
                                                    .label("Replication")
                                                    .on_toggle(cx.listener(|this, _, cx| {
                                                        this.replication = !this.replication;
                                                        cx.notify();
                                                    })),
                                            )
                                            .child(
                                                Checkbox::new("bypassrls")
                                                    .checked(self.bypassrls)
                                                    .label("Bypass RLS")
                                                    .on_toggle(cx.listener(|this, _, cx| {
                                                        this.bypassrls = !this.bypassrls;
                                                        cx.notify();
                                                    })),
                                            ),
                                    ),
                            )
                            // Limits section
                            .child(
                                div()
                                    .flex()
                                    .gap_4()
                                    .child(
                                        Section::new("connection-limit")
                                            .label("Connection Limit")
                                            .hint("-1 = unlimited")
                                            .flex_1()
                                            .child(
                                                NumberInput::new("connection-limit")
                                                    .value(self.connection_limit)
                                                    .min(-1)
                                                    .on_change(cx.listener(
                                                        |this, value: &i32, cx| {
                                                            this.connection_limit = *value;
                                                            cx.notify();
                                                        },
                                                    )),
                                            ),
                                    )
                                    .child(
                                        Section::new("valid-until")
                                            .label("Valid Until")
                                            .hint("Leave empty for no expiration")
                                            .flex_1()
                                            .child(
                                                DatePicker::new("valid-until")
                                                    .value(self.valid_until)
                                                    .clearable()
                                                    .on_change(cx.listener(
                                                        |this, value: &Option<NaiveDate>, cx| {
                                                            this.valid_until = *value;
                                                            cx.notify();
                                                        },
                                                    )),
                                            ),
                                    ),
                            )
                            // Membership section
                            .child(
                                Section::new("member-of")
                                    .label("Member Of")
                                    .child(
                                        div()
                                            .max_h(px(120.0))
                                            .overflow_y_auto()
                                            .border_1()
                                            .border_color(gpui::rgb(0xe5e7eb))
                                            .rounded(px(6.0))
                                            .p_2()
                                            .children(
                                                if self.available_roles.is_empty() {
                                                    vec![div()
                                                        .text_sm()
                                                        .text_color(gpui::rgb(0x6b7280))
                                                        .py_2()
                                                        .text_center()
                                                        .child("No other roles available")]
                                                } else {
                                                    self.available_roles
                                                        .iter()
                                                        .map(|role_name| {
                                                            let is_member =
                                                                self.member_of.contains(role_name);
                                                            let role_name_clone = role_name.clone();

                                                            Checkbox::new(format!(
                                                                "member-{}",
                                                                role_name
                                                            ))
                                                            .checked(is_member)
                                                            .label(role_name.clone())
                                                            .on_toggle(cx.listener(
                                                                move |this, _, cx| {
                                                                    this.toggle_membership(
                                                                        role_name_clone.clone(),
                                                                        cx,
                                                                    );
                                                                },
                                                            ))
                                                            .into_any_element()
                                                        })
                                                        .collect()
                                                },
                                            ),
                                    ),
                            )
                            // Comment section
                            .child(
                                Section::new("comment")
                                    .label("Comment")
                                    .child(
                                        TextArea::new("comment")
                                            .value(self.comment.clone())
                                            .rows(2)
                                            .on_change(cx.listener(|this, value: &String, cx| {
                                                this.comment = value.clone();
                                                cx.notify();
                                            })),
                                    ),
                            )
                            // SQL preview
                            .when(self.show_sql, |this| {
                                this.child(
                                    Section::new("sql-preview")
                                        .label("Generated SQL")
                                        .child(
                                            div()
                                                .p_3()
                                                .bg(gpui::rgb(0xf3f4f6))
                                                .rounded(px(6.0))
                                                .font_family("monospace")
                                                .text_xs()
                                                .overflow_x_auto()
                                                .child(self.generated_sql.clone()),
                                        ),
                                )
                            }),
                    ),
            )
            .footer(
                ModalFooter::new()
                    .left(
                        Button::new("view-sql")
                            .label("View SQL")
                            .variant(ButtonVariant::Ghost)
                            .on_click(cx.listener(|this, _, cx| {
                                this.generate_sql(cx);
                            })),
                    )
                    .right(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                Button::new("cancel")
                                    .label("Cancel")
                                    .variant(ButtonVariant::Secondary)
                                    .on_click(cx.listener(|this, _, cx| {
                                        this.handle_cancel(cx);
                                    })),
                            )
                            .child(
                                Button::new("save")
                                    .label(if self.saving {
                                        "Saving..."
                                    } else if self.is_edit_mode() {
                                        "Save Changes"
                                    } else {
                                        "Create Role"
                                    })
                                    .variant(ButtonVariant::Primary)
                                    .disabled(!can_save)
                                    .on_click(cx.listener(|this, _, cx| {
                                        this.handle_save(cx);
                                    })),
                            ),
                    ),
            )
    }
}
```

### 22.6 Role Privileges View

```rust
// src/components/roles/privileges_view.rs

use crate::models::role::{Privilege, PrivilegeObjectType, PrivilegeType};
use crate::state::role_state::RoleState;
use crate::ui::{
    Button, ButtonVariant, EmptyState, Icon, IconName, ScrollView, Select,
    SelectOption, Table, TableColumn, TableRow, Tooltip,
};
use gpui::{
    div, px, AppContext, Context, Element, EventEmitter, FocusHandle, FocusableView,
    InteractiveElement, IntoElement, ParentElement, Render, SharedString, Styled,
    View, ViewContext, VisualContext,
};

/// Events emitted by the privileges view
pub enum PrivilegesViewEvent {
    GrantPrivilege {
        object_type: PrivilegeObjectType,
        schema: Option<String>,
        object_name: String,
        privilege: PrivilegeType,
    },
    RevokePrivilege {
        object_type: PrivilegeObjectType,
        schema: Option<String>,
        object_name: String,
        privilege: PrivilegeType,
    },
}

pub struct PrivilegesView {
    focus_handle: FocusHandle,
    filter_object_type: Option<PrivilegeObjectType>,
}

impl EventEmitter<PrivilegesViewEvent> for PrivilegesView {}

impl PrivilegesView {
    pub fn new(cx: &mut ViewContext<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            filter_object_type: None,
        }
    }

    fn set_object_type_filter(
        &mut self,
        object_type: Option<PrivilegeObjectType>,
        cx: &mut ViewContext<Self>,
    ) {
        self.filter_object_type = object_type;
        cx.notify();
    }

    fn filtered_privileges(&self, cx: &ViewContext<Self>) -> Vec<Privilege> {
        let role_state = cx.global::<RoleState>();
        let privileges = role_state.privileges();

        if let Some(filter_type) = self.filter_object_type {
            privileges
                .into_iter()
                .filter(|p| p.object_type == filter_type)
                .collect()
        } else {
            privileges
        }
    }

    fn render_privileges_badges(&self, privileges: &[PrivilegeType]) -> impl IntoElement {
        div()
            .flex()
            .flex_wrap()
            .gap_1()
            .children(privileges.iter().map(|priv_type| {
                let (label, bg_color) = match priv_type {
                    PrivilegeType::Select => ("SELECT", 0xe0f2fe),
                    PrivilegeType::Insert => ("INSERT", 0xdcfce7),
                    PrivilegeType::Update => ("UPDATE", 0xfef3c7),
                    PrivilegeType::Delete => ("DELETE", 0xfee2e2),
                    PrivilegeType::Truncate => ("TRUNCATE", 0xfce7f3),
                    PrivilegeType::References => ("REFERENCES", 0xede9fe),
                    PrivilegeType::Trigger => ("TRIGGER", 0xf3e8ff),
                    PrivilegeType::Usage => ("USAGE", 0xe0e7ff),
                    PrivilegeType::Create => ("CREATE", 0xccfbf1),
                    PrivilegeType::Connect => ("CONNECT", 0xcffafe),
                    PrivilegeType::Temporary => ("TEMP", 0xfef9c3),
                    PrivilegeType::Execute => ("EXECUTE", 0xd1fae5),
                    PrivilegeType::All => ("ALL", 0xfecaca),
                };

                div()
                    .px_1()
                    .py_px()
                    .bg(gpui::rgb(bg_color))
                    .rounded(px(4.0))
                    .text_xs()
                    .child(label)
            }))
    }

    fn object_type_icon(&self, object_type: PrivilegeObjectType) -> IconName {
        match object_type {
            PrivilegeObjectType::Table => IconName::Table,
            PrivilegeObjectType::View => IconName::Eye,
            PrivilegeObjectType::Sequence => IconName::ListOrdered,
            PrivilegeObjectType::Function => IconName::Code,
            PrivilegeObjectType::Schema => IconName::Folder,
            PrivilegeObjectType::Database => IconName::Database,
            PrivilegeObjectType::Tablespace => IconName::HardDrive,
            PrivilegeObjectType::Type => IconName::Type,
        }
    }
}

impl FocusableView for PrivilegesView {
    fn focus_handle(&self, _cx: &AppContext) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for PrivilegesView {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let role_state = cx.global::<RoleState>();
        let selected_role = role_state.selected_role();
        let privileges = self.filtered_privileges(cx);

        div()
            .flex()
            .flex_col()
            .size_full()
            .track_focus(&self.focus_handle)
            .child(
                // Header
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .p_4()
                    .border_b_1()
                    .border_color(gpui::rgb(0xe5e7eb))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .text_lg()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child("Privileges"),
                            )
                            .when_some(selected_role.as_ref(), |this, role| {
                                this.child(
                                    div()
                                        .text_sm()
                                        .text_color(gpui::rgb(0x6b7280))
                                        .child(format!("for {}", role.name)),
                                )
                            }),
                    )
                    .child(
                        Select::new("filter-object-type")
                            .placeholder("All object types")
                            .width(px(180.0))
                            .value(
                                self.filter_object_type
                                    .map(|t| SharedString::from(t.as_str())),
                            )
                            .options(vec![
                                SelectOption::new("", "All object types"),
                                SelectOption::new("TABLE", "Tables"),
                                SelectOption::new("VIEW", "Views"),
                                SelectOption::new("SEQUENCE", "Sequences"),
                                SelectOption::new("FUNCTION", "Functions"),
                                SelectOption::new("SCHEMA", "Schemas"),
                                SelectOption::new("DATABASE", "Databases"),
                            ])
                            .on_change(cx.listener(|this, value: &Option<SharedString>, cx| {
                                let object_type = value.as_ref().and_then(|v| {
                                    match v.as_ref() {
                                        "TABLE" => Some(PrivilegeObjectType::Table),
                                        "VIEW" => Some(PrivilegeObjectType::View),
                                        "SEQUENCE" => Some(PrivilegeObjectType::Sequence),
                                        "FUNCTION" => Some(PrivilegeObjectType::Function),
                                        "SCHEMA" => Some(PrivilegeObjectType::Schema),
                                        "DATABASE" => Some(PrivilegeObjectType::Database),
                                        _ => None,
                                    }
                                });
                                this.set_object_type_filter(object_type, cx);
                            })),
                    ),
            )
            .child(
                // Content
                if selected_role.is_none() {
                    div()
                        .flex_1()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            EmptyState::new("no-role-selected")
                                .icon(IconName::Users)
                                .title("No role selected")
                                .description("Select a role to view its privileges"),
                        )
                } else if privileges.is_empty() {
                    div()
                        .flex_1()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            EmptyState::new("no-privileges")
                                .icon(IconName::Shield)
                                .title("No privileges")
                                .description("This role has no explicit privileges"),
                        )
                } else {
                    ScrollView::new("privileges-scroll")
                        .flex_1()
                        .child(
                            Table::new("privileges-table")
                                .header(vec![
                                    TableColumn::new("Type").width(px(100.0)),
                                    TableColumn::new("Object").flex(1.0),
                                    TableColumn::new("Privileges").width(px(250.0)),
                                    TableColumn::new("Grant Option").width(px(100.0)).center(),
                                    TableColumn::new("Grantor").width(px(120.0)),
                                ])
                                .children(privileges.iter().enumerate().map(|(idx, privilege)| {
                                    let full_name = if let Some(ref schema) = privilege.schema {
                                        format!("{}.{}", schema, privilege.object_name)
                                    } else {
                                        privilege.object_name.clone()
                                    };

                                    TableRow::new(format!("privilege-{}", idx))
                                        .child(
                                            // Type column
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap_2()
                                                .child(Icon::new(
                                                    self.object_type_icon(privilege.object_type),
                                                ))
                                                .child(
                                                    div()
                                                        .text_sm()
                                                        .child(privilege.object_type.as_str()),
                                                ),
                                        )
                                        .child(
                                            // Object column
                                            div()
                                                .font_family("monospace")
                                                .text_sm()
                                                .child(full_name),
                                        )
                                        .child(
                                            // Privileges column
                                            self.render_privileges_badges(&privilege.privileges),
                                        )
                                        .child(
                                            // Grant option column
                                            div()
                                                .flex()
                                                .justify_center()
                                                .child(if privilege.grant_option {
                                                    Icon::new(IconName::Check)
                                                        .color(gpui::rgb(0x22c55e))
                                                } else {
                                                    Icon::new(IconName::X)
                                                        .color(gpui::rgb(0x9ca3af))
                                                }),
                                        )
                                        .child(
                                            // Grantor column
                                            div()
                                                .text_sm()
                                                .text_color(gpui::rgb(0x6b7280))
                                                .child(privilege.grantor.clone()),
                                        )
                                })),
                        )
                },
            )
    }
}
```

### 22.7 Delete Role Confirmation Dialog

```rust
// src/components/roles/delete_role_dialog.rs

use crate::models::role::Role;
use crate::state::role_state::RoleState;
use crate::ui::{
    Button, ButtonVariant, Checkbox, Input, Modal, ModalFooter, Section, Select,
    SelectOption,
};
use gpui::{
    div, px, AppContext, Context, Element, EventEmitter, FocusHandle, FocusableView,
    InteractiveElement, IntoElement, ParentElement, Render, SharedString, Styled,
    View, ViewContext, VisualContext,
};

/// Events emitted by the delete dialog
pub enum DeleteRoleEvent {
    Deleted,
    Cancelled,
}

pub struct DeleteRoleDialog {
    focus_handle: FocusHandle,
    role: Role,
    confirm_name: String,
    reassign_to: Option<String>,
    cascade: bool,
    deleting: bool,
    error: Option<String>,
    available_roles: Vec<String>,
}

impl EventEmitter<DeleteRoleEvent> for DeleteRoleDialog {}

impl DeleteRoleDialog {
    pub fn new(role: Role, cx: &mut ViewContext<Self>) -> Self {
        let role_state = cx.global::<RoleState>();
        let available_roles: Vec<String> = role_state
            .role_names()
            .into_iter()
            .filter(|r| r != &role.name)
            .collect();

        Self {
            focus_handle: cx.focus_handle(),
            role,
            confirm_name: String::new(),
            reassign_to: available_roles.first().cloned(),
            cascade: false,
            deleting: false,
            error: None,
            available_roles,
        }
    }

    fn can_delete(&self) -> bool {
        !self.deleting && self.confirm_name == self.role.name
    }

    fn handle_delete(&mut self, cx: &mut ViewContext<Self>) {
        if !self.can_delete() {
            return;
        }

        self.deleting = true;
        self.error = None;
        cx.notify();

        let role_state = cx.global::<RoleState>().clone();
        let role_name = self.role.name.clone();
        let cascade = self.cascade;
        let reassign_to = self.reassign_to.clone();

        cx.spawn(|this, mut cx| async move {
            let result = if cascade {
                if let Some(ref reassign) = reassign_to {
                    // Use cascade delete with reassignment
                    role_state
                        .drop_role(&role_name) // Would need drop_role_cascade method
                        .await
                } else {
                    Err(crate::services::role::RoleError::InvalidOperation(
                        "Must select a role to reassign objects to".to_string(),
                    ))
                }
            } else {
                role_state.drop_role(&role_name).await
            };

            match result {
                Ok(()) => {
                    this.update(&mut cx, |_, cx| {
                        cx.emit(DeleteRoleEvent::Deleted);
                    })
                    .ok();
                }
                Err(e) => {
                    this.update(&mut cx, |this, cx| {
                        this.deleting = false;
                        this.error = Some(e.to_string());
                        cx.notify();
                    })
                    .ok();
                }
            }
        })
        .detach();
    }

    fn handle_cancel(&mut self, cx: &mut ViewContext<Self>) {
        cx.emit(DeleteRoleEvent::Cancelled);
    }
}

impl FocusableView for DeleteRoleDialog {
    fn focus_handle(&self, _cx: &AppContext) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for DeleteRoleDialog {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let can_delete = self.can_delete();
        let has_members = !self.role.members.is_empty();
        let has_memberships = !self.role.member_of.is_empty();

        Modal::new("delete-role-dialog")
            .title(format!("Delete Role: {}", self.role.name))
            .width(px(500.0))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .p_4()
                    // Warning message
                    .child(
                        div()
                            .p_3()
                            .bg(gpui::rgb(0xfef3c7))
                            .border_1()
                            .border_color(gpui::rgb(0xfbbf24))
                            .rounded(px(6.0))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(gpui::rgb(0x92400e))
                                    .child("This action cannot be undone. The role will be permanently deleted."),
                            ),
                    )
                    // Error message
                    .when_some(self.error.clone(), |this, error| {
                        this.child(
                            div()
                                .p_3()
                                .bg(gpui::rgb(0xfef2f2))
                                .border_1()
                                .border_color(gpui::rgb(0xfecaca))
                                .rounded(px(6.0))
                                .text_sm()
                                .text_color(gpui::rgb(0xb91c1c))
                                .child(error),
                        )
                    })
                    // Role info
                    .when(has_members || has_memberships, |this| {
                        this.child(
                            div()
                                .p_3()
                                .bg(gpui::rgb(0xf3f4f6))
                                .rounded(px(6.0))
                                .flex()
                                .flex_col()
                                .gap_2()
                                .when(has_members, |this| {
                                    this.child(
                                        div()
                                            .text_sm()
                                            .child(format!(
                                                "Has {} member(s): {}",
                                                self.role.members.len(),
                                                self.role.members.join(", ")
                                            )),
                                    )
                                })
                                .when(has_memberships, |this| {
                                    this.child(
                                        div()
                                            .text_sm()
                                            .child(format!(
                                                "Member of {} role(s): {}",
                                                self.role.member_of.len(),
                                                self.role.member_of.join(", ")
                                            )),
                                    )
                                }),
                        )
                    })
                    // Cascade option
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                Checkbox::new("cascade")
                                    .checked(self.cascade)
                                    .label("Drop owned objects and reassign to another role")
                                    .on_toggle(cx.listener(|this, _, cx| {
                                        this.cascade = !this.cascade;
                                        cx.notify();
                                    })),
                            )
                            .when(self.cascade, |this| {
                                this.child(
                                    div()
                                        .pl_6()
                                        .child(
                                            Select::new("reassign-to")
                                                .label("Reassign owned objects to")
                                                .value(self.reassign_to.clone().map(SharedString::from))
                                                .options(
                                                    self.available_roles
                                                        .iter()
                                                        .map(|r| SelectOption::new(r.clone(), r.clone()))
                                                        .collect(),
                                                )
                                                .on_change(cx.listener(
                                                    |this, value: &Option<SharedString>, cx| {
                                                        this.reassign_to =
                                                            value.as_ref().map(|v| v.to_string());
                                                        cx.notify();
                                                    },
                                                )),
                                        ),
                                )
                            }),
                    )
                    // Confirmation input
                    .child(
                        Section::new("confirm-name")
                            .label(format!(
                                "Type \"{}\" to confirm deletion",
                                self.role.name
                            ))
                            .child(
                                Input::new("confirm-name")
                                    .value(self.confirm_name.clone())
                                    .placeholder(self.role.name.clone())
                                    .on_change(cx.listener(|this, value: &String, cx| {
                                        this.confirm_name = value.clone();
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
            .footer(
                ModalFooter::new().right(
                    div()
                        .flex()
                        .gap_2()
                        .child(
                            Button::new("cancel")
                                .label("Cancel")
                                .variant(ButtonVariant::Secondary)
                                .on_click(cx.listener(|this, _, cx| {
                                    this.handle_cancel(cx);
                                })),
                        )
                        .child(
                            Button::new("delete")
                                .label(if self.deleting {
                                    "Deleting..."
                                } else {
                                    "Delete Role"
                                })
                                .variant(ButtonVariant::Danger)
                                .disabled(!can_delete)
                                .on_click(cx.listener(|this, _, cx| {
                                    this.handle_delete(cx);
                                })),
                        ),
                ),
            )
    }
}
```

### 22.8 Role Management Panel (Main Container)

```rust
// src/components/roles/role_panel.rs

use crate::components::roles::{
    DeleteRoleDialog, DeleteRoleEvent, PrivilegesView, RoleEditorDialog, RoleEditorEvent,
    RoleEditorMode, RoleListEvent, RoleListView,
};
use crate::models::role::Role;
use crate::state::role_state::RoleState;
use crate::ui::{Panel, SplitView, SplitViewDirection};
use gpui::{
    div, AppContext, Context, Element, FocusHandle, FocusableView, IntoElement,
    ParentElement, Render, Styled, View, ViewContext, VisualContext,
};

pub struct RolePanel {
    focus_handle: FocusHandle,
    role_list: View<RoleListView>,
    privileges_view: View<PrivilegesView>,
    editor_dialog: Option<View<RoleEditorDialog>>,
    delete_dialog: Option<View<DeleteRoleDialog>>,
}

impl RolePanel {
    pub fn new(cx: &mut ViewContext<Self>) -> Self {
        // Initialize role state
        let role_state = RoleState::new();
        cx.set_global(role_state);

        // Create child views
        let role_list = cx.new_view(|cx| RoleListView::new(cx));
        let privileges_view = cx.new_view(|cx| PrivilegesView::new(cx));

        // Subscribe to role list events
        cx.subscribe(&role_list, Self::handle_role_list_event).detach();

        Self {
            focus_handle: cx.focus_handle(),
            role_list,
            privileges_view,
            editor_dialog: None,
            delete_dialog: None,
        }
    }

    /// Load roles when panel becomes active
    pub fn load_roles(&self, cx: &mut ViewContext<Self>) {
        let role_state = cx.global::<RoleState>().clone();

        cx.spawn(|this, mut cx| async move {
            if let Err(e) = role_state.load_roles().await {
                log::error!("Failed to load roles: {}", e);
            }

            if let Err(e) = role_state.load_memberships().await {
                log::error!("Failed to load memberships: {}", e);
            }

            this.update(&mut cx, |_, cx| cx.notify()).ok();
        })
        .detach();
    }

    fn handle_role_list_event(
        &mut self,
        _: View<RoleListView>,
        event: &RoleListEvent,
        cx: &mut ViewContext<Self>,
    ) {
        match event {
            RoleListEvent::CreateRole => {
                self.show_create_dialog(cx);
            }
            RoleListEvent::EditRole(name) => {
                self.show_edit_dialog(name.clone(), cx);
            }
            RoleListEvent::DeleteRole(name) => {
                self.show_delete_dialog(name.clone(), cx);
            }
            RoleListEvent::SelectRole(_) => {
                // Selection is handled by the role list and state
                cx.notify();
            }
        }
    }

    fn show_create_dialog(&mut self, cx: &mut ViewContext<Self>) {
        let editor = cx.new_view(|cx| RoleEditorDialog::new_create(cx));
        cx.subscribe(&editor, Self::handle_editor_event).detach();
        self.editor_dialog = Some(editor);
        cx.notify();
    }

    fn show_edit_dialog(&mut self, role_name: String, cx: &mut ViewContext<Self>) {
        let role_state = cx.global::<RoleState>();

        if let Some(role) = role_state.roles().into_iter().find(|r| r.name == role_name) {
            let editor = cx.new_view(|cx| RoleEditorDialog::new_edit(role, cx));
            cx.subscribe(&editor, Self::handle_editor_event).detach();
            self.editor_dialog = Some(editor);
            cx.notify();
        }
    }

    fn show_delete_dialog(&mut self, role_name: String, cx: &mut ViewContext<Self>) {
        let role_state = cx.global::<RoleState>();

        if let Some(role) = role_state.roles().into_iter().find(|r| r.name == role_name) {
            let dialog = cx.new_view(|cx| DeleteRoleDialog::new(role, cx));
            cx.subscribe(&dialog, Self::handle_delete_event).detach();
            self.delete_dialog = Some(dialog);
            cx.notify();
        }
    }

    fn handle_editor_event(
        &mut self,
        _: View<RoleEditorDialog>,
        event: &RoleEditorEvent,
        cx: &mut ViewContext<Self>,
    ) {
        match event {
            RoleEditorEvent::Saved | RoleEditorEvent::Cancelled => {
                self.editor_dialog = None;
                cx.notify();
            }
        }
    }

    fn handle_delete_event(
        &mut self,
        _: View<DeleteRoleDialog>,
        event: &DeleteRoleEvent,
        cx: &mut ViewContext<Self>,
    ) {
        match event {
            DeleteRoleEvent::Deleted | DeleteRoleEvent::Cancelled => {
                self.delete_dialog = None;
                cx.notify();
            }
        }
    }
}

impl FocusableView for RolePanel {
    fn focus_handle(&self, _cx: &AppContext) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for RolePanel {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        Panel::new("role-management")
            .title("Role Management")
            .child(
                SplitView::new("role-split")
                    .direction(SplitViewDirection::Horizontal)
                    .initial_ratio(0.6)
                    .min_size(300.0)
                    .left(self.role_list.clone())
                    .right(self.privileges_view.clone()),
            )
            // Editor dialog overlay
            .when_some(self.editor_dialog.clone(), |this, editor| {
                this.child(editor)
            })
            // Delete dialog overlay
            .when_some(self.delete_dialog.clone(), |this, dialog| {
                this.child(dialog)
            })
    }
}
```

## Acceptance Criteria

1. **Role Listing**
   - [ ] Display all roles with attributes in a sortable table
   - [ ] Show role badges (superuser, login, etc.) with distinct colors
   - [ ] Filter by name and login capability
   - [ ] Display role memberships with overflow handling
   - [ ] Handle empty state gracefully

2. **Role Creation**
   - [ ] Create roles with all PostgreSQL options
   - [ ] Set password with confirmation and validation
   - [ ] Configure all privilege flags via checkboxes
   - [ ] Set connection limit with numeric input
   - [ ] Set expiration date with date picker
   - [ ] Assign initial role memberships
   - [ ] Preview generated SQL before execution

3. **Role Editing**
   - [ ] Modify all role attributes
   - [ ] Change password optionally (leave empty to keep)
   - [ ] Add/remove role memberships with diff tracking
   - [ ] Preview ALTER statements
   - [ ] Handle rename separately if needed

4. **Role Deletion**
   - [ ] Require typing role name for confirmation
   - [ ] Option to cascade with object reassignment
   - [ ] Handle dependent objects warning
   - [ ] Show role relationships before deletion

5. **Privilege Management**
   - [ ] View privileges for selected role
   - [ ] Display object-level permissions with icons
   - [ ] Filter by object type
   - [ ] Show grant option status
   - [ ] Display grantor information

6. **State Management**
   - [ ] Use GPUI Global trait for RoleState
   - [ ] Thread-safe access with parking_lot::RwLock
   - [ ] Automatic refresh after mutations
   - [ ] Loading and error states

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_create_role_sql_basic() {
        let options = RoleCreateOptions::new("test_user").with_login();
        let sql = RoleService::build_create_role_sql(&options);

        assert!(sql.contains("CREATE ROLE test_user"));
        assert!(sql.contains("LOGIN"));
        assert!(sql.contains("NOSUPERUSER"));
    }

    #[test]
    fn test_build_create_role_sql_full() {
        let options = RoleCreateOptions {
            name: "admin_user".to_string(),
            password: Some("secret123".to_string()),
            superuser: true,
            createdb: true,
            createrole: true,
            inherit: true,
            login: true,
            replication: false,
            bypassrls: true,
            connection_limit: 10,
            valid_until: None,
            in_roles: vec!["admins".to_string()],
            roles: vec![],
            admin_roles: vec![],
        };

        let sql = RoleService::build_create_role_sql(&options);

        assert!(sql.contains("SUPERUSER"));
        assert!(sql.contains("CREATEDB"));
        assert!(sql.contains("CREATEROLE"));
        assert!(sql.contains("BYPASSRLS"));
        assert!(sql.contains("CONNECTION LIMIT 10"));
        assert!(sql.contains("ENCRYPTED PASSWORD"));
        assert!(sql.contains("IN ROLE admins"));
    }

    #[test]
    fn test_build_alter_role_sql() {
        let options = RoleAlterOptions {
            superuser: Some(false),
            login: Some(true),
            connection_limit: Some(5),
            ..Default::default()
        };

        let sql = RoleService::build_alter_role_sql("test_user", &options);

        assert!(sql.is_some());
        let sql = sql.unwrap();
        assert!(sql.contains("ALTER ROLE test_user"));
        assert!(sql.contains("NOSUPERUSER"));
        assert!(sql.contains("LOGIN"));
        assert!(sql.contains("CONNECTION LIMIT 5"));
    }

    #[test]
    fn test_build_alter_role_sql_no_changes() {
        let options = RoleAlterOptions::default();
        let sql = RoleService::build_alter_role_sql("test_user", &options);
        assert!(sql.is_none());
    }

    #[test]
    fn test_quote_ident() {
        assert_eq!(quote_ident("simple"), "simple");
        assert_eq!(quote_ident("has space"), "\"has space\"");
        assert_eq!(quote_ident("has\"quote"), "\"has\"\"quote\"");
        assert_eq!(quote_ident("UPPER"), "\"UPPER\"");
        assert_eq!(quote_ident("123start"), "\"123start\"");
    }

    #[test]
    fn test_parse_privilege_type() {
        assert_eq!(parse_privilege_type("SELECT"), Some(PrivilegeType::Select));
        assert_eq!(parse_privilege_type("select"), Some(PrivilegeType::Select));
        assert_eq!(parse_privilege_type("TEMPORARY"), Some(PrivilegeType::Temporary));
        assert_eq!(parse_privilege_type("TEMP"), Some(PrivilegeType::Temporary));
        assert_eq!(parse_privilege_type("INVALID"), None);
    }
}
```

### Integration Tests

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use gpui::TestAppContext;

    #[gpui::test]
    async fn test_role_list_filtering(cx: &mut TestAppContext) {
        // Initialize test state with sample roles
        cx.update(|cx| {
            let state = RoleState::new();
            cx.set_global(state);
        });

        let view = cx.new_view(|cx| RoleListView::new(cx));

        // Set filter
        view.update(cx, |view, cx| {
            view.set_filter("admin".to_string(), cx);
        });

        // Verify filtered results
        view.update(cx, |view, _| {
            assert!(view.filter_text == "admin");
        });
    }

    #[gpui::test]
    async fn test_role_editor_validation(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let state = RoleState::new();
            cx.set_global(state);
        });

        let view = cx.new_view(|cx| RoleEditorDialog::new_create(cx));

        // Test password mismatch
        view.update(cx, |view, cx| {
            view.password = "password1".to_string();
            view.confirm_password = "password2".to_string();
            cx.notify();
        });

        view.update(cx, |view, _| {
            assert!(view.password_error().is_some());
            assert!(!view.can_save());
        });

        // Fix password match
        view.update(cx, |view, cx| {
            view.confirm_password = "password1".to_string();
            view.name = "new_role".to_string();
            cx.notify();
        });

        view.update(cx, |view, _| {
            assert!(view.password_error().is_none());
            assert!(view.can_save());
        });
    }
}
```

### Database Integration Tests

```rust
#[cfg(test)]
mod db_tests {
    use super::*;

    #[tokio::test]
    async fn test_role_crud_operations() {
        let pool = create_test_pool().await;

        // Create role
        let create_options = RoleCreateOptions::new("test_crud_role")
            .with_login()
            .with_password("test_password");

        RoleService::create_role(&pool, &create_options)
            .await
            .expect("Failed to create role");

        // Verify creation
        let roles = RoleService::get_roles(&pool).await.expect("Failed to get roles");
        assert!(roles.iter().any(|r| r.name == "test_crud_role"));

        // Alter role
        let alter_options = RoleAlterOptions {
            createdb: Some(true),
            ..Default::default()
        };

        RoleService::alter_role(&pool, "test_crud_role", &alter_options)
            .await
            .expect("Failed to alter role");

        // Verify alteration
        let role = RoleService::get_role(&pool, "test_crud_role")
            .await
            .expect("Failed to get role");
        assert!(role.can_create_db);

        // Drop role
        RoleService::drop_role(&pool, "test_crud_role")
            .await
            .expect("Failed to drop role");

        // Verify deletion
        let roles = RoleService::get_roles(&pool).await.expect("Failed to get roles");
        assert!(!roles.iter().any(|r| r.name == "test_crud_role"));
    }

    #[tokio::test]
    async fn test_role_membership() {
        let pool = create_test_pool().await;

        // Create two roles
        RoleService::create_role(
            &pool,
            &RoleCreateOptions::new("test_parent_role"),
        )
        .await
        .expect("Failed to create parent role");

        RoleService::create_role(
            &pool,
            &RoleCreateOptions::new("test_child_role"),
        )
        .await
        .expect("Failed to create child role");

        // Grant membership
        RoleService::grant_role(&pool, "test_parent_role", "test_child_role", false)
            .await
            .expect("Failed to grant role");

        // Verify membership
        let child = RoleService::get_role(&pool, "test_child_role")
            .await
            .expect("Failed to get role");
        assert!(child.member_of.contains(&"test_parent_role".to_string()));

        // Revoke membership
        RoleService::revoke_role(&pool, "test_parent_role", "test_child_role")
            .await
            .expect("Failed to revoke role");

        // Cleanup
        RoleService::drop_role(&pool, "test_child_role").await.ok();
        RoleService::drop_role(&pool, "test_parent_role").await.ok();
    }
}
```
