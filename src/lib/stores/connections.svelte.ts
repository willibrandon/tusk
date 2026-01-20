/**
 * Connections store for managing database connections.
 *
 * This store provides the frontend state management for database connections.
 * Backend integration is implemented in docs/features/07-connection-management.md.
 *
 * @module stores/connections
 */

import type {
	Connection,
	ConnectionGroup,
	ConnectionStatus,
	ConnectionStoreInterface
} from '$lib/types';

/**
 * Create the connections store with Svelte 5 runes pattern.
 */
function createConnectionsStore(): ConnectionStoreInterface {
	// Initialize state - empty until backend provides data
	let connections = $state<Connection[]>([]);
	let groups = $state<ConnectionGroup[]>([]);
	let activeConnectionId = $state<string | null>(null);
	let connectionStatuses = $state<Map<string, ConnectionStatus>>(new Map());

	// Derived values
	const activeConnection = $derived(connections.find((c) => c.id === activeConnectionId) ?? null);

	return {
		get connections() {
			return connections;
		},

		get groups() {
			return groups;
		},

		get activeConnectionId() {
			return activeConnectionId;
		},

		get activeConnection() {
			return activeConnection;
		},

		get connectionStatuses() {
			return connectionStatuses;
		},

		getStatus(connectionId: string): ConnectionStatus | undefined {
			return connectionStatuses.get(connectionId);
		},

		getConnection(connectionId: string): Connection | undefined {
			return connections.find((c) => c.id === connectionId);
		},

		setConnections(newConnections: Connection[]) {
			connections = newConnections;
		},

		setGroups(newGroups: ConnectionGroup[]) {
			groups = newGroups;
		},

		setActiveConnection(id: string | null) {
			if (id === null || connections.some((c) => c.id === id)) {
				activeConnectionId = id;
			}
		},

		updateStatus(connectionId: string, status: Partial<ConnectionStatus>) {
			const existing = connectionStatuses.get(connectionId);
			const newStatus: ConnectionStatus = {
				connectionId,
				state: status.state ?? existing?.state ?? 'disconnected',
				error: status.error ?? existing?.error ?? null,
				connectedAt: status.connectedAt ?? existing?.connectedAt ?? null,
				serverVersion: status.serverVersion ?? existing?.serverVersion ?? null
			};
			connectionStatuses = new Map(connectionStatuses).set(connectionId, newStatus);
		},

		toggleGroup(groupId: string) {
			const index = groups.findIndex((g) => g.id === groupId);
			if (index !== -1) {
				groups = groups.map((g, i) => (i === index ? { ...g, isExpanded: !g.isExpanded } : g));
			}
		}
	};
}

export const connectionStore = createConnectionsStore();
