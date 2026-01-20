# Feature 12: Monaco Editor Integration

## Overview

Monaco Editor provides the SQL editing experience in Tusk. This feature integrates the Monaco editor with schema-aware autocomplete, syntax highlighting for Postgres-specific keywords, error highlighting at exact positions, and full keyboard shortcut support.

## Goals

- Integrate Monaco Editor for SQL editing
- Implement schema-aware autocomplete (tables, columns, functions, keywords)
- Provide Postgres-specific syntax highlighting
- Show error squiggles at positions returned by Postgres
- Support code folding for CTEs and subqueries
- Enable multi-cursor editing and all standard Monaco features

## Dependencies

- Feature 03: Frontend Architecture (Svelte component structure)
- Feature 10: Schema Introspection (schema metadata for autocomplete)
- Feature 11: Query Execution (error position information)

## Technical Specification

### 12.1 Monaco Setup and Configuration

```typescript
// src/lib/components/editor/monacoSetup.ts

import * as monaco from 'monaco-editor';
import editorWorker from 'monaco-editor/esm/vs/editor/editor.worker?worker';
import { postgresLanguage, postgresTheme } from './postgresLanguage';

// Configure Monaco workers
self.MonacoEnvironment = {
  getWorker(_: any, label: string) {
    return new editorWorker();
  }
};

let initialized = false;

export function initializeMonaco() {
  if (initialized) return;
  initialized = true;

  // Register PostgreSQL language
  monaco.languages.register({ id: 'postgresql' });

  // Set language configuration
  monaco.languages.setLanguageConfiguration('postgresql', {
    comments: {
      lineComment: '--',
      blockComment: ['/*', '*/'],
    },
    brackets: [
      ['{', '}'],
      ['[', ']'],
      ['(', ')'],
    ],
    autoClosingPairs: [
      { open: '{', close: '}' },
      { open: '[', close: ']' },
      { open: '(', close: ')' },
      { open: "'", close: "'", notIn: ['string'] },
      { open: '"', close: '"', notIn: ['string'] },
      { open: '$$', close: '$$' },
    ],
    surroundingPairs: [
      { open: '{', close: '}' },
      { open: '[', close: ']' },
      { open: '(', close: ')' },
      { open: "'", close: "'" },
      { open: '"', close: '"' },
    ],
    folding: {
      markers: {
        start: /^\s*--\s*#?region\b/,
        end: /^\s*--\s*#?endregion\b/,
      },
    },
    wordPattern: /(-?\d*\.\d\w*)|([^\`\~\!\@\#\%\^\&\*\(\)\-\=\+\[\{\]\}\\\|\;\:\'\"\,\.\<\>\/\?\s]+)/g,
  });

  // Set monarch tokenizer
  monaco.languages.setMonarchTokensProvider('postgresql', postgresLanguage);

  // Register themes
  monaco.editor.defineTheme('tusk-light', {
    base: 'vs',
    inherit: true,
    rules: postgresTheme.light,
    colors: {
      'editor.background': '#ffffff',
      'editor.foreground': '#1f2937',
    },
  });

  monaco.editor.defineTheme('tusk-dark', {
    base: 'vs-dark',
    inherit: true,
    rules: postgresTheme.dark,
    colors: {
      'editor.background': '#1f2937',
      'editor.foreground': '#f3f4f6',
    },
  });
}

export function getEditorOptions(readOnly: boolean = false): monaco.editor.IStandaloneEditorConstructionOptions {
  return {
    language: 'postgresql',
    theme: 'tusk-light',
    automaticLayout: true,
    minimap: { enabled: false },
    scrollBeyondLastLine: false,
    lineNumbers: 'on',
    glyphMargin: true,
    folding: true,
    foldingStrategy: 'auto',
    wordWrap: 'off',
    wrappingStrategy: 'advanced',
    tabSize: 2,
    insertSpaces: true,
    formatOnPaste: false,
    formatOnType: false,
    autoIndent: 'advanced',
    suggestOnTriggerCharacters: true,
    quickSuggestions: {
      other: true,
      comments: false,
      strings: false,
    },
    acceptSuggestionOnEnter: 'smart',
    tabCompletion: 'on',
    wordBasedSuggestions: 'off',
    parameterHints: { enabled: true },
    bracketPairColorization: { enabled: true },
    matchBrackets: 'always',
    renderLineHighlight: 'all',
    cursorStyle: 'line',
    cursorBlinking: 'smooth',
    smoothScrolling: true,
    mouseWheelZoom: true,
    contextmenu: true,
    readOnly,
    domReadOnly: readOnly,
  };
}
```

### 12.2 PostgreSQL Language Definition

```typescript
// src/lib/components/editor/postgresLanguage.ts

import type * as monaco from 'monaco-editor';

// PostgreSQL keywords
const keywords = [
  // Standard SQL
  'SELECT', 'FROM', 'WHERE', 'AND', 'OR', 'NOT', 'IN', 'EXISTS', 'BETWEEN',
  'LIKE', 'ILIKE', 'IS', 'NULL', 'TRUE', 'FALSE', 'AS', 'ON', 'JOIN',
  'LEFT', 'RIGHT', 'INNER', 'OUTER', 'FULL', 'CROSS', 'NATURAL',
  'UNION', 'INTERSECT', 'EXCEPT', 'ALL', 'DISTINCT', 'ORDER', 'BY',
  'ASC', 'DESC', 'NULLS', 'FIRST', 'LAST', 'LIMIT', 'OFFSET', 'FETCH',
  'GROUP', 'HAVING', 'WINDOW', 'PARTITION', 'OVER', 'ROWS', 'RANGE',
  'UNBOUNDED', 'PRECEDING', 'FOLLOWING', 'CURRENT', 'ROW',

  // DML
  'INSERT', 'INTO', 'VALUES', 'UPDATE', 'SET', 'DELETE', 'TRUNCATE',
  'MERGE', 'UPSERT', 'RETURNING', 'ON CONFLICT', 'DO', 'NOTHING',

  // DDL
  'CREATE', 'ALTER', 'DROP', 'TABLE', 'VIEW', 'INDEX', 'SEQUENCE',
  'SCHEMA', 'DATABASE', 'FUNCTION', 'PROCEDURE', 'TRIGGER', 'TYPE',
  'DOMAIN', 'EXTENSION', 'MATERIALIZED', 'TEMPORARY', 'TEMP', 'UNLOGGED',
  'IF', 'EXISTS', 'CASCADE', 'RESTRICT', 'ADD', 'COLUMN', 'RENAME', 'TO',

  // Constraints
  'PRIMARY', 'KEY', 'FOREIGN', 'REFERENCES', 'UNIQUE', 'CHECK', 'DEFAULT',
  'CONSTRAINT', 'DEFERRABLE', 'INITIALLY', 'DEFERRED', 'IMMEDIATE',

  // PostgreSQL specific
  'WITH', 'RECURSIVE', 'LATERAL', 'TABLESAMPLE', 'ORDINALITY',
  'EXCLUDE', 'INCLUDING', 'ONLY', 'INHERITS', 'LIKE', 'STORAGE',
  'TABLESPACE', 'USING', 'CONCURRENTLY', 'NOWAIT', 'SKIP', 'LOCKED',

  // Transactions
  'BEGIN', 'COMMIT', 'ROLLBACK', 'SAVEPOINT', 'RELEASE', 'TRANSACTION',
  'ISOLATION', 'LEVEL', 'READ', 'WRITE', 'COMMITTED', 'UNCOMMITTED',
  'REPEATABLE', 'SERIALIZABLE',

  // PL/pgSQL
  'DECLARE', 'RETURN', 'RETURNS', 'LANGUAGE', 'PLPGSQL', 'SQL',
  'VOLATILE', 'STABLE', 'IMMUTABLE', 'STRICT', 'PARALLEL', 'SAFE',
  'SECURITY', 'DEFINER', 'INVOKER', 'LEAKPROOF', 'COST', 'SUPPORT',

  // Admin
  'GRANT', 'REVOKE', 'VACUUM', 'ANALYZE', 'REINDEX', 'CLUSTER',
  'EXPLAIN', 'COPY', 'LISTEN', 'NOTIFY', 'PREPARE', 'EXECUTE',
  'DEALLOCATE', 'LOCK', 'COMMENT', 'REFRESH',

  // Types
  'CAST', 'ARRAY', 'ROW', 'RECORD', 'SETOF', 'VARIADIC', 'OUT', 'INOUT',
];

// PostgreSQL data types
const typeKeywords = [
  'INTEGER', 'INT', 'INT2', 'INT4', 'INT8', 'SMALLINT', 'BIGINT',
  'DECIMAL', 'NUMERIC', 'REAL', 'FLOAT', 'FLOAT4', 'FLOAT8',
  'DOUBLE', 'PRECISION', 'MONEY', 'SERIAL', 'BIGSERIAL', 'SMALLSERIAL',
  'BOOLEAN', 'BOOL', 'BIT', 'VARBIT',
  'CHAR', 'CHARACTER', 'VARCHAR', 'TEXT', 'NAME', 'BYTEA',
  'DATE', 'TIME', 'TIMESTAMP', 'TIMESTAMPTZ', 'TIMETZ', 'INTERVAL',
  'UUID', 'JSON', 'JSONB', 'XML', 'CIDR', 'INET', 'MACADDR',
  'POINT', 'LINE', 'LSEG', 'BOX', 'PATH', 'POLYGON', 'CIRCLE',
  'TSVECTOR', 'TSQUERY', 'REGCLASS', 'REGTYPE', 'OID', 'VOID',
  'INT4RANGE', 'INT8RANGE', 'NUMRANGE', 'TSRANGE', 'TSTZRANGE', 'DATERANGE',
];

// PostgreSQL built-in functions
const builtinFunctions = [
  // Aggregate
  'COUNT', 'SUM', 'AVG', 'MIN', 'MAX', 'ARRAY_AGG', 'STRING_AGG',
  'JSONB_AGG', 'JSONB_OBJECT_AGG', 'BOOL_AND', 'BOOL_OR', 'BIT_AND', 'BIT_OR',

  // String
  'LENGTH', 'LOWER', 'UPPER', 'TRIM', 'LTRIM', 'RTRIM', 'CONCAT', 'CONCAT_WS',
  'SUBSTRING', 'SUBSTR', 'LEFT', 'RIGHT', 'POSITION', 'STRPOS', 'REPLACE',
  'SPLIT_PART', 'REGEXP_REPLACE', 'REGEXP_MATCH', 'REGEXP_MATCHES',
  'FORMAT', 'QUOTE_LITERAL', 'QUOTE_IDENT', 'QUOTE_NULLABLE',
  'ENCODE', 'DECODE', 'MD5', 'SHA256', 'SHA512',

  // Numeric
  'ABS', 'CEIL', 'CEILING', 'FLOOR', 'ROUND', 'TRUNC', 'MOD', 'POWER', 'SQRT',
  'RANDOM', 'SETSEED', 'SIGN', 'LOG', 'LN', 'EXP', 'PI', 'DEGREES', 'RADIANS',

  // Date/Time
  'NOW', 'CURRENT_DATE', 'CURRENT_TIME', 'CURRENT_TIMESTAMP', 'LOCALTIME',
  'LOCALTIMESTAMP', 'CLOCK_TIMESTAMP', 'STATEMENT_TIMESTAMP', 'TRANSACTION_TIMESTAMP',
  'DATE_PART', 'DATE_TRUNC', 'EXTRACT', 'AGE', 'MAKE_DATE', 'MAKE_TIME',
  'MAKE_TIMESTAMP', 'MAKE_TIMESTAMPTZ', 'MAKE_INTERVAL', 'TO_TIMESTAMP',

  // JSON
  'JSON_BUILD_OBJECT', 'JSON_BUILD_ARRAY', 'JSONB_BUILD_OBJECT', 'JSONB_BUILD_ARRAY',
  'JSON_OBJECT', 'JSON_ARRAY', 'JSONB_SET', 'JSONB_INSERT', 'JSONB_DELETE_PATH',
  'JSONB_PATH_QUERY', 'JSONB_PATH_EXISTS', 'JSONB_PRETTY', 'TO_JSON', 'TO_JSONB',

  // Array
  'ARRAY_LENGTH', 'ARRAY_DIMS', 'ARRAY_LOWER', 'ARRAY_UPPER', 'ARRAY_CAT',
  'ARRAY_APPEND', 'ARRAY_PREPEND', 'ARRAY_REMOVE', 'ARRAY_REPLACE', 'ARRAY_POSITION',
  'UNNEST', 'GENERATE_SERIES', 'GENERATE_SUBSCRIPTS',

  // Conditional
  'COALESCE', 'NULLIF', 'GREATEST', 'LEAST', 'CASE', 'WHEN', 'THEN', 'ELSE', 'END',

  // System
  'CURRENT_USER', 'CURRENT_SCHEMA', 'CURRENT_DATABASE', 'CURRENT_CATALOG',
  'SESSION_USER', 'USER', 'VERSION', 'PG_BACKEND_PID', 'PG_TYPEOF',
  'PG_GET_TABLEDEF', 'PG_GET_VIEWDEF', 'PG_GET_INDEXDEF', 'PG_GET_CONSTRAINTDEF',
  'PG_RELATION_SIZE', 'PG_TOTAL_RELATION_SIZE', 'PG_SIZE_PRETTY',

  // Window
  'ROW_NUMBER', 'RANK', 'DENSE_RANK', 'PERCENT_RANK', 'CUME_DIST',
  'NTILE', 'LAG', 'LEAD', 'FIRST_VALUE', 'LAST_VALUE', 'NTH_VALUE',
];

// Operators
const operators = [
  '=', '<>', '!=', '<', '>', '<=', '>=',
  '+', '-', '*', '/', '%', '^', '||',
  '&', '|', '#', '~', '<<', '>>',
  '->', '->>', '#>', '#>>', '@>', '<@', '?', '?|', '?&',
  '@@', '@@@', '!!', '~*', '!~', '!~*',
];

export const postgresLanguage: monaco.languages.IMonarchLanguage = {
  defaultToken: '',
  tokenPostfix: '.sql',
  ignoreCase: true,

  keywords,
  typeKeywords,
  builtinFunctions,
  operators,

  brackets: [
    { open: '[', close: ']', token: 'delimiter.bracket' },
    { open: '(', close: ')', token: 'delimiter.parenthesis' },
    { open: '{', close: '}', token: 'delimiter.curly' },
  ],

  tokenizer: {
    root: [
      // Whitespace
      { include: '@whitespace' },

      // Comments
      { include: '@comments' },

      // Dollar-quoted strings
      [/\$([a-zA-Z_][a-zA-Z0-9_]*)?\$/, { token: 'string.quote', next: '@dollarString.$1' }],

      // Strings
      [/'/, { token: 'string.quote', next: '@string' }],

      // Identifiers
      [/"/, { token: 'identifier.quote', next: '@quotedIdentifier' }],

      // Numbers
      [/\d*\.\d+([eE][\-+]?\d+)?/, 'number.float'],
      [/\d+/, 'number'],

      // Operators
      [/[<>=!]+/, 'operator'],
      [/[\-+*/%^|&#~@?]/, 'operator'],
      [/::/, 'operator.cast'],

      // Delimiters
      [/[;,.]/, 'delimiter'],
      [/[\[\]()]/, '@brackets'],

      // Keywords and identifiers
      [/[a-zA-Z_][a-zA-Z0-9_]*/, {
        cases: {
          '@keywords': 'keyword',
          '@typeKeywords': 'type',
          '@builtinFunctions': 'predefined',
          '@default': 'identifier',
        },
      }],

      // Parameters
      [/\$\d+/, 'variable'],
      [/:[a-zA-Z_][a-zA-Z0-9_]*/, 'variable.named'],
    ],

    whitespace: [
      [/\s+/, 'white'],
    ],

    comments: [
      [/--.*$/, 'comment'],
      [/\/\*/, { token: 'comment', next: '@blockComment' }],
    ],

    blockComment: [
      [/[^/*]+/, 'comment'],
      [/\/\*/, { token: 'comment', next: '@push' }],
      [/\*\//, { token: 'comment', next: '@pop' }],
      [/[/*]/, 'comment'],
    ],

    string: [
      [/[^']+/, 'string'],
      [/''/, 'string.escape'],
      [/'/, { token: 'string.quote', next: '@pop' }],
    ],

    quotedIdentifier: [
      [/[^"]+/, 'identifier'],
      [/""/, 'identifier.escape'],
      [/"/, { token: 'identifier.quote', next: '@pop' }],
    ],

    dollarString: [
      [/[^$]+/, 'string'],
      [/\$([a-zA-Z_][a-zA-Z0-9_]*)?\$/, {
        cases: {
          '$1==$S2': { token: 'string.quote', next: '@pop' },
          '@default': 'string',
        },
      }],
      [/\$/, 'string'],
    ],
  },
};

export const postgresTheme = {
  light: [
    { token: 'keyword', foreground: '7C3AED', fontStyle: 'bold' },
    { token: 'type', foreground: '0891B2' },
    { token: 'predefined', foreground: 'EA580C' },
    { token: 'string', foreground: '16A34A' },
    { token: 'number', foreground: 'DC2626' },
    { token: 'comment', foreground: '6B7280', fontStyle: 'italic' },
    { token: 'operator', foreground: '4B5563' },
    { token: 'identifier', foreground: '1F2937' },
    { token: 'variable', foreground: 'B91C1C' },
  ],
  dark: [
    { token: 'keyword', foreground: 'A78BFA', fontStyle: 'bold' },
    { token: 'type', foreground: '22D3EE' },
    { token: 'predefined', foreground: 'FB923C' },
    { token: 'string', foreground: '4ADE80' },
    { token: 'number', foreground: 'F87171' },
    { token: 'comment', foreground: '9CA3AF', fontStyle: 'italic' },
    { token: 'operator', foreground: 'D1D5DB' },
    { token: 'identifier', foreground: 'F3F4F6' },
    { token: 'variable', foreground: 'FCA5A5' },
  ],
};
```

### 12.3 Autocomplete Provider

```typescript
// src/lib/components/editor/autocompleteProvider.ts

import * as monaco from 'monaco-editor';
import type { SchemaCache } from '$lib/stores/schemaCache.svelte';

export class PostgresCompletionProvider implements monaco.languages.CompletionItemProvider {
  triggerCharacters = ['.', ' ', '(', ',', '"'];
  private schemaCache: SchemaCache;

  constructor(schemaCache: SchemaCache) {
    this.schemaCache = schemaCache;
  }

  async provideCompletionItems(
    model: monaco.editor.ITextModel,
    position: monaco.Position,
    context: monaco.languages.CompletionContext,
    token: monaco.CancellationToken
  ): Promise<monaco.languages.CompletionList> {
    const word = model.getWordUntilPosition(position);
    const range: monaco.IRange = {
      startLineNumber: position.lineNumber,
      endLineNumber: position.lineNumber,
      startColumn: word.startColumn,
      endColumn: word.endColumn,
    };

    // Get text before cursor to understand context
    const textUntilPosition = model.getValueInRange({
      startLineNumber: 1,
      startColumn: 1,
      endLineNumber: position.lineNumber,
      endColumn: position.column,
    });

    const suggestions: monaco.languages.CompletionItem[] = [];

    // Determine completion context
    const completionContext = this.analyzeContext(textUntilPosition, position);

    switch (completionContext.type) {
      case 'schema':
        suggestions.push(...this.getSchemaCompletions(range));
        break;

      case 'table':
        suggestions.push(...this.getTableCompletions(completionContext.schema, range));
        break;

      case 'column':
        suggestions.push(...this.getColumnCompletions(
          completionContext.tables,
          completionContext.aliases,
          range
        ));
        break;

      case 'function':
        suggestions.push(...this.getFunctionCompletions(completionContext.schema, range));
        break;

      default:
        // General completions
        suggestions.push(...this.getKeywordCompletions(range));
        suggestions.push(...this.getSchemaCompletions(range));
        suggestions.push(...this.getTableCompletions(undefined, range));
        suggestions.push(...this.getFunctionCompletions(undefined, range));
    }

    return { suggestions };
  }

  private analyzeContext(text: string, position: monaco.Position): CompletionContext {
    const textLower = text.toLowerCase();

    // Check for schema.table pattern
    const schemaTableMatch = text.match(/(\w+)\.(\w*)$/);
    if (schemaTableMatch) {
      const schema = schemaTableMatch[1];
      // If schema exists, complete tables in that schema
      if (this.schemaCache.hasSchema(schema)) {
        return { type: 'table', schema };
      }
      // Otherwise it might be table.column
      const table = this.schemaCache.findTable(schema);
      if (table) {
        return {
          type: 'column',
          tables: [table],
          aliases: new Map([[schema, table]]),
        };
      }
    }

    // Check for alias.column pattern
    const aliasMatch = text.match(/(\w+)\.$/);
    if (aliasMatch) {
      const alias = aliasMatch[1];
      const aliases = this.extractAliases(text);
      const table = aliases.get(alias);
      if (table) {
        return {
          type: 'column',
          tables: [table],
          aliases,
        };
      }
    }

    // After SELECT, suggest columns and tables
    if (/\bselect\s+[\w\s,*]*$/i.test(text) && !/\bfrom\b/i.test(text)) {
      const tables = this.extractTablesFromQuery(text);
      if (tables.length > 0) {
        return {
          type: 'column',
          tables,
          aliases: this.extractAliases(text),
        };
      }
    }

    // After FROM or JOIN, suggest tables
    if (/\b(from|join)\s+(\w*)$/i.test(text)) {
      return { type: 'table' };
    }

    // After WHERE, ON, AND, OR, suggest columns
    if (/\b(where|on|and|or)\s+[\w.]*$/i.test(text)) {
      const tables = this.extractTablesFromQuery(text);
      return {
        type: 'column',
        tables,
        aliases: this.extractAliases(text),
      };
    }

    return { type: 'general' };
  }

  private extractAliases(text: string): Map<string, string> {
    const aliases = new Map<string, string>();

    // Match table aliases: FROM table AS alias, FROM table alias
    const aliasPattern = /\b(\w+(?:\.\w+)?)\s+(?:as\s+)?(\w+)(?=\s*(?:,|\bwhere\b|\bjoin\b|\bon\b|$))/gi;
    let match;

    while ((match = aliasPattern.exec(text)) !== null) {
      const table = match[1];
      const alias = match[2];
      if (alias.toLowerCase() !== 'as') {
        aliases.set(alias.toLowerCase(), table);
      }
    }

    return aliases;
  }

  private extractTablesFromQuery(text: string): string[] {
    const tables: string[] = [];

    // Match FROM clause tables
    const fromMatch = text.match(/\bfrom\s+([\w.\s,]+?)(?:\bwhere\b|\bjoin\b|\bgroup\b|\border\b|\blimit\b|$)/i);
    if (fromMatch) {
      const fromClause = fromMatch[1];
      const tableMatches = fromClause.matchAll(/(\w+(?:\.\w+)?)/g);
      for (const m of tableMatches) {
        if (!['as'].includes(m[1].toLowerCase())) {
          tables.push(m[1]);
        }
      }
    }

    // Match JOIN tables
    const joinPattern = /\bjoin\s+(\w+(?:\.\w+)?)/gi;
    let match;
    while ((match = joinPattern.exec(text)) !== null) {
      tables.push(match[1]);
    }

    return [...new Set(tables)];
  }

  private getSchemaCompletions(range: monaco.IRange): monaco.languages.CompletionItem[] {
    return this.schemaCache.getSchemaNames().map((name) => ({
      label: name,
      kind: monaco.languages.CompletionItemKind.Module,
      insertText: name,
      range,
      detail: 'Schema',
    }));
  }

  private getTableCompletions(
    schemaName: string | undefined,
    range: monaco.IRange
  ): monaco.languages.CompletionItem[] {
    const tables = schemaName
      ? this.schemaCache.getTablesInSchema(schemaName)
      : this.schemaCache.getAllTables();

    return tables.map((table) => ({
      label: table.name,
      kind: monaco.languages.CompletionItemKind.Class,
      insertText: table.name,
      range,
      detail: `Table (${table.schema})`,
      documentation: table.comment || `${table.row_count_estimate?.toLocaleString() ?? '?'} rows`,
    }));
  }

  private getColumnCompletions(
    tableRefs: string[],
    aliases: Map<string, string>,
    range: monaco.IRange
  ): monaco.languages.CompletionItem[] {
    const columns: monaco.languages.CompletionItem[] = [];
    const seenColumns = new Set<string>();

    for (const tableRef of tableRefs) {
      const table = this.schemaCache.findTable(tableRef);
      if (!table) continue;

      for (const column of table.columns) {
        const key = `${table.name}.${column.name}`;
        if (seenColumns.has(key)) continue;
        seenColumns.add(key);

        columns.push({
          label: column.name,
          kind: monaco.languages.CompletionItemKind.Field,
          insertText: column.name,
          range,
          detail: column.type,
          documentation: column.comment || undefined,
          sortText: `0${column.name}`, // Prioritize columns
        });
      }
    }

    return columns;
  }

  private getFunctionCompletions(
    schemaName: string | undefined,
    range: monaco.IRange
  ): monaco.languages.CompletionItem[] {
    const functions = schemaName
      ? this.schemaCache.getFunctionsInSchema(schemaName)
      : this.schemaCache.getAllFunctions();

    return functions.map((fn) => ({
      label: fn.name,
      kind: monaco.languages.CompletionItemKind.Function,
      insertText: `${fn.name}($0)`,
      insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
      range,
      detail: fn.return_type,
      documentation: {
        value: `**${fn.name}**(${fn.arguments})\n\nReturns: ${fn.return_type}\n\n${fn.comment || ''}`,
      },
    }));
  }

  private getKeywordCompletions(range: monaco.IRange): monaco.languages.CompletionItem[] {
    const keywords = [
      'SELECT', 'FROM', 'WHERE', 'AND', 'OR', 'NOT', 'IN', 'EXISTS',
      'JOIN', 'LEFT JOIN', 'RIGHT JOIN', 'INNER JOIN', 'FULL JOIN',
      'ON', 'AS', 'ORDER BY', 'GROUP BY', 'HAVING', 'LIMIT', 'OFFSET',
      'INSERT INTO', 'VALUES', 'UPDATE', 'SET', 'DELETE FROM',
      'CREATE TABLE', 'ALTER TABLE', 'DROP TABLE',
      'WITH', 'UNION', 'INTERSECT', 'EXCEPT',
    ];

    return keywords.map((kw) => ({
      label: kw,
      kind: monaco.languages.CompletionItemKind.Keyword,
      insertText: kw,
      range,
      sortText: `2${kw}`, // Lower priority than columns
    }));
  }
}

interface CompletionContext {
  type: 'schema' | 'table' | 'column' | 'function' | 'general';
  schema?: string;
  tables?: string[];
  aliases?: Map<string, string>;
}
```

### 12.4 SQL Editor Component

```svelte
<!-- src/lib/components/editor/SqlEditor.svelte -->
<script lang="ts">
  import { onMount, onDestroy, createEventDispatcher } from 'svelte';
  import * as monaco from 'monaco-editor';
  import { initializeMonaco, getEditorOptions } from './monacoSetup';
  import { PostgresCompletionProvider } from './autocompleteProvider';
  import { schemaCache } from '$lib/stores/schemaCache.svelte';
  import { settingsStore } from '$lib/stores/settings.svelte';
  import { themeStore } from '$lib/stores/theme.svelte';

  interface Props {
    value?: string;
    readonly?: boolean;
    connectionId?: string;
    onExecute?: () => void;
    onExecuteAll?: () => void;
    onCancel?: () => void;
    onChange?: (value: string) => void;
  }

  let {
    value = $bindable(''),
    readonly = false,
    connectionId,
    onExecute,
    onExecuteAll,
    onCancel,
    onChange,
  }: Props = $props();

  const dispatch = createEventDispatcher<{
    execute: { sql: string; selection: boolean };
    executeAll: { sql: string };
    cancel: void;
    change: { value: string };
    save: void;
  }>();

  let container: HTMLDivElement;
  let editor: monaco.editor.IStandaloneCodeEditor | null = null;
  let completionProvider: monaco.IDisposable | null = null;

  // Error decorations
  let errorDecorations: string[] = [];

  onMount(() => {
    initializeMonaco();

    // Create editor
    editor = monaco.editor.create(container, {
      ...getEditorOptions(readonly),
      value,
    });

    // Register completion provider
    const provider = new PostgresCompletionProvider(schemaCache);
    completionProvider = monaco.languages.registerCompletionItemProvider(
      'postgresql',
      provider
    );

    // Listen for content changes
    editor.onDidChangeModelContent(() => {
      const newValue = editor!.getValue();
      value = newValue;
      onChange?.(newValue);
      dispatch('change', { value: newValue });

      // Clear errors on change
      clearErrors();
    });

    // Add keyboard shortcuts
    setupKeyboardShortcuts();

    // Set initial theme
    updateTheme();
  });

  onDestroy(() => {
    completionProvider?.dispose();
    editor?.dispose();
  });

  // React to theme changes
  $effect(() => {
    if ($themeStore === 'dark') {
      monaco.editor.setTheme('tusk-dark');
    } else {
      monaco.editor.setTheme('tusk-light');
    }
  });

  // React to external value changes
  $effect(() => {
    if (editor && editor.getValue() !== value) {
      editor.setValue(value);
    }
  });

  function setupKeyboardShortcuts() {
    if (!editor) return;

    // Execute current statement (Cmd/Ctrl+Enter)
    editor.addCommand(
      monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter,
      () => {
        const selection = editor!.getSelection();
        const sql = selection && !selection.isEmpty()
          ? editor!.getModel()!.getValueInRange(selection)
          : getCurrentStatement();

        dispatch('execute', { sql, selection: selection !== null && !selection.isEmpty() });
        onExecute?.();
      }
    );

    // Execute all (Cmd/Ctrl+Shift+Enter)
    editor.addCommand(
      monaco.KeyMod.CtrlCmd | monaco.KeyMod.Shift | monaco.KeyCode.Enter,
      () => {
        const sql = editor!.getValue();
        dispatch('executeAll', { sql });
        onExecuteAll?.();
      }
    );

    // Cancel (Cmd/Ctrl+.)
    editor.addCommand(
      monaco.KeyMod.CtrlCmd | monaco.KeyCode.Period,
      () => {
        dispatch('cancel');
        onCancel?.();
      }
    );

    // Save (Cmd/Ctrl+S)
    editor.addCommand(
      monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS,
      () => {
        dispatch('save');
      }
    );

    // Comment line (Cmd/Ctrl+/)
    editor.addCommand(
      monaco.KeyMod.CtrlCmd | monaco.KeyCode.Slash,
      () => {
        editor!.trigger('keyboard', 'editor.action.commentLine', null);
      }
    );
  }

  function getCurrentStatement(): string {
    if (!editor) return '';

    const model = editor.getModel();
    if (!model) return '';

    const position = editor.getPosition();
    if (!position) return model.getValue();

    const text = model.getValue();
    const offset = model.getOffsetAt(position);

    // Find statement boundaries
    let start = 0;
    let end = text.length;

    // Find previous semicolon
    for (let i = offset - 1; i >= 0; i--) {
      if (text[i] === ';') {
        start = i + 1;
        break;
      }
    }

    // Find next semicolon
    for (let i = offset; i < text.length; i++) {
      if (text[i] === ';') {
        end = i + 1;
        break;
      }
    }

    return text.substring(start, end).trim();
  }

  function updateTheme() {
    const theme = $themeStore === 'dark' ? 'tusk-dark' : 'tusk-light';
    monaco.editor.setTheme(theme);
  }

  // Public API
  export function showError(position: number, message: string) {
    if (!editor) return;

    const model = editor.getModel();
    if (!model) return;

    const pos = model.getPositionAt(position);

    // Add error decoration
    const newDecorations = editor.deltaDecorations(errorDecorations, [
      {
        range: new monaco.Range(pos.lineNumber, pos.column, pos.lineNumber, pos.column + 1),
        options: {
          isWholeLine: false,
          className: 'squiggly-error',
          hoverMessage: { value: message },
          glyphMarginClassName: 'error-glyph',
        },
      },
    ]);

    errorDecorations = newDecorations;

    // Move cursor to error position
    editor.setPosition(pos);
    editor.revealPositionInCenter(pos);
  }

  export function clearErrors() {
    if (!editor) return;
    errorDecorations = editor.deltaDecorations(errorDecorations, []);
  }

  export function focus() {
    editor?.focus();
  }

  export function getSelectedText(): string | null {
    if (!editor) return null;
    const selection = editor.getSelection();
    if (!selection || selection.isEmpty()) return null;
    return editor.getModel()!.getValueInRange(selection);
  }

  export function insertText(text: string) {
    editor?.trigger('keyboard', 'type', { text });
  }

  export function formatDocument() {
    // SQL formatting would need a separate formatter
    // For now, just trigger Monaco's built-in formatting
    editor?.trigger('keyboard', 'editor.action.formatDocument', null);
  }
</script>

<div bind:this={container} class="sql-editor"></div>

<style>
  .sql-editor {
    width: 100%;
    height: 100%;
    min-height: 200px;
  }

  :global(.squiggly-error) {
    text-decoration: underline wavy red;
  }

  :global(.error-glyph) {
    background-color: #ef4444;
    border-radius: 50%;
    width: 8px !important;
    height: 8px !important;
    margin-left: 4px;
    margin-top: 6px;
  }
</style>
```

### 12.5 Editor Toolbar Component

```svelte
<!-- src/lib/components/editor/EditorToolbar.svelte -->
<script lang="ts">
  import { Play, Square, FileText, Save, ChevronDown } from 'lucide-svelte';
  import { connectionsStore } from '$lib/stores/connections.svelte';

  interface Props {
    connectionId?: string;
    isExecuting?: boolean;
    rowLimit?: number;
    onRun?: () => void;
    onRunAll?: () => void;
    onStop?: () => void;
    onFormat?: () => void;
    onSave?: () => void;
    onConnectionChange?: (id: string) => void;
    onRowLimitChange?: (limit: number) => void;
  }

  let {
    connectionId,
    isExecuting = false,
    rowLimit = 1000,
    onRun,
    onRunAll,
    onStop,
    onFormat,
    onSave,
    onConnectionChange,
    onRowLimitChange,
  }: Props = $props();

  const rowLimitOptions = [100, 500, 1000, 5000, 10000, 50000, 0];

  function formatRowLimit(limit: number): string {
    if (limit === 0) return 'No limit';
    return limit.toLocaleString();
  }
</script>

<div class="toolbar">
  <div class="toolbar-group">
    <button
      class="btn btn-primary"
      onclick={onRun}
      disabled={isExecuting || !connectionId}
      title="Execute (Cmd+Enter)"
    >
      <Play size={16} />
      Run
    </button>

    <button
      class="btn btn-secondary"
      onclick={onRunAll}
      disabled={isExecuting || !connectionId}
      title="Execute All (Cmd+Shift+Enter)"
    >
      Run All
    </button>

    {#if isExecuting}
      <button
        class="btn btn-danger"
        onclick={onStop}
        title="Cancel (Cmd+.)"
      >
        <Square size={16} />
        Stop
      </button>
    {/if}
  </div>

  <div class="toolbar-separator"></div>

  <div class="toolbar-group">
    <button
      class="btn btn-ghost"
      onclick={onFormat}
      title="Format SQL (Cmd+Shift+F)"
    >
      <FileText size={16} />
      Format
    </button>

    <button
      class="btn btn-ghost"
      onclick={onSave}
      title="Save (Cmd+S)"
    >
      <Save size={16} />
      Save
    </button>
  </div>

  <div class="toolbar-spacer"></div>

  <div class="toolbar-group">
    <!-- Connection selector -->
    <div class="select-wrapper">
      <select
        class="select"
        value={connectionId}
        onchange={(e) => onConnectionChange?.(e.currentTarget.value)}
      >
        <option value="">Select connection...</option>
        {#each $connectionsStore.connections as conn}
          <option value={conn.id}>
            {conn.name}
          </option>
        {/each}
      </select>
      <ChevronDown size={16} class="select-icon" />
    </div>

    <!-- Row limit selector -->
    <div class="select-wrapper">
      <select
        class="select"
        value={rowLimit}
        onchange={(e) => onRowLimitChange?.(Number(e.currentTarget.value))}
      >
        {#each rowLimitOptions as limit}
          <option value={limit}>
            Limit: {formatRowLimit(limit)}
          </option>
        {/each}
      </select>
      <ChevronDown size={16} class="select-icon" />
    </div>
  </div>
</div>

<style>
  .toolbar {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem;
    border-bottom: 1px solid var(--border-color);
    background: var(--surface-color);
  }

  .toolbar-group {
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }

  .toolbar-separator {
    width: 1px;
    height: 24px;
    background: var(--border-color);
    margin: 0 0.25rem;
  }

  .toolbar-spacer {
    flex: 1;
  }

  .btn {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.375rem 0.75rem;
    border: none;
    border-radius: 0.375rem;
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-primary {
    background: var(--primary-color);
    color: white;
  }

  .btn-primary:hover:not(:disabled) {
    background: var(--primary-hover);
  }

  .btn-secondary {
    background: var(--secondary-color);
    color: var(--text-color);
  }

  .btn-secondary:hover:not(:disabled) {
    background: var(--secondary-hover);
  }

  .btn-ghost {
    background: transparent;
    color: var(--text-muted);
  }

  .btn-ghost:hover:not(:disabled) {
    background: var(--hover-color);
    color: var(--text-color);
  }

  .btn-danger {
    background: #ef4444;
    color: white;
  }

  .btn-danger:hover {
    background: #dc2626;
  }

  .select-wrapper {
    position: relative;
    display: flex;
    align-items: center;
  }

  .select {
    appearance: none;
    padding: 0.375rem 2rem 0.375rem 0.75rem;
    border: 1px solid var(--border-color);
    border-radius: 0.375rem;
    background: var(--surface-color);
    color: var(--text-color);
    font-size: 0.875rem;
    cursor: pointer;
  }

  .select:focus {
    outline: none;
    border-color: var(--primary-color);
  }

  .select-icon {
    position: absolute;
    right: 0.5rem;
    pointer-events: none;
    color: var(--text-muted);
  }
</style>
```

### 12.6 Schema Cache for Autocomplete

```typescript
// src/lib/stores/schemaCache.svelte.ts

import type { Schema, Table, Function as PgFunction } from '$lib/services/schema';

interface CachedSchema {
  name: string;
  tables: Map<string, Table>;
  functions: Map<string, PgFunction>;
}

class SchemaCache {
  private schemas = $state<Map<string, CachedSchema>>(new Map());
  private connectionId = $state<string | null>(null);

  setConnectionSchema(connId: string, schemas: Schema[]) {
    this.connectionId = connId;
    this.schemas.clear();

    for (const schema of schemas) {
      const cachedSchema: CachedSchema = {
        name: schema.name,
        tables: new Map(schema.tables.map(t => [t.name.toLowerCase(), t])),
        functions: new Map(schema.functions.map(f => [f.name.toLowerCase(), f])),
      };
      this.schemas.set(schema.name.toLowerCase(), cachedSchema);
    }
  }

  clear() {
    this.schemas.clear();
    this.connectionId = null;
  }

  hasSchema(name: string): boolean {
    return this.schemas.has(name.toLowerCase());
  }

  getSchemaNames(): string[] {
    return Array.from(this.schemas.keys());
  }

  getTablesInSchema(schemaName: string): Table[] {
    const schema = this.schemas.get(schemaName.toLowerCase());
    return schema ? Array.from(schema.tables.values()) : [];
  }

  getAllTables(): Table[] {
    const tables: Table[] = [];
    for (const schema of this.schemas.values()) {
      tables.push(...schema.tables.values());
    }
    return tables;
  }

  findTable(ref: string): Table | null {
    // Handle schema.table format
    if (ref.includes('.')) {
      const [schemaName, tableName] = ref.split('.');
      const schema = this.schemas.get(schemaName.toLowerCase());
      return schema?.tables.get(tableName.toLowerCase()) ?? null;
    }

    // Search all schemas
    for (const schema of this.schemas.values()) {
      const table = schema.tables.get(ref.toLowerCase());
      if (table) return table;
    }
    return null;
  }

  getFunctionsInSchema(schemaName: string): PgFunction[] {
    const schema = this.schemas.get(schemaName.toLowerCase());
    return schema ? Array.from(schema.functions.values()) : [];
  }

  getAllFunctions(): PgFunction[] {
    const functions: PgFunction[] = [];
    for (const schema of this.schemas.values()) {
      functions.push(...schema.functions.values());
    }
    return functions;
  }
}

export const schemaCache = new SchemaCache();
export type { SchemaCache };
```

## Acceptance Criteria

1. **Monaco Integration**
   - Monaco editor renders correctly in Svelte components
   - Editor handles Postgres SQL syntax properly
   - Web workers load correctly for editor performance

2. **Syntax Highlighting**
   - Keywords are highlighted correctly (SELECT, FROM, WHERE, etc.)
   - Data types have distinct highlighting
   - Built-in functions are recognized
   - Strings, comments, and numbers are properly colored
   - Dollar-quoted strings work correctly

3. **Autocomplete**
   - Tables autocomplete after FROM/JOIN
   - Columns autocomplete in SELECT/WHERE context
   - Schema-qualified names work (schema.table)
   - Aliases are recognized for column completion
   - Functions show signature in documentation
   - Keywords have lower priority than schema objects

4. **Error Highlighting**
   - Errors from Postgres appear at correct positions
   - Error messages show on hover
   - Errors clear when user edits the line

5. **Keyboard Shortcuts**
   - Cmd/Ctrl+Enter executes current statement
   - Cmd/Ctrl+Shift+Enter executes all
   - Cmd/Ctrl+. cancels query
   - Cmd/Ctrl+S triggers save
   - Cmd/Ctrl+/ comments line

6. **Theme Support**
   - Light and dark themes work correctly
   - Theme changes update editor immediately

## MCP Testing Instructions

### Using Playwright MCP

```typescript
// Navigate to query editor
await mcp.browser_navigate({ url: 'http://localhost:1420' });

// Wait for editor to load
await mcp.browser_wait_for({ text: 'SELECT' });

// Take snapshot of editor
const snapshot = await mcp.browser_snapshot();

// Test typing SQL
await mcp.browser_click({ element: 'SQL editor', ref: 'editor-container' });
await mcp.browser_type({
  element: 'SQL editor',
  ref: 'monaco-editor',
  text: 'SELECT * FROM users WHERE id = 1;'
});

// Test autocomplete trigger
await mcp.browser_type({
  element: 'SQL editor',
  ref: 'monaco-editor',
  text: 'SELECT u.',
  slowly: true
});

// Verify autocomplete popup appears
await mcp.browser_wait_for({ text: 'id' }); // Column name from autocomplete

// Test keyboard shortcut
await mcp.browser_press_key({ key: 'Enter', modifiers: ['Meta'] });

// Verify execution started
await mcp.browser_wait_for({ text: 'Executing' });
```

### Using Tauri MCP

```typescript
// Connect to app
await mcp.driver_session({ action: 'start' });

// Find editor element
const editor = await mcp.webview_find_element({
  selector: '.monaco-editor',
  strategy: 'css'
});

// Execute JS to get editor value
const content = await mcp.webview_execute_js({
  script: `
    const editor = monaco.editor.getModels()[0];
    return editor?.getValue() ?? '';
  `
});

// Verify syntax highlighting
const tokens = await mcp.webview_execute_js({
  script: `
    const model = monaco.editor.getModels()[0];
    return monaco.editor.tokenize(model.getValue(), 'postgresql');
  `
});
```

## Dependencies

- monaco-editor (npm package)
- vite-plugin-monaco-editor (for web worker bundling)
- Feature 10: Schema Introspection (for autocomplete data)
