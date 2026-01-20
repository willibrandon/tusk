/**
 * Type contracts for Tusk frontend architecture.
 *
 * This module re-exports all type definitions used by the frontend.
 *
 * @module contracts
 */

// Tab types
export type {
  TabType,
  Tab,
  TabContent,
  QueryTabContent,
  TableTabContent,
  ViewTabContent,
  FunctionTabContent,
  SchemaTabContent,
  CursorPosition,
  SelectionRange,
  QueryResult,
  ColumnDefinition,
  ColumnFilter,
  FilterOperator,
  CloseResult,
  CreateTabOptions,
  TabStoreInterface,
} from './tab';

export { isTabType, isQueryContent, isTableContent } from './tab';

// Connection types
export type {
  SslMode,
  Connection,
  SshTunnelConfig,
  ConnectionGroup,
  ConnectionState,
  ConnectionStatus,
  ConnectionStoreInterface,
  ConnectionDisplayInfo,
} from './connection';

export { getConnectionDisplayInfo, isSslMode, isConnectionState, isHexColor } from './connection';

// UI types
export type {
  ThemeMode,
  ThemePreference,
  ThemeState,
  ThemeStoreInterface,
  UIState,
  UIStoreInterface,
  ConfirmDialogResult,
  UnsavedChangesResult,
  DialogState,
  UnsavedChangesDialogData,
  KeyboardShortcut,
  ResizeDirection,
  ResizeState,
  TabDragState,
  StatusBarInfo,
} from './ui';

export {
  SIDEBAR_MIN_WIDTH,
  SIDEBAR_MAX_WIDTH,
  SIDEBAR_DEFAULT_WIDTH,
  RESULTS_MIN_HEIGHT,
  RESULTS_MAX_HEIGHT,
  RESULTS_DEFAULT_HEIGHT,
  DEFAULT_UI_STATE,
  INITIAL_TAB_DRAG_STATE,
  formatShortcut,
  isThemePreference,
  isThemeMode,
  isUIState,
  clamp,
  clampSidebarWidth,
  clampResultsHeight,
} from './ui';
