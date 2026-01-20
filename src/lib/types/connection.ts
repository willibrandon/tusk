/**
 * Connection-related type definitions for Tusk frontend architecture.
 *
 * @module contracts/connection
 */

// =============================================================================
// Connection Types
// =============================================================================

/**
 * SSL/TLS mode for database connections.
 */
export type SslMode = 'disable' | 'prefer' | 'require' | 'verify-ca' | 'verify-full';

/**
 * Database connection configuration.
 *
 * Note: Passwords are NOT stored in this interface. They are stored in the
 * OS keychain and retrieved via the Rust backend when needed.
 */
export interface Connection {
  /** Unique identifier (UUID v4) */
  id: string;

  /** User-defined connection name */
  name: string;

  /** Database server hostname */
  host: string;

  /** Database server port */
  port: number;

  /** Database name */
  database: string;

  /** Database username */
  username: string;

  /** SSL/TLS configuration */
  sslMode: SslMode;

  /** Color for visual identification (hex format, e.g., "#3b82f6") */
  color: string | null;

  /** Parent connection group ID */
  groupId: string | null;

  /** Sort order within group */
  sortOrder: number;

  /** SSH tunnel configuration (future feature) */
  sshTunnel: SshTunnelConfig | null;
}

/**
 * SSH tunnel configuration for remote database access.
 */
export interface SshTunnelConfig {
  /** SSH server hostname */
  host: string;

  /** SSH server port */
  port: number;

  /** SSH username */
  username: string;

  /** Authentication method */
  authMethod: 'password' | 'key';

  /** Path to private key file (if authMethod is 'key') */
  privateKeyPath: string | null;
}

// =============================================================================
// Connection Group Types
// =============================================================================

/**
 * Connection group for organizing connections hierarchically.
 */
export interface ConnectionGroup {
  /** Unique identifier (UUID v4) */
  id: string;

  /** Group display name */
  name: string;

  /** Parent group ID (null for root level) */
  parentId: string | null;

  /** Sort order within parent */
  sortOrder: number;

  /** Group color (hex format) */
  color: string | null;

  /** Whether the group is expanded in the tree view */
  isExpanded: boolean;
}

// =============================================================================
// Connection Status Types
// =============================================================================

/**
 * Connection state enumeration.
 */
export type ConnectionState = 'disconnected' | 'connecting' | 'connected' | 'error';

/**
 * Current status of a database connection.
 */
export interface ConnectionStatus {
  /** Associated connection ID */
  connectionId: string;

  /** Current connection state */
  state: ConnectionState;

  /** Error message (only when state is 'error') */
  error: string | null;

  /** Unix timestamp when connection was established */
  connectedAt: number | null;

  /** Server version string (when connected) */
  serverVersion: string | null;
}

// =============================================================================
// Connection Store Interface
// =============================================================================

/**
 * Connection store interface for state management.
 */
export interface ConnectionStoreInterface {
  /** All configured connections */
  readonly connections: Connection[];

  /** All connection groups */
  readonly groups: ConnectionGroup[];

  /** Currently active connection ID */
  readonly activeConnectionId: string | null;

  /** Currently active connection */
  readonly activeConnection: Connection | null;

  /** Connection status map */
  readonly connectionStatuses: Map<string, ConnectionStatus>;

  /** Get status for a specific connection */
  getStatus(connectionId: string): ConnectionStatus | undefined;

  /** Get a specific connection by ID */
  getConnection(connectionId: string): Connection | undefined;

  /** Set all connections (from backend) */
  setConnections(connections: Connection[]): void;

  /** Set all groups (from backend) */
  setGroups(groups: ConnectionGroup[]): void;

  /** Set the active connection */
  setActiveConnection(id: string | null): void;

  /** Update connection status */
  updateStatus(connectionId: string, status: Partial<ConnectionStatus>): void;

  /** Toggle group expansion */
  toggleGroup(groupId: string): void;
}

// =============================================================================
// Connection Display Helpers
// =============================================================================

/**
 * Format a connection for display in the status bar.
 */
export interface ConnectionDisplayInfo {
  /** Formatted connection string (e.g., "user@host:port/db") */
  connectionString: string;

  /** Short display name */
  displayName: string;

  /** Status color based on connection state */
  statusColor: 'green' | 'yellow' | 'red' | 'gray';

  /** Human-readable status text */
  statusText: string;
}

/**
 * Get display information for a connection.
 */
export function getConnectionDisplayInfo(
  connection: Connection | null,
  status: ConnectionStatus | null
): ConnectionDisplayInfo {
  if (!connection) {
    return {
      connectionString: '',
      displayName: 'No connection',
      statusColor: 'gray',
      statusText: 'Disconnected',
    };
  }

  const connectionString = `${connection.username}@${connection.host}:${connection.port}/${connection.database}`;

  if (!status) {
    return {
      connectionString,
      displayName: connection.name,
      statusColor: 'gray',
      statusText: 'Disconnected',
    };
  }

  const statusMap: Record<ConnectionState, { color: 'green' | 'yellow' | 'red' | 'gray'; text: string }> = {
    disconnected: { color: 'gray', text: 'Disconnected' },
    connecting: { color: 'yellow', text: 'Connecting...' },
    connected: { color: 'green', text: 'Connected' },
    error: { color: 'red', text: status.error ?? 'Connection error' },
  };

  const { color, text } = statusMap[status.state];

  return {
    connectionString,
    displayName: connection.name,
    statusColor: color,
    statusText: text,
  };
}

// =============================================================================
// Type Guards
// =============================================================================

/**
 * Check if a value is a valid SslMode.
 */
export function isSslMode(value: unknown): value is SslMode {
  return (
    typeof value === 'string' &&
    ['disable', 'prefer', 'require', 'verify-ca', 'verify-full'].includes(value)
  );
}

/**
 * Check if a value is a valid ConnectionState.
 */
export function isConnectionState(value: unknown): value is ConnectionState {
  return (
    typeof value === 'string' &&
    ['disconnected', 'connecting', 'connected', 'error'].includes(value)
  );
}

/**
 * Check if a value is a valid hex color.
 */
export function isHexColor(value: unknown): value is string {
  return typeof value === 'string' && /^#[0-9a-f]{6}$/i.test(value);
}
