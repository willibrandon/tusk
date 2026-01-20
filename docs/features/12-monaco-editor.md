# Feature 12: SQL Editor

## Overview

The SQL Editor provides the primary code editing experience in Tusk. Built natively in Rust with GPUI, it features schema-aware autocomplete, syntax highlighting for Postgres-specific keywords using tree-sitter, error highlighting at exact positions, and full keyboard shortcut support. This is a custom text editor component optimized for SQL editing.

## Goals

- Implement a native Rust text editor for SQL editing
- Provide Postgres-specific syntax highlighting via tree-sitter
- Implement schema-aware autocomplete (tables, columns, functions, keywords)
- Show error markers at positions returned by Postgres
- Support code folding for CTEs and subqueries
- Enable multi-cursor editing and standard editor features
- Achieve <50ms autocomplete response time

## Dependencies

- Feature 03: Frontend Architecture (GPUI component structure)
- Feature 10: Schema Introspection (schema metadata for autocomplete)
- Feature 11: Query Execution (error position information)

## Technical Specification

### 12.1 Editor Buffer and State

```rust
// src/ui/editor/buffer.rs

use std::ops::Range;
use std::sync::Arc;
use ropey::Rope;
use parking_lot::RwLock;
use uuid::Uuid;

/// A text buffer that supports efficient editing operations
#[derive(Clone)]
pub struct Buffer {
    /// Unique buffer ID
    id: Uuid,
    /// Text content using rope data structure for efficient editing
    content: Arc<RwLock<Rope>>,
    /// Undo/redo history
    history: Arc<RwLock<EditHistory>>,
    /// Current syntax tree
    syntax_tree: Arc<RwLock<Option<tree_sitter::Tree>>>,
    /// Parser instance
    parser: Arc<RwLock<tree_sitter::Parser>>,
    /// Edit version (incremented on each change)
    version: Arc<RwLock<u64>>,
}

impl Buffer {
    pub fn new() -> Self {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_sql::language()).ok();

        Self {
            id: Uuid::new_v4(),
            content: Arc::new(RwLock::new(Rope::new())),
            history: Arc::new(RwLock::new(EditHistory::new())),
            syntax_tree: Arc::new(RwLock::new(None)),
            parser: Arc::new(RwLock::new(parser)),
            version: Arc::new(RwLock::new(0)),
        }
    }

    pub fn from_str(text: &str) -> Self {
        let buffer = Self::new();
        buffer.set_content(text);
        buffer
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn version(&self) -> u64 {
        *self.version.read()
    }

    /// Get the full text content
    pub fn content(&self) -> String {
        self.content.read().to_string()
    }

    /// Set the full content
    pub fn set_content(&self, text: &str) {
        let mut content = self.content.write();
        *content = Rope::from_str(text);
        *self.version.write() += 1;
        self.reparse();
    }

    /// Get total line count
    pub fn line_count(&self) -> usize {
        self.content.read().len_lines()
    }

    /// Get a specific line
    pub fn line(&self, line_num: usize) -> Option<String> {
        let content = self.content.read();
        if line_num < content.len_lines() {
            Some(content.line(line_num).to_string())
        } else {
            None
        }
    }

    /// Get character count
    pub fn len(&self) -> usize {
        self.content.read().len_chars()
    }

    /// Insert text at position
    pub fn insert(&self, offset: usize, text: &str) {
        let mut content = self.content.write();
        let old_len = content.len_chars();

        // Record for undo
        self.history.write().push(EditOperation::Insert {
            offset,
            text: text.to_string(),
        });

        content.insert(offset, text);
        *self.version.write() += 1;

        drop(content);
        self.reparse();
    }

    /// Delete text in range
    pub fn delete(&self, range: Range<usize>) {
        let mut content = self.content.write();

        // Record deleted text for undo
        let deleted: String = content.slice(range.clone()).chars().collect();
        self.history.write().push(EditOperation::Delete {
            offset: range.start,
            text: deleted,
        });

        content.remove(range);
        *self.version.write() += 1;

        drop(content);
        self.reparse();
    }

    /// Replace text in range
    pub fn replace(&self, range: Range<usize>, text: &str) {
        let mut content = self.content.write();

        // Record for undo
        let deleted: String = content.slice(range.clone()).chars().collect();
        self.history.write().push(EditOperation::Replace {
            offset: range.start,
            old_text: deleted,
            new_text: text.to_string(),
        });

        content.remove(range.clone());
        content.insert(range.start, text);
        *self.version.write() += 1;

        drop(content);
        self.reparse();
    }

    /// Get text in range
    pub fn slice(&self, range: Range<usize>) -> String {
        self.content.read().slice(range).to_string()
    }

    /// Convert line/column to offset
    pub fn line_col_to_offset(&self, line: usize, col: usize) -> usize {
        let content = self.content.read();
        if line >= content.len_lines() {
            return content.len_chars();
        }
        let line_start = content.line_to_char(line);
        let line_len = content.line(line).len_chars();
        line_start + col.min(line_len)
    }

    /// Convert offset to line/column
    pub fn offset_to_line_col(&self, offset: usize) -> (usize, usize) {
        let content = self.content.read();
        let line = content.char_to_line(offset.min(content.len_chars().saturating_sub(1)));
        let line_start = content.line_to_char(line);
        let col = offset.saturating_sub(line_start);
        (line, col)
    }

    /// Reparse the syntax tree
    fn reparse(&self) {
        let content = self.content.read();
        let text = content.to_string();
        drop(content);

        let old_tree = self.syntax_tree.read().clone();
        let mut parser = self.parser.write();

        let new_tree = parser.parse(&text, old_tree.as_ref());
        *self.syntax_tree.write() = new_tree;
    }

    /// Get the syntax tree
    pub fn syntax_tree(&self) -> Option<tree_sitter::Tree> {
        self.syntax_tree.read().clone()
    }

    /// Undo last edit
    pub fn undo(&self) -> bool {
        if let Some(op) = self.history.write().undo() {
            self.apply_inverse(&op);
            true
        } else {
            false
        }
    }

    /// Redo last undone edit
    pub fn redo(&self) -> bool {
        if let Some(op) = self.history.write().redo() {
            self.apply_operation(&op);
            true
        } else {
            false
        }
    }

    fn apply_operation(&self, op: &EditOperation) {
        let mut content = self.content.write();
        match op {
            EditOperation::Insert { offset, text } => {
                content.insert(*offset, text);
            }
            EditOperation::Delete { offset, text } => {
                content.remove(*offset..(*offset + text.len()));
            }
            EditOperation::Replace { offset, old_text, new_text } => {
                content.remove(*offset..(*offset + old_text.len()));
                content.insert(*offset, new_text);
            }
        }
        *self.version.write() += 1;
        drop(content);
        self.reparse();
    }

    fn apply_inverse(&self, op: &EditOperation) {
        let mut content = self.content.write();
        match op {
            EditOperation::Insert { offset, text } => {
                content.remove(*offset..(*offset + text.len()));
            }
            EditOperation::Delete { offset, text } => {
                content.insert(*offset, text);
            }
            EditOperation::Replace { offset, old_text, new_text } => {
                content.remove(*offset..(*offset + new_text.len()));
                content.insert(*offset, old_text);
            }
        }
        *self.version.write() += 1;
        drop(content);
        self.reparse();
    }
}

/// An edit operation for undo/redo
#[derive(Clone, Debug)]
enum EditOperation {
    Insert { offset: usize, text: String },
    Delete { offset: usize, text: String },
    Replace { offset: usize, old_text: String, new_text: String },
}

/// Edit history for undo/redo
struct EditHistory {
    undo_stack: Vec<EditOperation>,
    redo_stack: Vec<EditOperation>,
    max_history: usize,
}

impl EditHistory {
    fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history: 1000,
        }
    }

    fn push(&mut self, op: EditOperation) {
        self.undo_stack.push(op);
        self.redo_stack.clear();

        if self.undo_stack.len() > self.max_history {
            self.undo_stack.remove(0);
        }
    }

    fn undo(&mut self) -> Option<EditOperation> {
        let op = self.undo_stack.pop()?;
        self.redo_stack.push(op.clone());
        Some(op)
    }

    fn redo(&mut self) -> Option<EditOperation> {
        let op = self.redo_stack.pop()?;
        self.undo_stack.push(op.clone());
        Some(op)
    }
}
```

### 12.2 Selection and Cursor

```rust
// src/ui/editor/selection.rs

use std::ops::Range;

/// A position in the editor (line and column)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl Position {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

impl PartialOrd for Position {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Position {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.line.cmp(&other.line) {
            std::cmp::Ordering::Equal => self.column.cmp(&other.column),
            ord => ord,
        }
    }
}

/// A selection in the editor
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selection {
    /// Anchor position (where selection started)
    pub anchor: Position,
    /// Head position (current cursor position)
    pub head: Position,
    /// Desired column for vertical movement
    pub goal_column: Option<usize>,
}

impl Selection {
    pub fn new(anchor: Position, head: Position) -> Self {
        Self {
            anchor,
            head,
            goal_column: None,
        }
    }

    pub fn cursor(position: Position) -> Self {
        Self {
            anchor: position,
            head: position,
            goal_column: None,
        }
    }

    /// Check if this is a cursor (no selection)
    pub fn is_cursor(&self) -> bool {
        self.anchor == self.head
    }

    /// Get the start position (min of anchor and head)
    pub fn start(&self) -> Position {
        self.anchor.min(self.head)
    }

    /// Get the end position (max of anchor and head)
    pub fn end(&self) -> Position {
        self.anchor.max(self.head)
    }

    /// Get as range of positions
    pub fn range(&self) -> Range<Position> {
        self.start()..self.end()
    }

    /// Collapse selection to cursor at head
    pub fn collapse_to_head(&self) -> Self {
        Self::cursor(self.head)
    }

    /// Collapse selection to cursor at start
    pub fn collapse_to_start(&self) -> Self {
        Self::cursor(self.start())
    }

    /// Collapse selection to cursor at end
    pub fn collapse_to_end(&self) -> Self {
        Self::cursor(self.end())
    }
}

/// Multiple selections (for multi-cursor support)
#[derive(Clone, Debug)]
pub struct Selections {
    /// All selections (always at least one)
    selections: Vec<Selection>,
    /// Primary selection index
    primary: usize,
}

impl Selections {
    pub fn new(selection: Selection) -> Self {
        Self {
            selections: vec![selection],
            primary: 0,
        }
    }

    pub fn cursor(position: Position) -> Self {
        Self::new(Selection::cursor(position))
    }

    pub fn primary(&self) -> &Selection {
        &self.selections[self.primary]
    }

    pub fn primary_mut(&mut self) -> &mut Selection {
        &mut self.selections[self.primary]
    }

    pub fn all(&self) -> &[Selection] {
        &self.selections
    }

    pub fn all_mut(&mut self) -> &mut [Selection] {
        &mut self.selections
    }

    pub fn add(&mut self, selection: Selection) {
        self.selections.push(selection);
        self.primary = self.selections.len() - 1;
        self.merge_overlapping();
    }

    pub fn remove(&mut self, index: usize) {
        if self.selections.len() > 1 {
            self.selections.remove(index);
            if self.primary >= self.selections.len() {
                self.primary = self.selections.len() - 1;
            }
        }
    }

    pub fn clear_secondary(&mut self) {
        let primary = self.selections[self.primary].clone();
        self.selections = vec![primary];
        self.primary = 0;
    }

    /// Merge overlapping selections
    fn merge_overlapping(&mut self) {
        if self.selections.len() <= 1 {
            return;
        }

        // Sort by start position
        self.selections.sort_by(|a, b| a.start().cmp(&b.start()));

        let mut merged = Vec::new();
        let mut current = self.selections[0].clone();

        for selection in self.selections.iter().skip(1) {
            if selection.start() <= current.end() {
                // Overlapping - extend current
                if selection.end() > current.end() {
                    current.head = selection.end();
                }
            } else {
                merged.push(current);
                current = selection.clone();
            }
        }
        merged.push(current);

        self.selections = merged;
        self.primary = self.primary.min(self.selections.len() - 1);
    }
}
```

### 12.3 Syntax Highlighting

```rust
// src/ui/editor/highlight.rs

use tree_sitter::{Tree, Node, Query, QueryCursor};
use gpui::Hsla;
use std::ops::Range;

use crate::ui::theme::Theme;

/// Token types for PostgreSQL syntax
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TokenType {
    Keyword,
    KeywordDml,
    KeywordDdl,
    DataType,
    BuiltinFunction,
    String,
    Number,
    Identifier,
    QuotedIdentifier,
    Comment,
    Operator,
    Punctuation,
    Parameter,
    Variable,
    Error,
}

/// A highlighted token
#[derive(Clone, Debug)]
pub struct HighlightedToken {
    pub range: Range<usize>,
    pub token_type: TokenType,
}

/// PostgreSQL-specific keywords for classification
const KEYWORDS_DML: &[&str] = &[
    "select", "from", "where", "and", "or", "not", "in", "exists",
    "between", "like", "ilike", "is", "null", "as", "on", "join",
    "left", "right", "inner", "outer", "full", "cross", "natural",
    "union", "intersect", "except", "order", "by", "group", "having",
    "limit", "offset", "distinct", "all", "insert", "into", "values",
    "update", "set", "delete", "returning", "with", "recursive",
];

const KEYWORDS_DDL: &[&str] = &[
    "create", "alter", "drop", "table", "view", "index", "schema",
    "database", "function", "procedure", "trigger", "type", "domain",
    "extension", "sequence", "materialized", "temporary", "temp",
    "primary", "key", "foreign", "references", "unique", "check",
    "default", "constraint", "cascade", "restrict", "if", "exists",
    "grant", "revoke", "vacuum", "analyze", "reindex",
];

const DATA_TYPES: &[&str] = &[
    "integer", "int", "int2", "int4", "int8", "smallint", "bigint",
    "decimal", "numeric", "real", "float", "float4", "float8",
    "double", "precision", "serial", "bigserial", "boolean", "bool",
    "char", "character", "varchar", "text", "bytea", "date", "time",
    "timestamp", "timestamptz", "interval", "uuid", "json", "jsonb",
    "array", "inet", "cidr", "macaddr", "point", "line", "polygon",
];

const BUILTIN_FUNCTIONS: &[&str] = &[
    "count", "sum", "avg", "min", "max", "array_agg", "string_agg",
    "coalesce", "nullif", "greatest", "least", "now", "current_date",
    "current_timestamp", "extract", "date_part", "date_trunc",
    "length", "lower", "upper", "trim", "substring", "position",
    "concat", "replace", "split_part", "regexp_replace",
    "json_build_object", "jsonb_build_object", "to_json", "to_jsonb",
    "row_number", "rank", "dense_rank", "lag", "lead", "first_value",
    "generate_series", "unnest", "array_length",
];

/// Syntax highlighter using tree-sitter
pub struct SyntaxHighlighter {
    query: Query,
}

impl SyntaxHighlighter {
    pub fn new() -> Self {
        // Tree-sitter query for SQL highlighting
        let query_source = r#"
            (keyword) @keyword
            (identifier) @identifier
            (string) @string
            (number) @number
            (comment) @comment
            (operator) @operator
            (punctuation) @punctuation
            (parameter) @parameter
            (type_identifier) @type
            (function_call name: (identifier) @function)
            (ERROR) @error
        "#;

        let query = Query::new(&tree_sitter_sql::language(), query_source)
            .unwrap_or_else(|_| {
                // Fallback empty query
                Query::new(&tree_sitter_sql::language(), "").unwrap()
            });

        Self { query }
    }

    /// Highlight using tree-sitter
    pub fn highlight_tree(&self, tree: &Tree, source: &str) -> Vec<HighlightedToken> {
        let mut tokens = Vec::new();
        let mut cursor = QueryCursor::new();

        for match_ in cursor.matches(&self.query, tree.root_node(), source.as_bytes()) {
            for capture in match_.captures {
                let node = capture.node;
                let capture_name = &self.query.capture_names()[capture.index as usize];

                let token_type = match capture_name.as_str() {
                    "keyword" => self.classify_keyword(node, source),
                    "identifier" => self.classify_identifier(node, source),
                    "string" => TokenType::String,
                    "number" => TokenType::Number,
                    "comment" => TokenType::Comment,
                    "operator" => TokenType::Operator,
                    "punctuation" => TokenType::Punctuation,
                    "parameter" => TokenType::Parameter,
                    "type" => TokenType::DataType,
                    "function" => TokenType::BuiltinFunction,
                    "error" => TokenType::Error,
                    _ => TokenType::Identifier,
                };

                tokens.push(HighlightedToken {
                    range: node.byte_range(),
                    token_type,
                });
            }
        }

        // Sort by position
        tokens.sort_by_key(|t| t.range.start);
        tokens
    }

    /// Fallback regex-based highlighting when tree-sitter isn't available
    pub fn highlight_regex(&self, source: &str) -> Vec<HighlightedToken> {
        let mut tokens = Vec::new();
        let mut pos = 0;

        while pos < source.len() {
            let remaining = &source[pos..];

            // Skip whitespace
            if let Some(ws_len) = self.skip_whitespace(remaining) {
                pos += ws_len;
                continue;
            }

            // Try to match token patterns
            if let Some((len, token_type)) = self.match_token(remaining) {
                tokens.push(HighlightedToken {
                    range: pos..pos + len,
                    token_type,
                });
                pos += len;
            } else {
                pos += 1;
            }
        }

        tokens
    }

    fn skip_whitespace(&self, s: &str) -> Option<usize> {
        let trimmed = s.trim_start();
        let skipped = s.len() - trimmed.len();
        if skipped > 0 { Some(skipped) } else { None }
    }

    fn match_token(&self, s: &str) -> Option<(usize, TokenType)> {
        // Line comment
        if s.starts_with("--") {
            let end = s.find('\n').unwrap_or(s.len());
            return Some((end, TokenType::Comment));
        }

        // Block comment
        if s.starts_with("/*") {
            if let Some(end) = s.find("*/") {
                return Some((end + 2, TokenType::Comment));
            }
            return Some((s.len(), TokenType::Comment));
        }

        // Single-quoted string
        if s.starts_with('\'') {
            let end = self.find_string_end(&s[1..], '\'');
            return Some((end + 1, TokenType::String));
        }

        // Double-quoted identifier
        if s.starts_with('"') {
            let end = self.find_string_end(&s[1..], '"');
            return Some((end + 1, TokenType::QuotedIdentifier));
        }

        // Dollar-quoted string
        if s.starts_with('$') {
            if let Some(end) = self.find_dollar_string_end(s) {
                return Some((end, TokenType::String));
            }
        }

        // Number
        if s.chars().next()?.is_ascii_digit() ||
           (s.starts_with('.') && s.chars().nth(1).map(|c| c.is_ascii_digit()).unwrap_or(false)) {
            let len = self.match_number(s);
            return Some((len, TokenType::Number));
        }

        // Parameter ($1, $2, etc.)
        if s.starts_with('$') && s.chars().nth(1).map(|c| c.is_ascii_digit()).unwrap_or(false) {
            let len = 1 + s[1..].chars().take_while(|c| c.is_ascii_digit()).count();
            return Some((len, TokenType::Parameter));
        }

        // Named parameter (:name)
        if s.starts_with(':') && s.chars().nth(1).map(|c| c.is_alphabetic()).unwrap_or(false) {
            let len = 1 + s[1..].chars().take_while(|c| c.is_alphanumeric() || *c == '_').count();
            return Some((len, TokenType::Variable));
        }

        // Identifier/keyword
        if s.chars().next()?.is_alphabetic() || s.starts_with('_') {
            let len = s.chars().take_while(|c| c.is_alphanumeric() || *c == '_').count();
            let word = &s[..len];
            let lower = word.to_lowercase();

            let token_type = if KEYWORDS_DDL.contains(&lower.as_str()) {
                TokenType::KeywordDdl
            } else if KEYWORDS_DML.contains(&lower.as_str()) {
                TokenType::Keyword
            } else if DATA_TYPES.contains(&lower.as_str()) {
                TokenType::DataType
            } else if BUILTIN_FUNCTIONS.contains(&lower.as_str()) {
                TokenType::BuiltinFunction
            } else {
                TokenType::Identifier
            };

            return Some((len, token_type));
        }

        // Operators
        let operators = ["<>", "!=", "<=", ">=", "->", "->>", "#>", "#>>",
                         "@>", "<@", "?|", "?&", "||", "::", "<<", ">>",
                         "=", "<", ">", "+", "-", "*", "/", "%", "^", "&", "|", "~", "?"];
        for op in operators {
            if s.starts_with(op) {
                return Some((op.len(), TokenType::Operator));
            }
        }

        // Punctuation
        if matches!(s.chars().next()?, '(' | ')' | '[' | ']' | '{' | '}' | ',' | ';' | '.') {
            return Some((1, TokenType::Punctuation));
        }

        None
    }

    fn find_string_end(&self, s: &str, quote: char) -> usize {
        let mut chars = s.char_indices();
        while let Some((i, c)) = chars.next() {
            if c == quote {
                // Check for escaped quote
                if chars.clone().next().map(|(_, c2)| c2) == Some(quote) {
                    chars.next(); // Skip escaped quote
                } else {
                    return i + 1;
                }
            }
        }
        s.len()
    }

    fn find_dollar_string_end(&self, s: &str) -> Option<usize> {
        // Find opening tag
        let tag_end = s[1..].find('$')? + 2;
        let tag = &s[..tag_end];

        // Find closing tag
        let rest = &s[tag_end..];
        rest.find(tag).map(|i| tag_end + i + tag.len())
    }

    fn match_number(&self, s: &str) -> usize {
        let mut len = 0;
        let mut has_dot = false;
        let mut has_e = false;

        for (i, c) in s.char_indices() {
            match c {
                '0'..='9' => len = i + 1,
                '.' if !has_dot && !has_e => {
                    has_dot = true;
                    len = i + 1;
                }
                'e' | 'E' if !has_e => {
                    has_e = true;
                    len = i + 1;
                }
                '+' | '-' if has_e && s[..i].ends_with(|c| c == 'e' || c == 'E') => {
                    len = i + 1;
                }
                _ => break,
            }
        }

        len
    }

    fn classify_keyword(&self, node: Node, source: &str) -> TokenType {
        let text = &source[node.byte_range()].to_lowercase();
        if KEYWORDS_DDL.contains(&text.as_str()) {
            TokenType::KeywordDdl
        } else {
            TokenType::Keyword
        }
    }

    fn classify_identifier(&self, node: Node, source: &str) -> TokenType {
        let text = &source[node.byte_range()].to_lowercase();
        if DATA_TYPES.contains(&text.as_str()) {
            TokenType::DataType
        } else if BUILTIN_FUNCTIONS.contains(&text.as_str()) {
            TokenType::BuiltinFunction
        } else {
            TokenType::Identifier
        }
    }
}

/// Get color for token type
pub fn token_color(token_type: TokenType, theme: &Theme) -> Hsla {
    match token_type {
        TokenType::Keyword | TokenType::KeywordDml => theme.syntax_keyword,
        TokenType::KeywordDdl => theme.syntax_keyword_ddl,
        TokenType::DataType => theme.syntax_type,
        TokenType::BuiltinFunction => theme.syntax_function,
        TokenType::String => theme.syntax_string,
        TokenType::Number => theme.syntax_number,
        TokenType::Comment => theme.syntax_comment,
        TokenType::Operator => theme.syntax_operator,
        TokenType::Punctuation => theme.syntax_punctuation,
        TokenType::Identifier => theme.syntax_identifier,
        TokenType::QuotedIdentifier => theme.syntax_identifier_quoted,
        TokenType::Parameter => theme.syntax_parameter,
        TokenType::Variable => theme.syntax_variable,
        TokenType::Error => theme.error,
    }
}
```

### 12.4 Autocomplete Provider

```rust
// src/ui/editor/autocomplete.rs

use std::sync::Arc;
use parking_lot::RwLock;

use crate::services::schema::SchemaCache;

/// Autocomplete suggestion
#[derive(Clone, Debug)]
pub struct Completion {
    /// Display label
    pub label: String,
    /// Text to insert
    pub insert_text: String,
    /// Kind of completion
    pub kind: CompletionKind,
    /// Detail text (type info, etc.)
    pub detail: Option<String>,
    /// Documentation
    pub documentation: Option<String>,
    /// Sort priority (lower = higher priority)
    pub sort_priority: u32,
    /// Filter text (for matching user input)
    pub filter_text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompletionKind {
    Keyword,
    Schema,
    Table,
    View,
    Column,
    Function,
    Type,
    Snippet,
}

impl CompletionKind {
    pub fn icon_name(&self) -> &'static str {
        match self {
            Self::Keyword => "keyword",
            Self::Schema => "folder",
            Self::Table => "table",
            Self::View => "eye",
            Self::Column => "columns",
            Self::Function => "function",
            Self::Type => "type",
            Self::Snippet => "snippet",
        }
    }
}

/// Context for autocomplete
#[derive(Clone, Debug)]
pub struct CompletionContext {
    /// Text before cursor
    pub prefix: String,
    /// Current word being typed
    pub word: String,
    /// Trigger character (if any)
    pub trigger: Option<char>,
    /// Cursor offset
    pub offset: usize,
    /// Full SQL text
    pub sql: String,
}

/// Autocomplete provider
pub struct AutocompleteProvider {
    schema_cache: Arc<SchemaCache>,
}

impl AutocompleteProvider {
    pub fn new(schema_cache: Arc<SchemaCache>) -> Self {
        Self { schema_cache }
    }

    /// Get completions for context
    pub fn get_completions(&self, ctx: &CompletionContext) -> Vec<Completion> {
        let mut completions = Vec::new();
        let context_type = self.analyze_context(ctx);

        match context_type {
            ContextType::SchemaQualified { schema } => {
                completions.extend(self.get_tables_in_schema(&schema, &ctx.word));
                completions.extend(self.get_functions_in_schema(&schema, &ctx.word));
            }
            ContextType::TableQualified { table, alias } => {
                completions.extend(self.get_columns_for_table(&table, &ctx.word));
            }
            ContextType::AfterFrom | ContextType::AfterJoin => {
                completions.extend(self.get_schema_completions(&ctx.word));
                completions.extend(self.get_table_completions(&ctx.word));
            }
            ContextType::AfterSelect | ContextType::AfterWhere | ContextType::AfterOn => {
                let tables = self.extract_tables_from_query(&ctx.sql);
                let aliases = self.extract_aliases_from_query(&ctx.sql);
                completions.extend(self.get_column_completions(&tables, &aliases, &ctx.word));
                completions.extend(self.get_function_completions(&ctx.word));
            }
            ContextType::General => {
                completions.extend(self.get_keyword_completions(&ctx.word));
                completions.extend(self.get_schema_completions(&ctx.word));
                completions.extend(self.get_table_completions(&ctx.word));
                completions.extend(self.get_function_completions(&ctx.word));
                completions.extend(self.get_snippet_completions(&ctx.word));
            }
        }

        // Sort by priority and filter text
        completions.sort_by(|a, b| {
            a.sort_priority.cmp(&b.sort_priority)
                .then_with(|| a.filter_text.cmp(&b.filter_text))
        });

        completions
    }

    fn analyze_context(&self, ctx: &CompletionContext) -> ContextType {
        let prefix_lower = ctx.prefix.to_lowercase();

        // Check for qualified name (schema.table or table.column)
        if ctx.trigger == Some('.') || ctx.prefix.ends_with('.') {
            let before_dot = ctx.prefix.trim_end_matches('.').split_whitespace().last().unwrap_or("");

            // Check if it's a schema name
            if self.schema_cache.has_schema(before_dot) {
                return ContextType::SchemaQualified { schema: before_dot.to_string() };
            }

            // Check if it's a table name or alias
            return ContextType::TableQualified {
                table: before_dot.to_string(),
                alias: None,
            };
        }

        // Check SQL context keywords
        if self.ends_with_keyword(&prefix_lower, &["from", "join", "into"]) {
            return ContextType::AfterFrom;
        }

        if self.ends_with_keyword(&prefix_lower, &["left", "right", "inner", "outer", "cross", "full"]) {
            // Likely before JOIN
            return ContextType::General;
        }

        if self.ends_with_keyword(&prefix_lower, &["select", "distinct"]) {
            return ContextType::AfterSelect;
        }

        if self.ends_with_keyword(&prefix_lower, &["where", "and", "or", "not"]) {
            return ContextType::AfterWhere;
        }

        if self.ends_with_keyword(&prefix_lower, &["on"]) {
            return ContextType::AfterOn;
        }

        ContextType::General
    }

    fn ends_with_keyword(&self, text: &str, keywords: &[&str]) -> bool {
        let last_word = text.split_whitespace().last().unwrap_or("");
        keywords.iter().any(|kw| last_word == *kw)
    }

    fn get_keyword_completions(&self, prefix: &str) -> Vec<Completion> {
        let keywords = [
            ("SELECT", "Query data from tables"),
            ("FROM", "Specify source tables"),
            ("WHERE", "Filter rows"),
            ("AND", "Logical AND"),
            ("OR", "Logical OR"),
            ("ORDER BY", "Sort results"),
            ("GROUP BY", "Group rows"),
            ("HAVING", "Filter groups"),
            ("LIMIT", "Limit number of rows"),
            ("OFFSET", "Skip rows"),
            ("JOIN", "Join tables"),
            ("LEFT JOIN", "Left outer join"),
            ("RIGHT JOIN", "Right outer join"),
            ("INNER JOIN", "Inner join"),
            ("FULL JOIN", "Full outer join"),
            ("INSERT INTO", "Insert rows"),
            ("VALUES", "Specify values"),
            ("UPDATE", "Update rows"),
            ("SET", "Set column values"),
            ("DELETE FROM", "Delete rows"),
            ("CREATE TABLE", "Create new table"),
            ("ALTER TABLE", "Modify table"),
            ("DROP TABLE", "Delete table"),
            ("WITH", "Common table expression"),
            ("UNION", "Combine results"),
            ("DISTINCT", "Remove duplicates"),
            ("AS", "Alias"),
            ("ON", "Join condition"),
            ("RETURNING", "Return affected rows"),
        ];

        keywords.iter()
            .filter(|(kw, _)| kw.to_lowercase().starts_with(&prefix.to_lowercase()))
            .map(|(kw, doc)| Completion {
                label: kw.to_string(),
                insert_text: kw.to_string(),
                kind: CompletionKind::Keyword,
                detail: Some("keyword".to_string()),
                documentation: Some(doc.to_string()),
                sort_priority: 100,
                filter_text: kw.to_lowercase(),
            })
            .collect()
    }

    fn get_schema_completions(&self, prefix: &str) -> Vec<Completion> {
        self.schema_cache.get_schema_names()
            .into_iter()
            .filter(|name| name.to_lowercase().starts_with(&prefix.to_lowercase()))
            .map(|name| Completion {
                label: name.clone(),
                insert_text: name.clone(),
                kind: CompletionKind::Schema,
                detail: Some("schema".to_string()),
                documentation: None,
                sort_priority: 20,
                filter_text: name.to_lowercase(),
            })
            .collect()
    }

    fn get_table_completions(&self, prefix: &str) -> Vec<Completion> {
        self.schema_cache.get_all_tables()
            .into_iter()
            .filter(|table| table.name.to_lowercase().starts_with(&prefix.to_lowercase()))
            .map(|table| {
                let detail = format!(
                    "{}{}",
                    table.schema,
                    table.row_estimate.map(|r| format!(" (~{} rows)", r)).unwrap_or_default()
                );
                Completion {
                    label: table.name.clone(),
                    insert_text: table.name.clone(),
                    kind: if table.is_view { CompletionKind::View } else { CompletionKind::Table },
                    detail: Some(detail),
                    documentation: table.comment.clone(),
                    sort_priority: 10,
                    filter_text: table.name.to_lowercase(),
                }
            })
            .collect()
    }

    fn get_tables_in_schema(&self, schema: &str, prefix: &str) -> Vec<Completion> {
        self.schema_cache.get_tables_in_schema(schema)
            .into_iter()
            .filter(|table| table.name.to_lowercase().starts_with(&prefix.to_lowercase()))
            .map(|table| Completion {
                label: table.name.clone(),
                insert_text: table.name.clone(),
                kind: if table.is_view { CompletionKind::View } else { CompletionKind::Table },
                detail: table.row_estimate.map(|r| format!("~{} rows", r)),
                documentation: table.comment.clone(),
                sort_priority: 10,
                filter_text: table.name.to_lowercase(),
            })
            .collect()
    }

    fn get_column_completions(
        &self,
        tables: &[String],
        aliases: &[(String, String)],
        prefix: &str,
    ) -> Vec<Completion> {
        let mut completions = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for table_ref in tables {
            if let Some(table) = self.schema_cache.find_table(table_ref) {
                for column in &table.columns {
                    if !column.name.to_lowercase().starts_with(&prefix.to_lowercase()) {
                        continue;
                    }

                    let key = format!("{}.{}", table.name, column.name);
                    if seen.contains(&key) {
                        continue;
                    }
                    seen.insert(key);

                    completions.push(Completion {
                        label: column.name.clone(),
                        insert_text: column.name.clone(),
                        kind: CompletionKind::Column,
                        detail: Some(format!("{} ({})", column.data_type, table.name)),
                        documentation: column.comment.clone(),
                        sort_priority: 5,
                        filter_text: column.name.to_lowercase(),
                    });
                }
            }
        }

        completions
    }

    fn get_columns_for_table(&self, table_ref: &str, prefix: &str) -> Vec<Completion> {
        if let Some(table) = self.schema_cache.find_table(table_ref) {
            table.columns.iter()
                .filter(|col| col.name.to_lowercase().starts_with(&prefix.to_lowercase()))
                .map(|col| Completion {
                    label: col.name.clone(),
                    insert_text: col.name.clone(),
                    kind: CompletionKind::Column,
                    detail: Some(col.data_type.clone()),
                    documentation: col.comment.clone(),
                    sort_priority: 5,
                    filter_text: col.name.to_lowercase(),
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    fn get_function_completions(&self, prefix: &str) -> Vec<Completion> {
        self.schema_cache.get_all_functions()
            .into_iter()
            .filter(|f| f.name.to_lowercase().starts_with(&prefix.to_lowercase()))
            .map(|f| {
                let insert = format!("{}($0)", f.name);
                Completion {
                    label: f.name.clone(),
                    insert_text: insert,
                    kind: CompletionKind::Function,
                    detail: Some(format!("→ {}", f.return_type)),
                    documentation: Some(format!(
                        "{}({})\n\n{}",
                        f.name,
                        f.arguments,
                        f.comment.as_deref().unwrap_or("")
                    )),
                    sort_priority: 30,
                    filter_text: f.name.to_lowercase(),
                }
            })
            .collect()
    }

    fn get_functions_in_schema(&self, schema: &str, prefix: &str) -> Vec<Completion> {
        self.schema_cache.get_functions_in_schema(schema)
            .into_iter()
            .filter(|f| f.name.to_lowercase().starts_with(&prefix.to_lowercase()))
            .map(|f| Completion {
                label: f.name.clone(),
                insert_text: format!("{}($0)", f.name),
                kind: CompletionKind::Function,
                detail: Some(format!("→ {}", f.return_type)),
                documentation: f.comment.clone(),
                sort_priority: 30,
                filter_text: f.name.to_lowercase(),
            })
            .collect()
    }

    fn get_snippet_completions(&self, prefix: &str) -> Vec<Completion> {
        let snippets = [
            ("sel", "SELECT * FROM $1 WHERE $2", "SELECT template"),
            ("selc", "SELECT COUNT(*) FROM $1", "SELECT COUNT template"),
            ("ins", "INSERT INTO $1 ($2) VALUES ($3)", "INSERT template"),
            ("upd", "UPDATE $1 SET $2 WHERE $3", "UPDATE template"),
            ("del", "DELETE FROM $1 WHERE $2", "DELETE template"),
            ("cte", "WITH $1 AS (\n  $2\n)\nSELECT * FROM $1", "CTE template"),
            ("join", "$1 JOIN $2 ON $3", "JOIN template"),
            ("case", "CASE\n  WHEN $1 THEN $2\n  ELSE $3\nEND", "CASE template"),
        ];

        snippets.iter()
            .filter(|(trigger, _, _)| trigger.starts_with(&prefix.to_lowercase()))
            .map(|(trigger, body, doc)| Completion {
                label: trigger.to_string(),
                insert_text: body.to_string(),
                kind: CompletionKind::Snippet,
                detail: Some("snippet".to_string()),
                documentation: Some(doc.to_string()),
                sort_priority: 50,
                filter_text: trigger.to_string(),
            })
            .collect()
    }

    fn extract_tables_from_query(&self, sql: &str) -> Vec<String> {
        let mut tables = Vec::new();
        let sql_lower = sql.to_lowercase();

        // Match FROM clause
        if let Some(from_idx) = sql_lower.find("from") {
            let after_from = &sql[from_idx + 4..];
            // Extract until WHERE, JOIN, GROUP, ORDER, LIMIT, or end
            let end_idx = ["where", "join", "group", "order", "limit", "having"]
                .iter()
                .filter_map(|kw| after_from.to_lowercase().find(kw))
                .min()
                .unwrap_or(after_from.len());

            let from_clause = &after_from[..end_idx];
            for part in from_clause.split(',') {
                if let Some(table) = part.split_whitespace().next() {
                    tables.push(table.to_string());
                }
            }
        }

        // Match JOIN tables
        for kw in ["join", "from"] {
            let parts: Vec<_> = sql_lower.match_indices(kw).collect();
            for (idx, _) in parts {
                let after = &sql[idx + kw.len()..];
                if let Some(table) = after.split_whitespace().next() {
                    if !["select", "on", "where", "("].contains(&table.to_lowercase().as_str()) {
                        tables.push(table.to_string());
                    }
                }
            }
        }

        tables.sort();
        tables.dedup();
        tables
    }

    fn extract_aliases_from_query(&self, sql: &str) -> Vec<(String, String)> {
        let mut aliases = Vec::new();
        let sql_lower = sql.to_lowercase();

        // Simple pattern: table AS alias or table alias
        let words: Vec<_> = sql.split_whitespace().collect();
        for i in 0..words.len().saturating_sub(1) {
            let word = words[i];
            let next = words.get(i + 1).map(|s| *s).unwrap_or("");

            if next.to_lowercase() == "as" {
                if let Some(alias) = words.get(i + 2) {
                    aliases.push((alias.to_string(), word.to_string()));
                }
            }
        }

        aliases
    }
}

#[derive(Debug)]
enum ContextType {
    General,
    AfterFrom,
    AfterJoin,
    AfterSelect,
    AfterWhere,
    AfterOn,
    SchemaQualified { schema: String },
    TableQualified { table: String, alias: Option<String> },
}
```

### 12.5 SQL Editor Component

```rust
// src/ui/editor/sql_editor.rs

use gpui::*;
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

use crate::ui::theme::Theme;
use super::buffer::Buffer;
use super::selection::{Position, Selection, Selections};
use super::highlight::{SyntaxHighlighter, HighlightedToken, token_color};
use super::autocomplete::{AutocompleteProvider, Completion, CompletionContext};
use crate::services::schema::SchemaCache;

/// SQL Editor component
pub struct SqlEditor {
    /// Text buffer
    buffer: Buffer,
    /// Cursor and selections
    selections: Selections,
    /// Connection ID for autocomplete
    connection_id: Option<Uuid>,
    /// Schema cache for autocomplete
    schema_cache: Arc<SchemaCache>,
    /// Autocomplete provider
    autocomplete_provider: AutocompleteProvider,
    /// Current autocomplete suggestions
    autocomplete_suggestions: Vec<Completion>,
    /// Selected autocomplete index
    autocomplete_index: usize,
    /// Show autocomplete menu
    show_autocomplete: bool,
    /// Syntax highlighter
    highlighter: SyntaxHighlighter,
    /// Cached highlight tokens
    highlight_cache: Vec<HighlightedToken>,
    /// Last highlight version
    highlight_version: u64,
    /// Error decorations
    errors: Vec<EditorError>,
    /// Scroll offset
    scroll_offset: Point<Pixels>,
    /// Visible line range
    visible_lines: std::ops::Range<usize>,
    /// Line height
    line_height: Pixels,
    /// Character width (monospace)
    char_width: Pixels,
    /// Editor focused
    focused: bool,
    /// Last cursor blink
    cursor_blink_time: Instant,
    /// Cursor visible (blink state)
    cursor_visible: bool,
    /// Read-only mode
    read_only: bool,
}

/// Error decoration in editor
#[derive(Clone, Debug)]
pub struct EditorError {
    pub position: usize,
    pub message: String,
    pub detail: Option<String>,
}

/// Events emitted by the editor
pub enum SqlEditorEvent {
    /// Content changed
    Changed { content: String },
    /// Execute requested (Cmd+Enter)
    Execute { sql: String, selection: bool },
    /// Execute all requested (Cmd+Shift+Enter)
    ExecuteAll { sql: String },
    /// Cancel requested (Cmd+.)
    Cancel,
    /// Save requested (Cmd+S)
    Save,
}

impl SqlEditor {
    pub fn new(schema_cache: Arc<SchemaCache>) -> Self {
        let provider = AutocompleteProvider::new(schema_cache.clone());

        Self {
            buffer: Buffer::new(),
            selections: Selections::cursor(Position::new(0, 0)),
            connection_id: None,
            schema_cache,
            autocomplete_provider: provider,
            autocomplete_suggestions: Vec::new(),
            autocomplete_index: 0,
            show_autocomplete: false,
            highlighter: SyntaxHighlighter::new(),
            highlight_cache: Vec::new(),
            highlight_version: 0,
            errors: Vec::new(),
            scroll_offset: Point::default(),
            visible_lines: 0..50,
            line_height: px(20.0),
            char_width: px(8.4),
            focused: false,
            cursor_blink_time: Instant::now(),
            cursor_visible: true,
            read_only: false,
        }
    }

    pub fn with_content(mut self, content: &str) -> Self {
        self.buffer.set_content(content);
        self.update_highlighting();
        self
    }

    pub fn with_connection(mut self, connection_id: Uuid) -> Self {
        self.connection_id = Some(connection_id);
        self
    }

    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    /// Get current content
    pub fn content(&self) -> String {
        self.buffer.content()
    }

    /// Set content
    pub fn set_content(&mut self, content: &str) {
        self.buffer.set_content(content);
        self.selections = Selections::cursor(Position::new(0, 0));
        self.update_highlighting();
        self.errors.clear();
    }

    /// Show error at position
    pub fn show_error(&mut self, position: usize, message: String, detail: Option<String>) {
        self.errors.push(EditorError {
            position,
            message,
            detail,
        });

        // Move cursor to error
        let (line, col) = self.buffer.offset_to_line_col(position);
        self.selections = Selections::cursor(Position::new(line, col));
    }

    /// Clear all errors
    pub fn clear_errors(&mut self) {
        self.errors.clear();
    }

    /// Get selected text
    pub fn selected_text(&self) -> Option<String> {
        let sel = self.selections.primary();
        if sel.is_cursor() {
            return None;
        }

        let start = self.buffer.line_col_to_offset(sel.start().line, sel.start().column);
        let end = self.buffer.line_col_to_offset(sel.end().line, sel.end().column);
        Some(self.buffer.slice(start..end))
    }

    /// Get current statement (statement containing cursor)
    pub fn current_statement(&self) -> String {
        let pos = self.selections.primary().head;
        let offset = self.buffer.line_col_to_offset(pos.line, pos.column);
        let content = self.buffer.content();

        // Find statement boundaries
        let mut start = 0;
        let mut end = content.len();

        // Find previous semicolon
        for (i, c) in content[..offset].char_indices().rev() {
            if c == ';' {
                start = i + 1;
                break;
            }
        }

        // Find next semicolon
        for (i, c) in content[offset..].char_indices() {
            if c == ';' {
                end = offset + i + 1;
                break;
            }
        }

        content[start..end].trim().to_string()
    }

    /// Update syntax highlighting
    fn update_highlighting(&mut self) {
        let version = self.buffer.version();
        if version == self.highlight_version {
            return;
        }

        let content = self.buffer.content();
        self.highlight_cache = if let Some(tree) = self.buffer.syntax_tree() {
            self.highlighter.highlight_tree(&tree, &content)
        } else {
            self.highlighter.highlight_regex(&content)
        };
        self.highlight_version = version;
    }

    /// Update autocomplete
    fn update_autocomplete(&mut self) {
        if self.read_only {
            self.show_autocomplete = false;
            return;
        }

        let pos = self.selections.primary().head;
        let offset = self.buffer.line_col_to_offset(pos.line, pos.column);
        let content = self.buffer.content();

        // Get prefix (text before cursor on current line)
        let line_start = self.buffer.line_col_to_offset(pos.line, 0);
        let prefix = &content[line_start..offset];

        // Get current word
        let word_start = prefix.rfind(|c: char| !c.is_alphanumeric() && c != '_')
            .map(|i| i + 1)
            .unwrap_or(0);
        let word = &prefix[word_start..];

        // Check for trigger character
        let trigger = prefix.chars().last().filter(|&c| c == '.' || c == ' ' || c == '(');

        let ctx = CompletionContext {
            prefix: prefix.to_string(),
            word: word.to_string(),
            trigger,
            offset,
            sql: content,
        };

        self.autocomplete_suggestions = self.autocomplete_provider.get_completions(&ctx);
        self.autocomplete_index = 0;
        self.show_autocomplete = !self.autocomplete_suggestions.is_empty() && word.len() >= 1;
    }

    /// Apply selected autocomplete
    fn apply_autocomplete(&mut self, cx: &mut Context<Self>) {
        if !self.show_autocomplete || self.autocomplete_suggestions.is_empty() {
            return;
        }

        let completion = &self.autocomplete_suggestions[self.autocomplete_index];
        let pos = self.selections.primary().head;
        let offset = self.buffer.line_col_to_offset(pos.line, pos.column);
        let content = self.buffer.content();

        // Find word start
        let line_start = self.buffer.line_col_to_offset(pos.line, 0);
        let prefix = &content[line_start..offset];
        let word_start = prefix.rfind(|c: char| !c.is_alphanumeric() && c != '_')
            .map(|i| line_start + i + 1)
            .unwrap_or(line_start);

        // Replace word with completion
        self.buffer.replace(word_start..offset, &completion.insert_text);

        // Update cursor position
        let new_offset = word_start + completion.insert_text.len();
        let (new_line, new_col) = self.buffer.offset_to_line_col(new_offset);
        self.selections = Selections::cursor(Position::new(new_line, new_col));

        self.show_autocomplete = false;
        self.update_highlighting();
        cx.notify();
    }

    /// Handle key input
    fn handle_key(&mut self, key: &str, modifiers: Modifiers, cx: &mut Context<Self>) {
        let cmd = modifiers.platform;
        let shift = modifiers.shift;
        let alt = modifiers.alt;

        // Handle autocomplete navigation
        if self.show_autocomplete {
            match key {
                "up" => {
                    if self.autocomplete_index > 0 {
                        self.autocomplete_index -= 1;
                    }
                    cx.notify();
                    return;
                }
                "down" => {
                    if self.autocomplete_index < self.autocomplete_suggestions.len() - 1 {
                        self.autocomplete_index += 1;
                    }
                    cx.notify();
                    return;
                }
                "enter" | "tab" => {
                    self.apply_autocomplete(cx);
                    return;
                }
                "escape" => {
                    self.show_autocomplete = false;
                    cx.notify();
                    return;
                }
                _ => {}
            }
        }

        // Command shortcuts
        if cmd {
            match key {
                "enter" if shift => {
                    let sql = self.buffer.content();
                    cx.emit(SqlEditorEvent::ExecuteAll { sql });
                    return;
                }
                "enter" => {
                    let sql = self.selected_text()
                        .unwrap_or_else(|| self.current_statement());
                    let selection = self.selected_text().is_some();
                    cx.emit(SqlEditorEvent::Execute { sql, selection });
                    return;
                }
                "." => {
                    cx.emit(SqlEditorEvent::Cancel);
                    return;
                }
                "s" => {
                    cx.emit(SqlEditorEvent::Save);
                    return;
                }
                "z" if shift => {
                    self.buffer.redo();
                    self.update_highlighting();
                    cx.notify();
                    return;
                }
                "z" => {
                    self.buffer.undo();
                    self.update_highlighting();
                    cx.notify();
                    return;
                }
                "a" => {
                    // Select all
                    let end_line = self.buffer.line_count().saturating_sub(1);
                    let end_col = self.buffer.line(end_line).map(|l| l.len()).unwrap_or(0);
                    self.selections = Selections::new(Selection::new(
                        Position::new(0, 0),
                        Position::new(end_line, end_col),
                    ));
                    cx.notify();
                    return;
                }
                "/" => {
                    self.toggle_comment(cx);
                    return;
                }
                _ => {}
            }
        }

        if self.read_only {
            // Only allow navigation in read-only mode
            self.handle_navigation(key, shift, cx);
            return;
        }

        // Text editing
        match key {
            "backspace" => {
                let sel = self.selections.primary();
                if !sel.is_cursor() {
                    let start = self.buffer.line_col_to_offset(sel.start().line, sel.start().column);
                    let end = self.buffer.line_col_to_offset(sel.end().line, sel.end().column);
                    self.buffer.delete(start..end);
                    self.selections = Selections::cursor(sel.start());
                } else if sel.head.column > 0 {
                    let offset = self.buffer.line_col_to_offset(sel.head.line, sel.head.column);
                    self.buffer.delete(offset - 1..offset);
                    self.selections = Selections::cursor(Position::new(sel.head.line, sel.head.column - 1));
                } else if sel.head.line > 0 {
                    let prev_line_len = self.buffer.line(sel.head.line - 1).map(|l| l.trim_end().len()).unwrap_or(0);
                    let offset = self.buffer.line_col_to_offset(sel.head.line, 0);
                    self.buffer.delete(offset - 1..offset);
                    self.selections = Selections::cursor(Position::new(sel.head.line - 1, prev_line_len));
                }
                self.update_highlighting();
                self.update_autocomplete();
                self.clear_errors();
                cx.emit(SqlEditorEvent::Changed { content: self.buffer.content() });
                cx.notify();
            }
            "delete" => {
                let sel = self.selections.primary();
                if !sel.is_cursor() {
                    let start = self.buffer.line_col_to_offset(sel.start().line, sel.start().column);
                    let end = self.buffer.line_col_to_offset(sel.end().line, sel.end().column);
                    self.buffer.delete(start..end);
                    self.selections = Selections::cursor(sel.start());
                } else {
                    let offset = self.buffer.line_col_to_offset(sel.head.line, sel.head.column);
                    if offset < self.buffer.len() {
                        self.buffer.delete(offset..offset + 1);
                    }
                }
                self.update_highlighting();
                self.update_autocomplete();
                self.clear_errors();
                cx.emit(SqlEditorEvent::Changed { content: self.buffer.content() });
                cx.notify();
            }
            "enter" => {
                self.insert_text("\n", cx);
            }
            "tab" => {
                self.insert_text("  ", cx);
            }
            _ => {
                self.handle_navigation(key, shift, cx);
            }
        }
    }

    fn handle_navigation(&mut self, key: &str, shift: bool, cx: &mut Context<Self>) {
        let sel = self.selections.primary_mut();

        match key {
            "left" => {
                if sel.head.column > 0 {
                    sel.head.column -= 1;
                } else if sel.head.line > 0 {
                    sel.head.line -= 1;
                    sel.head.column = self.buffer.line(sel.head.line).map(|l| l.len()).unwrap_or(0);
                }
                if !shift {
                    sel.anchor = sel.head;
                }
                cx.notify();
            }
            "right" => {
                let line_len = self.buffer.line(sel.head.line).map(|l| l.len()).unwrap_or(0);
                if sel.head.column < line_len {
                    sel.head.column += 1;
                } else if sel.head.line < self.buffer.line_count() - 1 {
                    sel.head.line += 1;
                    sel.head.column = 0;
                }
                if !shift {
                    sel.anchor = sel.head;
                }
                cx.notify();
            }
            "up" => {
                if sel.head.line > 0 {
                    sel.head.line -= 1;
                    let line_len = self.buffer.line(sel.head.line).map(|l| l.len()).unwrap_or(0);
                    sel.head.column = sel.goal_column.unwrap_or(sel.head.column).min(line_len);
                }
                if !shift {
                    sel.anchor = sel.head;
                }
                cx.notify();
            }
            "down" => {
                if sel.head.line < self.buffer.line_count() - 1 {
                    sel.head.line += 1;
                    let line_len = self.buffer.line(sel.head.line).map(|l| l.len()).unwrap_or(0);
                    sel.head.column = sel.goal_column.unwrap_or(sel.head.column).min(line_len);
                }
                if !shift {
                    sel.anchor = sel.head;
                }
                cx.notify();
            }
            "home" => {
                sel.head.column = 0;
                if !shift {
                    sel.anchor = sel.head;
                }
                cx.notify();
            }
            "end" => {
                let line_len = self.buffer.line(sel.head.line).map(|l| l.len()).unwrap_or(0);
                sel.head.column = line_len;
                if !shift {
                    sel.anchor = sel.head;
                }
                cx.notify();
            }
            _ => {}
        }
    }

    /// Insert text at cursor
    fn insert_text(&mut self, text: &str, cx: &mut Context<Self>) {
        let sel = self.selections.primary();

        // Delete selection if any
        if !sel.is_cursor() {
            let start = self.buffer.line_col_to_offset(sel.start().line, sel.start().column);
            let end = self.buffer.line_col_to_offset(sel.end().line, sel.end().column);
            self.buffer.delete(start..end);
            self.selections = Selections::cursor(sel.start());
        }

        let pos = self.selections.primary().head;
        let offset = self.buffer.line_col_to_offset(pos.line, pos.column);
        self.buffer.insert(offset, text);

        // Update cursor
        let new_offset = offset + text.len();
        let (new_line, new_col) = self.buffer.offset_to_line_col(new_offset);
        self.selections = Selections::cursor(Position::new(new_line, new_col));

        self.update_highlighting();
        self.update_autocomplete();
        self.clear_errors();
        cx.emit(SqlEditorEvent::Changed { content: self.buffer.content() });
        cx.notify();
    }

    /// Toggle line comment
    fn toggle_comment(&mut self, cx: &mut Context<Self>) {
        let sel = self.selections.primary();
        let start_line = sel.start().line;
        let end_line = sel.end().line;

        for line_num in start_line..=end_line {
            if let Some(line) = self.buffer.line(line_num) {
                let trimmed = line.trim_start();
                let indent = line.len() - trimmed.len();
                let line_start = self.buffer.line_col_to_offset(line_num, 0);

                if trimmed.starts_with("--") {
                    // Remove comment
                    let comment_start = line_start + indent;
                    self.buffer.delete(comment_start..comment_start + 2);
                    if trimmed.starts_with("-- ") {
                        self.buffer.delete(comment_start..comment_start + 1);
                    }
                } else {
                    // Add comment
                    let insert_pos = line_start + indent;
                    self.buffer.insert(insert_pos, "-- ");
                }
            }
        }

        self.update_highlighting();
        cx.emit(SqlEditorEvent::Changed { content: self.buffer.content() });
        cx.notify();
    }

    /// Render a single line
    fn render_line(&self, line_num: usize, theme: &Theme) -> impl IntoElement {
        let line_text = self.buffer.line(line_num).unwrap_or_default();
        let line_start = self.buffer.line_col_to_offset(line_num, 0);
        let line_end = line_start + line_text.len();

        // Get tokens for this line
        let line_tokens: Vec<_> = self.highlight_cache.iter()
            .filter(|t| t.range.start < line_end && t.range.end > line_start)
            .collect();

        // Build styled spans
        let mut spans: Vec<(String, Hsla)> = Vec::new();
        let mut pos = 0;

        for token in line_tokens {
            let token_start = token.range.start.saturating_sub(line_start);
            let token_end = (token.range.end - line_start).min(line_text.len());

            // Add unstyled text before token
            if token_start > pos {
                spans.push((
                    line_text[pos..token_start].to_string(),
                    theme.syntax_identifier,
                ));
            }

            // Add token
            if token_end > token_start {
                spans.push((
                    line_text[token_start..token_end].to_string(),
                    token_color(token.token_type, theme),
                ));
            }

            pos = token_end;
        }

        // Add remaining text
        if pos < line_text.len() {
            spans.push((
                line_text[pos..].to_string(),
                theme.syntax_identifier,
            ));
        }

        // Handle empty line
        if spans.is_empty() {
            spans.push((" ".to_string(), theme.text_muted));
        }

        div()
            .h(self.line_height)
            .flex()
            .items_center()
            .children(
                spans.into_iter().map(|(text, color)| {
                    div()
                        .text_color(color)
                        .font_family("monospace")
                        .text_size(px(14.0))
                        .child(text)
                })
            )
    }
}

impl Render for SqlEditor {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        // Update cursor blink
        if self.cursor_blink_time.elapsed() > Duration::from_millis(500) {
            self.cursor_visible = !self.cursor_visible;
            self.cursor_blink_time = Instant::now();
        }

        let line_count = self.buffer.line_count();
        let gutter_width = px(50.0);

        div()
            .size_full()
            .flex()
            .bg(theme.editor_background)
            .font_family("monospace")
            .text_size(px(14.0))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, cx| {
                this.handle_key(&event.keystroke.key, event.keystroke.modifiers, cx);
            }))
            .on_click(cx.listener(|this, event: &ClickEvent, cx| {
                this.focused = true;
                cx.focus_self();
                cx.notify();
            }))
            // Line numbers gutter
            .child(
                div()
                    .w(gutter_width)
                    .flex_shrink_0()
                    .flex()
                    .flex_col()
                    .bg(theme.gutter_background)
                    .border_r_1()
                    .border_color(theme.border)
                    .children(
                        (0..line_count).map(|i| {
                            div()
                                .h(self.line_height)
                                .px(px(8.0))
                                .flex()
                                .items_center()
                                .justify_end()
                                .text_color(theme.line_number)
                                .text_size(px(12.0))
                                .child((i + 1).to_string())
                        })
                    )
            )
            // Editor content
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .relative()
                    // Lines
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .pl(px(8.0))
                            .children(
                                (0..line_count).map(|i| {
                                    self.render_line(i, theme)
                                })
                            )
                    )
                    // Cursor
                    .when(self.focused && self.cursor_visible, |el| {
                        let pos = self.selections.primary().head;
                        let x = pos.column as f32 * self.char_width.0 + 8.0;
                        let y = pos.line as f32 * self.line_height.0;

                        el.child(
                            div()
                                .absolute()
                                .left(px(x))
                                .top(px(y))
                                .w(px(2.0))
                                .h(self.line_height)
                                .bg(theme.cursor)
                        )
                    })
                    // Selection highlight
                    .when(!self.selections.primary().is_cursor(), |el| {
                        let sel = self.selections.primary();
                        let start = sel.start();
                        let end = sel.end();

                        // Simple single-line selection for now
                        if start.line == end.line {
                            let x = start.column as f32 * self.char_width.0 + 8.0;
                            let y = start.line as f32 * self.line_height.0;
                            let width = (end.column - start.column) as f32 * self.char_width.0;

                            el.child(
                                div()
                                    .absolute()
                                    .left(px(x))
                                    .top(px(y))
                                    .w(px(width))
                                    .h(self.line_height)
                                    .bg(theme.selection)
                            )
                        } else {
                            el
                        }
                    })
                    // Autocomplete popup
                    .when(self.show_autocomplete, |el| {
                        let pos = self.selections.primary().head;
                        let x = pos.column as f32 * self.char_width.0 + 8.0;
                        let y = (pos.line + 1) as f32 * self.line_height.0;

                        el.child(
                            div()
                                .absolute()
                                .left(px(x))
                                .top(px(y))
                                .w(px(300.0))
                                .max_h(px(200.0))
                                .overflow_y_auto()
                                .bg(theme.popup_background)
                                .border_1()
                                .border_color(theme.border)
                                .rounded(px(4.0))
                                .shadow_lg()
                                .children(
                                    self.autocomplete_suggestions.iter().enumerate().map(|(i, item)| {
                                        let selected = i == self.autocomplete_index;
                                        div()
                                            .px(px(8.0))
                                            .py(px(4.0))
                                            .flex()
                                            .items_center()
                                            .gap(px(8.0))
                                            .when(selected, |el| el.bg(theme.selection))
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(theme.text)
                                                    .child(item.label.clone())
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(theme.text_muted)
                                                    .child(item.detail.clone().unwrap_or_default())
                                            )
                                    })
                                )
                        )
                    })
            )
    }
}

impl EventEmitter<SqlEditorEvent> for SqlEditor {}
```

### 12.6 Editor Toolbar

```rust
// src/ui/editor/toolbar.rs

use gpui::*;
use uuid::Uuid;

use crate::ui::theme::Theme;
use crate::ui::components::{Button, ButtonStyle, ButtonSize, Icon, IconName, Select};

/// Editor toolbar component
pub struct EditorToolbar {
    connection_id: Option<Uuid>,
    connections: Vec<ConnectionOption>,
    is_executing: bool,
    row_limit: u64,
}

#[derive(Clone)]
pub struct ConnectionOption {
    pub id: Uuid,
    pub name: String,
}

pub enum EditorToolbarEvent {
    Run,
    RunAll,
    Stop,
    Format,
    Save,
    ConnectionChanged(Uuid),
    RowLimitChanged(u64),
}

impl EditorToolbar {
    pub fn new() -> Self {
        Self {
            connection_id: None,
            connections: Vec::new(),
            is_executing: false,
            row_limit: 1000,
        }
    }

    pub fn set_connections(&mut self, connections: Vec<ConnectionOption>) {
        self.connections = connections;
    }

    pub fn set_connection(&mut self, id: Option<Uuid>) {
        self.connection_id = id;
    }

    pub fn set_executing(&mut self, executing: bool) {
        self.is_executing = executing;
    }
}

impl Render for EditorToolbar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let row_limits = [100, 500, 1000, 5000, 10000, 50000, 0];

        div()
            .h(px(44.0))
            .w_full()
            .px(px(12.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .bg(theme.surface)
            .border_b_1()
            .border_color(theme.border)
            // Run buttons
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .child(
                        Button::new("run")
                            .label("Run")
                            .icon(IconName::Play)
                            .style(ButtonStyle::Primary)
                            .disabled(self.is_executing || self.connection_id.is_none())
                            .on_click(cx.listener(|_, _, cx| {
                                cx.emit(EditorToolbarEvent::Run);
                            }))
                    )
                    .child(
                        Button::new("run-all")
                            .label("Run All")
                            .style(ButtonStyle::Secondary)
                            .disabled(self.is_executing || self.connection_id.is_none())
                            .on_click(cx.listener(|_, _, cx| {
                                cx.emit(EditorToolbarEvent::RunAll);
                            }))
                    )
                    .when(self.is_executing, |el| {
                        el.child(
                            Button::new("stop")
                                .label("Stop")
                                .icon(IconName::Square)
                                .style(ButtonStyle::Danger)
                                .on_click(cx.listener(|_, _, cx| {
                                    cx.emit(EditorToolbarEvent::Stop);
                                }))
                        )
                    })
            )
            // Separator
            .child(
                div()
                    .w(px(1.0))
                    .h(px(24.0))
                    .bg(theme.border)
            )
            // Format and Save
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .child(
                        Button::new("format")
                            .icon(IconName::Code)
                            .tooltip("Format SQL")
                            .style(ButtonStyle::Ghost)
                            .on_click(cx.listener(|_, _, cx| {
                                cx.emit(EditorToolbarEvent::Format);
                            }))
                    )
                    .child(
                        Button::new("save")
                            .icon(IconName::Save)
                            .tooltip("Save (Cmd+S)")
                            .style(ButtonStyle::Ghost)
                            .on_click(cx.listener(|_, _, cx| {
                                cx.emit(EditorToolbarEvent::Save);
                            }))
                    )
            )
            // Spacer
            .child(div().flex_1())
            // Connection selector
            .child(
                Select::new("connection")
                    .placeholder("Select connection...")
                    .value(self.connection_id.map(|id| id.to_string()))
                    .options(
                        self.connections.iter()
                            .map(|c| (c.id.to_string(), c.name.clone()))
                            .collect()
                    )
                    .on_change(cx.listener(|this, value: &str, cx| {
                        if let Ok(id) = Uuid::parse_str(value) {
                            cx.emit(EditorToolbarEvent::ConnectionChanged(id));
                        }
                    }))
            )
            // Row limit selector
            .child(
                Select::new("row-limit")
                    .value(self.row_limit.to_string())
                    .options(
                        row_limits.iter()
                            .map(|&limit| {
                                let label = if limit == 0 {
                                    "No limit".to_string()
                                } else {
                                    format!("Limit: {}", limit)
                                };
                                (limit.to_string(), label)
                            })
                            .collect()
                    )
                    .on_change(cx.listener(|this, value: &str, cx| {
                        if let Ok(limit) = value.parse() {
                            this.row_limit = limit;
                            cx.emit(EditorToolbarEvent::RowLimitChanged(limit));
                        }
                    }))
            )
    }
}

impl EventEmitter<EditorToolbarEvent> for EditorToolbar {}
```

## Acceptance Criteria

1. **Text Editing**
   - Insert, delete, and replace text correctly
   - Multi-cursor support
   - Undo/redo with history
   - Line numbers display
   - Scroll for large documents

2. **Syntax Highlighting**
   - Keywords highlighted correctly (SELECT, FROM, WHERE, etc.)
   - Data types have distinct highlighting
   - Built-in functions are recognized
   - Strings, comments, and numbers properly colored
   - Dollar-quoted strings work correctly
   - Tree-sitter parsing when available, regex fallback

3. **Autocomplete**
   - Tables autocomplete after FROM/JOIN
   - Columns autocomplete in SELECT/WHERE context
   - Schema-qualified names work (schema.table)
   - Aliases recognized for column completion
   - Functions show signature in documentation
   - Keywords have lower priority than schema objects
   - <50ms response time

4. **Error Highlighting**
   - Errors from Postgres appear at correct positions
   - Error messages show on hover
   - Errors clear when user edits

5. **Keyboard Shortcuts**
   - Cmd/Ctrl+Enter executes current statement
   - Cmd/Ctrl+Shift+Enter executes all
   - Cmd/Ctrl+. cancels query
   - Cmd/Ctrl+S triggers save
   - Cmd/Ctrl+/ comments line
   - Cmd/Ctrl+Z undo, Cmd/Ctrl+Shift+Z redo

6. **Theme Support**
   - Light and dark themes work correctly
   - Theme changes update editor immediately

## Testing Instructions

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_insert() {
        let buffer = Buffer::from_str("SELECT * FROM users");
        buffer.insert(7, "id, name ");
        assert_eq!(buffer.content(), "SELECT id, name * FROM users");
    }

    #[test]
    fn test_buffer_delete() {
        let buffer = Buffer::from_str("SELECT * FROM users");
        buffer.delete(7..9);
        assert_eq!(buffer.content(), "SELECT FROM users");
    }

    #[test]
    fn test_buffer_undo_redo() {
        let buffer = Buffer::from_str("SELECT");
        buffer.insert(6, " *");
        assert_eq!(buffer.content(), "SELECT *");

        buffer.undo();
        assert_eq!(buffer.content(), "SELECT");

        buffer.redo();
        assert_eq!(buffer.content(), "SELECT *");
    }

    #[test]
    fn test_line_col_conversion() {
        let buffer = Buffer::from_str("line1\nline2\nline3");
        assert_eq!(buffer.line_col_to_offset(0, 0), 0);
        assert_eq!(buffer.line_col_to_offset(1, 0), 6);
        assert_eq!(buffer.line_col_to_offset(2, 3), 15);

        assert_eq!(buffer.offset_to_line_col(0), (0, 0));
        assert_eq!(buffer.offset_to_line_col(6), (1, 0));
        assert_eq!(buffer.offset_to_line_col(15), (2, 3));
    }

    #[test]
    fn test_syntax_highlighting() {
        let highlighter = SyntaxHighlighter::new();
        let tokens = highlighter.highlight_regex("SELECT id FROM users");

        assert!(!tokens.is_empty());
        assert_eq!(tokens[0].token_type, TokenType::Keyword); // SELECT
    }

    #[test]
    fn test_selection_ordering() {
        let sel = Selection::new(
            Position::new(5, 10),
            Position::new(2, 5),
        );
        assert_eq!(sel.start(), Position::new(2, 5));
        assert_eq!(sel.end(), Position::new(5, 10));
    }
}
```

## Dependencies

- ropey (rope data structure for efficient text editing)
- tree-sitter (incremental parsing)
- tree-sitter-sql (SQL grammar)
- parking_lot (synchronization)
- Feature 10: Schema Introspection (for autocomplete data)
