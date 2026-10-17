#![allow(non_snake_case)]

mod db;
mod schema;

use std::sync::Arc;
use dioxus::prelude::*;
use dioxus::desktop::{Config, WindowBuilder};
use lite_orm::Database;
use serde_json::Value;

use schema::Model;

const LOGO_BASE64: &str = include_str!("../assets/logo_square.b64");

const STYLE: &str = r#"
:root, .desktop-app.theme-light {
  color-scheme: light;
  --bg-app: #FFFEFA;
  --bg-sidebar: #FFFEFA;
  --bg-surface: #F5F4EE;
  --bg-surface-elevated: #EBEAE3;
  --bg-surface-hover: #E3E2DA;
  --bg-input: #FFFEFA;
  --border-subtle: rgba(0, 0, 0, 0.08);
  --border-default: rgba(0, 0, 0, 0.18);
  --border-focus: #000000;
  --text-main: #000000;
  --text-secondary: rgba(0, 0, 0, 0.72);
  --text-muted: rgba(0, 0, 0, 0.45);
  --accent: #000000;
  --accent-light: rgba(0, 0, 0, 0.06);
  --danger: #000000;
  --danger-light: rgba(0, 0, 0, 0.06);
  --warning: #000000;
  --font-sans: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  --font-mono: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace;
}

.desktop-app:not(.theme-light) {
  color-scheme: dark;
  --bg-app: #000000;
  --bg-sidebar: #000000;
  --bg-surface: #0e0e0e;
  --bg-surface-elevated: #181818;
  --bg-surface-hover: #222222;
  --bg-input: #000000;
  --border-subtle: rgba(255, 254, 250, 0.12);
  --border-default: rgba(255, 254, 250, 0.24);
  --border-focus: #FFFEFA;
  --text-main: #FFFEFA;
  --text-secondary: rgba(255, 254, 250, 0.75);
  --text-muted: rgba(255, 254, 250, 0.45);
  --accent: #FFFEFA;
  --accent-light: rgba(255, 254, 250, 0.10);
  --danger: #FFFEFA;
  --danger-light: rgba(255, 254, 250, 0.10);
  --warning: #FFFEFA;
}

*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
body {
  background-color: var(--bg-app);
  color: var(--text-main);
  font-family: var(--font-sans);
  font-size: 13px;
  line-height: 1.5;
  height: 100vh;
  width: 100vw;
  overflow: hidden;
  user-select: none;
}

.desktop-app {
  display: flex;
  height: 100vh;
  width: 100vw;
  overflow: hidden;
  background-color: var(--bg-app);
  color: var(--text-main);
}

/* Sidebar */
.sidebar {
  width: 280px;
  min-width: 260px;
  max-width: 380px;
  flex-shrink: 0;
  background: var(--bg-sidebar);
  border-right: 1px solid var(--border-default);
  display: flex;
  flex-direction: column;
}

.brand {
  height: 56px;
  min-height: 56px;
  max-height: 56px;
  box-sizing: border-box;
  padding: 0 16px;
  display: flex;
  align-items: center;
  gap: 12px;
  border-bottom: 1px solid var(--border-default);
  background: var(--bg-sidebar);
}

.brand-icon {
  width: 34px;
  height: 34px;
  min-width: 34px;
  border-radius: 8px;
  border: 1px solid var(--border-default);
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
  background: #FFFEFA;
}

.brand-logo-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}

.desktop-app:not(.theme-light) .brand-icon {
  background: #000000;
  border-color: rgba(255, 254, 250, 0.3);
}

.desktop-app:not(.theme-light) .brand-logo-img {
  filter: invert(1);
}

.brand-text {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.brand-title {
  font-size: 16px;
  font-weight: 800;
  color: var(--text-main);
  letter-spacing: -0.02em;
  line-height: 1.1;
}

.brand-badge {
  font-size: 9px;
  font-weight: 700;
  color: var(--text-muted);
  letter-spacing: 0.1em;
  font-family: var(--font-mono);
}

.nav-section {
  padding: 8px 12px;
  flex: 1;
  overflow-y: auto;
}

.nav-title {
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.08em;
  color: var(--text-muted);
  padding: 8px 8px 4px 8px;
}

.nav-item {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 7px 10px;
  background: transparent;
  border: none;
  border-radius: 6px;
  color: var(--text-secondary);
  font-size: 12.5px;
  font-weight: 500;
  cursor: pointer;
  text-align: left;
  transition: all 0.12s ease;
  min-width: 0;
}

.nav-item:hover {
  background: var(--bg-surface-hover);
  color: var(--text-main);
}

.nav-item.active {
  background: var(--bg-surface-elevated);
  color: var(--text-main);
  font-weight: 700;
}

.nav-label {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.nav-tag {
  margin-left: auto;
  font-size: 10px;
  font-family: var(--font-mono);
  color: var(--text-muted);
  background: var(--bg-surface);
  border: 1px solid var(--border-subtle);
  padding: 2px 7px;
  border-radius: 10px;
  flex-shrink: 0;
  white-space: nowrap;
  font-variant-numeric: tabular-nums;
}

.sidebar-footer {
  height: 44px;
  min-height: 44px;
  max-height: 44px;
  box-sizing: border-box;
  padding: 0 14px;
  border-top: 1px solid var(--border-default);
  display: flex;
  align-items: center;
  gap: 8px;
  background: var(--bg-sidebar);
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--text-main);
  border: 1px solid var(--border-default);
}

.status-text {
  font-size: 11px;
  color: var(--text-secondary);
  font-weight: 600;
}

.theme-toggle-btn {
  margin-left: auto;
  background: var(--bg-surface);
  border: 1px solid var(--border-default);
  border-radius: 6px;
  color: var(--text-main);
  font-size: 11px;
  font-weight: 700;
  padding: 4px 8px;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  gap: 4px;
  transition: all 0.12s ease;
}

.theme-toggle-btn:hover {
  background: var(--text-main);
  color: var(--bg-app);
}

/* Main Area */
.main-viewport {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: var(--bg-app);
}

/* App Header */
.top-bar {
  min-height: 56px;
  box-sizing: border-box;
  border-bottom: 1px solid var(--border-default);
  background: var(--bg-sidebar);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 20px;
  gap: 16px;
  flex-wrap: wrap;
}

.top-left {
  display: flex;
  align-items: center;
  gap: 16px;
}

.model-heading {
  font-size: 16px;
  font-weight: 700;
  color: var(--text-main);
  display: flex;
  align-items: center;
  gap: 8px;
}

.table-tag {
  font-size: 11px;
  font-family: var(--font-mono);
  background: var(--bg-surface);
  border: 1px solid var(--border-subtle);
  padding: 2px 6px;
  border-radius: 4px;
  color: var(--text-secondary);
}

.tab-group {
  display: flex;
  background: var(--bg-surface);
  border: 1px solid var(--border-default);
  border-radius: 6px;
  padding: 2px;
  gap: 2px;
}

.tab-btn {
  background: transparent;
  border: none;
  border-radius: 4px;
  color: var(--text-secondary);
  font-size: 12px;
  padding: 4px 10px;
  cursor: pointer;
  font-weight: 600;
  transition: all 0.12s ease;
}

.tab-btn:hover { color: var(--text-main); }
.tab-btn.active {
  background: var(--text-main);
  color: var(--bg-app);
  font-weight: 700;
}

.top-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  font-size: 12px;
  font-weight: 700;
  border-radius: 6px;
  border: 1px solid transparent;
  cursor: pointer;
  transition: all 0.12s ease;
}

.btn-primary {
  background: var(--text-main);
  color: var(--bg-app);
  border: 1px solid var(--text-main);
}
.btn-primary:hover {
  opacity: 0.88;
}

.btn-secondary {
  background: var(--bg-surface);
  border: 1px solid var(--border-default);
  color: var(--text-main);
}
.btn-secondary:hover {
  background: var(--bg-surface-hover);
  border-color: var(--text-main);
}

.btn-danger {
  background: transparent;
  border: 1px solid var(--border-default);
  color: var(--text-main);
  padding: 4px 8px;
  font-size: 11px;
}
.btn-danger:hover {
  background: var(--text-main);
  color: var(--bg-app);
}

/* Toolbar */
.toolbar {
  padding: 10px 20px;
  border-bottom: 1px solid var(--border-default);
  background: var(--bg-sidebar);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
}

.search-wrap {
  width: 260px;
}

.search-input {
  width: 100%;
  background: var(--bg-input);
  border: 1px solid var(--border-default);
  border-radius: 6px;
  padding: 6px 10px;
  color: var(--text-main);
  font-size: 12px;
  outline: none;
}
.search-input:focus {
  border-color: var(--border-focus);
}

.record-stat {
  font-size: 12px;
  color: var(--text-secondary);
}

/* Filter Bar */
.filter-bar {
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border-default);
  padding: 10px 20px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.filter-bar-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.filter-title {
  display: flex;
  align-items: center;
  gap: 8px;
}

.filter-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.filter-row {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.filter-select {
  background: var(--bg-input);
  border: 1px solid var(--border-default);
  border-radius: 6px;
  color: var(--text-main);
  padding: 5px 8px;
  font-size: 12px;
  outline: none;
}
.filter-select:focus {
  border-color: var(--border-focus);
}

.filter-input {
  background: var(--bg-input);
  border: 1px solid var(--border-default);
  border-radius: 6px;
  color: var(--text-main);
  padding: 5px 10px;
  font-size: 12px;
  outline: none;
  min-width: 180px;
}
.filter-input:focus {
  border-color: var(--border-focus);
}

.filter-remove-btn {
  background: transparent;
  border: 1px solid var(--border-default);
  border-radius: 4px;
  color: var(--text-main);
  font-size: 11px;
  font-weight: 700;
  padding: 4px 8px;
  cursor: pointer;
  transition: all 0.12s ease;
}
.filter-remove-btn:hover {
  background: var(--text-main);
  color: var(--bg-app);
}

.filter-empty-hint {
  font-size: 12px;
  color: var(--text-muted);
  font-style: italic;
  padding: 4px 0;
}

/* Table */
.table-area {
  flex: 1;
  overflow: auto;
  position: relative;
  min-width: 0;
  min-height: 0;
}

.data-grid {
  width: 100%;
  min-width: max-content;
  border-collapse: separate;
  border-spacing: 0;
  font-size: 12.5px;
  white-space: nowrap;
}

.data-grid th {
  position: sticky;
  top: 0;
  background: var(--bg-sidebar);
  border-bottom: 1px solid var(--border-default);
  border-right: 1px solid var(--border-subtle);
  color: var(--text-secondary);
  font-size: 11px;
  font-weight: 700;
  padding: 8px 12px;
  text-align: left;
  z-index: 2;
  cursor: pointer;
}
.data-grid th:hover {
  background: var(--bg-surface-hover);
  color: var(--text-main);
}

.data-grid td {
  padding: 8px 12px;
  border-bottom: 1px solid var(--border-subtle);
  border-right: 1px solid var(--border-subtle);
  color: var(--text-main);
  max-width: 300px;
  overflow: hidden;
  text-overflow: ellipsis;
}

.data-grid tbody tr:hover {
  background-color: var(--bg-surface-hover);
}

/* Skeleton Loading Structure */
@keyframes skeleton-shimmer {
  0% {
    background-position: -200% 0;
  }
  100% {
    background-position: 200% 0;
  }
}

.skeleton-shimmer {
  background: linear-gradient(90deg, rgba(0, 0, 0, 0.04) 25%, rgba(0, 0, 0, 0.12) 50%, rgba(0, 0, 0, 0.04) 75%);
  background-size: 200% 100%;
  animation: skeleton-shimmer 1.5s ease-in-out infinite;
  border-radius: 4px;
  display: inline-block;
  height: 14px;
  vertical-align: middle;
}

.desktop-app:not(.theme-light) .skeleton-shimmer {
  background: linear-gradient(90deg, rgba(255, 254, 250, 0.06) 25%, rgba(255, 254, 250, 0.18) 50%, rgba(255, 254, 250, 0.06) 75%);
  background-size: 200% 100%;
}

.skeleton-row td {
  padding: 10px 12px;
}

.skeleton-btn {
  width: 50px;
  height: 22px;
  border-radius: 4px;
}

/* Badges */
.badge-pk {
  background: var(--text-main);
  color: var(--bg-app);
  font-size: 9px;
  font-weight: 800;
  padding: 1px 5px;
  border-radius: 3px;
  margin-left: 4px;
  font-family: var(--font-mono);
}

.badge-type {
  font-size: 9px;
  font-family: var(--font-mono);
  font-weight: 700;
  padding: 1px 5px;
  border-radius: 3px;
  margin-right: 6px;
  text-transform: uppercase;
  background: transparent;
  color: var(--text-main);
  border: 1px solid var(--border-default);
}
.badge-int, .badge-other, .badge-bool, .badge-str, .badge-date {
  background: transparent;
  color: var(--text-main);
  border: 1px solid var(--border-default);
}

.badge-tag {
  display: inline-block;
  font-size: 9px;
  font-family: var(--font-mono);
  font-weight: 700;
  padding: 1px 6px;
  border-radius: 4px;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  background: transparent;
  color: var(--text-main);
  border: 1px solid var(--border-default);
  flex-shrink: 0;
  white-space: nowrap;
}

.badge-tag-table,
.badge-tag-sql,
.badge-tag-light,
.badge-tag-dark,
.badge-tag-relation,
.badge-tag-filter {
  background: transparent;
  color: var(--text-main);
  border: 1px solid var(--border-default);
}

/* Relation Pills (Prisma Studio style) */
.btn-relation-pill {
  background: var(--bg-surface);
  border: 1px solid var(--border-default);
  border-radius: 4px;
  padding: 2px 7px;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  transition: all 0.12s ease;
  color: var(--text-main);
  font-size: 11.5px;
}
.btn-relation-pill:hover {
  background: var(--bg-surface-hover);
  border-color: var(--text-main);
}

.relation-count-badge {
  font-weight: 800;
  font-family: var(--font-mono);
  font-size: 10px;
  background: var(--text-main);
  color: var(--bg-app);
  padding: 1px 5px;
  border-radius: 3px;
}

.relation-target-name {
  font-weight: 600;
  color: var(--text-main);
}

/* Connected Records Modal Overlay */
.relation-modal-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.60);
  backdrop-filter: blur(4px);
  z-index: 500;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 20px;
}

.relation-modal-dialog {
  background: var(--bg-sidebar);
  border: 1px solid var(--border-default);
  border-radius: 10px;
  width: 900px;
  max-width: 92vw;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  box-shadow: 0 20px 40px rgba(0, 0, 0, 0.4);
  overflow: hidden;
}

.relation-modal-header {
  padding: 14px 20px;
  border-bottom: 1px solid var(--border-default);
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.relation-modal-body {
  flex: 1;
  overflow: auto;
  padding: 0;
  min-height: 220px;
}

.relation-modal-footer {
  padding: 12px 20px;
  border-top: 1px solid var(--border-default);
  background: var(--bg-surface);
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.val-null { color: var(--text-muted); font-style: italic; }
.val-true { color: var(--text-main); font-weight: 800; font-family: var(--font-mono); }
.val-false { color: var(--text-muted); font-family: var(--font-mono); }

/* Table Footer */
.table-footer {
  height: 44px;
  min-height: 44px;
  max-height: 44px;
  box-sizing: border-box;
  border-top: 1px solid var(--border-default);
  background: var(--bg-sidebar);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 20px;
  font-size: 12px;
  color: var(--text-secondary);
}

/* Add Record Modal */
.add-modal-dialog {
  background: var(--bg-sidebar);
  border: 1px solid var(--border-default);
  border-radius: 10px;
  width: 650px;
  max-width: 92vw;
  max-height: 85vh;
  display: flex;
  flex-direction: column;
  box-shadow: 0 20px 40px rgba(0, 0, 0, 0.4);
  overflow: hidden;
}

.add-modal-header {
  height: 52px;
  min-height: 52px;
  box-sizing: border-box;
  padding: 0 20px;
  border-bottom: 1px solid var(--border-default);
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.add-modal-body {
  flex: 1;
  overflow-y: auto;
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.add-modal-footer {
  height: 52px;
  min-height: 52px;
  box-sizing: border-box;
  padding: 0 20px;
  border-top: 1px solid var(--border-default);
  background: var(--bg-surface);
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 10px;
}

.field-group {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.field-label {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  font-weight: 700;
  color: var(--text-main);
}

.field-input {
  background: var(--bg-input);
  border: 1px solid var(--border-default);
  border-radius: 6px;
  color: var(--text-main);
  padding: 8px 12px;
  font-size: 13px;
  outline: none;
  font-family: inherit;
  width: 100%;
  box-sizing: border-box;
}

.field-input:focus {
  border-color: var(--border-focus);
}

.field-input:disabled {
  opacity: 0.5;
  cursor: not-allowed;
  background: var(--bg-surface);
}

.pager-btns {
  display: flex;
  gap: 6px;
  align-items: center;
}

/* SQL Console */
.sql-container {
  flex: 1;
  display: flex;
  flex-direction: column;
  padding: 20px;
  gap: 16px;
  overflow-y: auto;
}

.sql-box {
  background: var(--bg-sidebar);
  border: 1px solid var(--border-default);
  border-radius: 8px;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.sql-box-header {
  padding: 10px 14px;
  border-bottom: 1px solid var(--border-default);
  display: flex;
  align-items: center;
  font-size: 12px;
}

.sql-area {
  width: 100%;
  height: 120px;
  background: var(--bg-input);
  color: var(--text-main);
  border: none;
  padding: 12px 14px;
  font-family: var(--font-mono);
  font-size: 12.5px;
  line-height: 1.5;
  resize: vertical;
  outline: none;
}

.sql-box-footer {
  padding: 10px 14px;
  border-top: 1px solid var(--border-default);
  background: var(--bg-surface);
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.sql-results-card {
  flex: 1;
  min-height: 200px;
  background: var(--bg-sidebar);
  border: 1px solid var(--border-default);
  border-radius: 8px;
  overflow: auto;
}

/* Schema View */
.schema-panel {
  padding: 20px;
  overflow-y: auto;
}

.schema-table-card {
  background: var(--bg-sidebar);
  border: 1px solid var(--border-default);
  border-radius: 8px;
  overflow: hidden;
}

/* Toast */
.toast-bar {
  position: fixed;
  bottom: 20px;
  right: 20px;
  background: var(--text-main);
  border: 1px solid var(--text-main);
  color: var(--bg-app);
  padding: 10px 16px;
  border-radius: 6px;
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.3);
  font-size: 12px;
  font-weight: 700;
  z-index: 1000;
  cursor: pointer;
}
"#;

#[derive(Clone, Copy, PartialEq)]
enum AppView {
    Data,
    Schema,
    Sql,
}

use dioxus::desktop::tao::dpi::LogicalSize;

fn format_row_count(count: i64) -> String {
    let s = count.to_string();
    let is_neg = count < 0;
    let s = if is_neg { &s[1..] } else { &s };
    let mut result = String::new();
    let len = s.len();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    if is_neg {
        format!("-{result}")
    } else {
        result
    }
}

fn main() {
    let window = WindowBuilder::new()
        .with_title("LiteORM Studio (Desktop)")
        .with_inner_size(LogicalSize::new(1200.0, 800.0))
        .with_min_inner_size(LogicalSize::new(900.0, 600.0));

    let config = Config::new().with_window(window);

    dioxus::LaunchBuilder::desktop()
        .with_cfg(config)
        .launch(App);
}

#[component]
fn App() -> Element {
    let mut db = use_signal(|| None::<Arc<Database>>);
    let mut models = use_signal(Vec::<Model>::new);
    let table_counts = use_signal(std::collections::HashMap::<String, i64>::new);
    let mut selected_idx = use_signal(|| 0_usize);
    let mut current_view = use_signal(|| AppView::Data);

    let mut rows = use_signal(Vec::<serde_json::Map<String, Value>>::new);
    let mut total_records = use_signal(|| 0_i64);
    let mut page = use_signal(|| 1_usize);
    let page_size = use_signal(|| 50_usize);
    let mut search_query = use_signal(String::new);
    let mut sort_field = use_signal(|| None::<String>);
    let mut sort_dir = use_signal(|| "asc".to_string());
    let mut filters = use_signal(Vec::<db::FilterRule>::new);
    let mut show_filters = use_signal(|| false);

    let relation_counts = use_signal(std::collections::HashMap::<String, std::collections::HashMap<String, i64>>::new);
    let mut relation_modal = use_signal(|| None::<db::ResolvedRelation>);
    let relation_modal_val = use_signal(String::new);
    let relation_modal_rows = use_signal(Vec::<serde_json::Map<String, Value>>::new);
    let relation_modal_loading = use_signal(|| false);

    let mut sql_input = use_signal(|| "SELECT current_database(), now();".to_string());
    let mut sql_results = use_signal(|| None::<(Vec<String>, Vec<Vec<String>>, u64, u128)>);
    let mut toast_text = use_signal(|| None::<String>);
    let mut is_dark = use_signal(|| false);
    let is_loading = use_signal(|| true);
    let mut show_add_modal = use_signal(|| false);
    let mut add_inputs = use_signal(std::collections::HashMap::<String, String>::new);

    // Initial boot hook: connects to PostgreSQL and parses schema
    use_hook(move || {
        let mut is_loading = is_loading;
        let mut relation_counts = relation_counts;
        let mut table_counts = table_counts;
        is_loading.set(true);
        spawn(async move {
            // Read schema from demo or current dir
            let schema_paths = ["demo/liteorm.schema", "liteorm.schema"];
            let mut parsed_models = Vec::new();

            for path in schema_paths {
                if let Ok(content) = std::fs::read_to_string(path) {
                    if let Ok(schema_data) = schema::parse(&content) {
                        parsed_models = schema_data.models;
                        break;
                    }
                }
            }

            models.set(parsed_models.clone());

            match db::connect_db().await {
                Ok(conn) => {
                    db.set(Some(conn.clone()));

                    // Asynchronously fetch table counts for all models in fast batch
                    let count_conn = conn.clone();
                    let count_models = parsed_models.clone();
                    let mut t_counts = table_counts;
                    spawn(async move {
                        let counts = db::query_all_table_counts(&count_conn, &count_models).await;
                        t_counts.set(counts);
                    });

                    if !parsed_models.is_empty() {
                        // Load rows for first model
                        let m = &parsed_models[0];
                        if let Ok((loaded_rows, count)) = db::query_rows(&conn, m, 1, 50, None, None, None, &[]).await {
                            rows.set(loaded_rows.clone());
                            total_records.set(count);
                            table_counts.write().insert(m.name.clone(), count);

                            let relations = db::resolve_relations(m, &parsed_models);
                            let mut counts_map = std::collections::HashMap::new();
                            for rel in &relations {
                                if rel.many {
                                    let source_ids: Vec<String> = loaded_rows
                                        .iter()
                                        .filter_map(|r| r.get(&rel.source_field).and_then(|v| match v {
                                            Value::String(s) => Some(s.clone()),
                                            Value::Number(n) => Some(n.to_string()),
                                            _ => None,
                                        }))
                                        .collect();
                                    if let Ok(c_map) = db::query_relation_counts(&conn, rel, &source_ids).await {
                                        counts_map.insert(rel.field_name.clone(), c_map);
                                    }
                                }
                            }
                            relation_counts.set(counts_map);
                        }
                    }
                }
                Err(err) => {
                    toast_text.set(Some(format!("DB connection failed: {err}")));
                }
            }
            let mut is_loading = is_loading;
            is_loading.set(false);
        });
    });

    // Helper closure to reload rows
    let reload_rows = move || {
        let mut is_loading = is_loading;
        let mut relation_counts = relation_counts;
        let mut table_counts = table_counts;
        is_loading.set(true);
        spawn(async move {
            if let Some(conn) = db() {
                let m_list = models();
                if let Some(m) = m_list.get(selected_idx()) {
                    let sort = sort_field();
                    let s_ref = sort.as_deref();
                    let dir = sort_dir();
                    let search = search_query();
                    let search_opt = if search.trim().is_empty() { None } else { Some(search.as_str()) };
                    let current_filters = filters();

                    if let Ok((loaded, count)) = db::query_rows(&conn, m, page(), page_size(), s_ref, Some(&dir), search_opt, &current_filters).await {
                        rows.set(loaded.clone());
                        total_records.set(count);
                        if current_filters.is_empty() && search_opt.is_none() {
                            table_counts.write().insert(m.name.clone(), count);
                        }

                        let relations = db::resolve_relations(m, &m_list);
                        let mut counts_map = std::collections::HashMap::new();
                        for rel in &relations {
                            if rel.many {
                                let source_ids: Vec<String> = loaded
                                    .iter()
                                    .filter_map(|r| r.get(&rel.source_field).and_then(|v| match v {
                                        Value::String(s) => Some(s.clone()),
                                        Value::Number(n) => Some(n.to_string()),
                                        _ => None,
                                    }))
                                    .collect();
                                if let Ok(c_map) = db::query_relation_counts(&conn, rel, &source_ids).await {
                                    counts_map.insert(rel.field_name.clone(), c_map);
                                }
                            }
                        }
                        relation_counts.set(counts_map);
                    }
                }
            }
            let mut is_loading = is_loading;
            is_loading.set(false);
        });
    };

    let open_relation = move |rel: db::ResolvedRelation, source_val: String| {
        let mut relation_modal = relation_modal;
        let mut relation_modal_val = relation_modal_val;
        let mut relation_modal_rows = relation_modal_rows;
        let mut relation_modal_loading = relation_modal_loading;
        relation_modal.set(Some(rel.clone()));
        relation_modal_val.set(source_val.clone());
        relation_modal_rows.set(Vec::new());
        relation_modal_loading.set(true);
        spawn(async move {
            if let Some(conn) = db() {
                let m_list = models();
                if let Some(target) = m_list.iter().find(|m| m.name == rel.target_model) {
                    if let Ok(records) = db::query_connected_records(&conn, target, &rel.target_column, &source_val).await {
                        relation_modal_rows.set(records);
                    }
                }
            }
            relation_modal_loading.set(false);
        });
    };

    let active_model = models().get(selected_idx()).cloned();
    let active_relations = active_model.as_ref().map(|m| db::resolve_relations(m, &models())).unwrap_or_default();

    rsx! {
        style { {STYLE} }
        div { class: if is_dark() { "desktop-app" } else { "desktop-app theme-light" },
            // Sidebar
            aside { class: "sidebar",
                div { class: "brand",
                    div { class: "brand-icon",
                        img {
                            class: "brand-logo-img",
                            src: "data:image/png;base64,{LOGO_BASE64}",
                            alt: "LiteORM Logo",
                        }
                    }
                    div { class: "brand-text",
                        div { class: "brand-title", "LiteORM" }
                        div { class: "brand-badge", "DESKTOP STUDIO" }
                    }
                }

                div { class: "nav-section",
                    div { class: "nav-title", "MODELS & TABLES" }
                    if is_loading() && models().is_empty() {
                        for idx in 0..4 {
                            div { key: "side-skel-{idx}", style: "padding: 8px 10px; display: flex; gap: 8px; align-items: center;",
                                span { class: "skeleton-shimmer", style: "width: 30px; height: 16px; border-radius: 4px;" }
                                span { class: "skeleton-shimmer", style: "width: 90px; height: 14px;" }
                            }
                        }
                    } else {
                        for (idx, m) in models().iter().enumerate() {
                            button {
                                key: "{m.name}",
                                class: if selected_idx() == idx && current_view() != AppView::Sql { "nav-item active" } else { "nav-item" },
                                onclick: move |_| {
                                    selected_idx.set(idx);
                                    if current_view() == AppView::Sql {
                                        current_view.set(AppView::Data);
                                    }
                                    page.set(1);
                                    filters.set(Vec::new());
                                    reload_rows();
                                },
                                span { class: "badge-tag badge-tag-table", "TBL" }
                                span { class: "nav-label", title: "{m.name}", "{m.name}" }
                                span {
                                    class: "nav-tag",
                                    {
                                        match table_counts().get(&m.name).copied() {
                                            Some(1) => "1 row".to_string(),
                                            Some(c) => format!("{} rows", format_row_count(c)),
                                            None => "- rows".to_string(),
                                        }
                                    }
                                }
                            }
                        }
                    }
                    // Direct SQL console hidden for now
                }

                div { class: "sidebar-footer",
                    div { class: "status-dot" }
                    div { class: "status-text", "PostgreSQL Connected" }
                    button {
                        class: "theme-toggle-btn",
                        title: "Toggle Light/Dark Theme",
                        onclick: move |_| is_dark.set(!is_dark()),
                        if is_dark() {
                            span { class: "badge-tag badge-tag-light", "LIGHT" }
                        } else {
                            span { class: "badge-tag badge-tag-dark", "DARK" }
                        }
                    }
                }
            }

            // Main Viewport
            div { class: "main-viewport",
                // Top Application Bar
                header { class: "top-bar",
                    div { class: "top-left",
                        if let Some(ref m) = active_model {
                            div { class: "model-heading",
                                "{m.name}"
                                span { class: "table-tag", "@@map(\"{m.table}\")" }
                            }
                        } else {
                            div { class: "model-heading", "Studio" }
                        }

                        if current_view() != AppView::Sql {
                            div { class: "tab-group",
                                button {
                                    class: if current_view() == AppView::Data { "tab-btn active" } else { "tab-btn" },
                                    onclick: move |_| current_view.set(AppView::Data),
                                    "Table"
                                }
                                button {
                                    class: if current_view() == AppView::Schema { "tab-btn active" } else { "tab-btn" },
                                    onclick: move |_| current_view.set(AppView::Schema),
                                    "Schema"
                                }
                            }
                        }
                    }

                    div { class: "top-actions",
                        if current_view() == AppView::Data {
                            button {
                                class: "btn btn-secondary",
                                onclick: move |_| {
                                    reload_rows();
                                    if let Some(conn) = db() {
                                        let c_conn = conn.clone();
                                        let c_models = models();
                                        let mut t_counts = table_counts;
                                        spawn(async move {
                                            let counts = db::query_all_table_counts(&c_conn, &c_models).await;
                                            t_counts.set(counts);
                                        });
                                    }
                                },
                                "Refresh"
                            }
                        }
                    }
                }

                // Main Content View
                match current_view() {
                    AppView::Data => rsx! {
                        div { class: "toolbar",
                            div { style: "display: flex; align-items: center; gap: 10px;",
                                div { class: "search-wrap",
                                    input {
                                        class: "search-input",
                                        placeholder: "Search text fields...",
                                        value: "{search_query}",
                                        oninput: move |e| {
                                            search_query.set(e.value());
                                            page.set(1);
                                            reload_rows();
                                        }
                                    }
                                }
                                button {
                                    class: if show_filters() { "btn btn-secondary active" } else { "btn btn-secondary" },
                                    onclick: move |_| show_filters.set(!show_filters()),
                                    span { class: "badge-tag badge-tag-filter", "FILTER" }
                                    if !filters().is_empty() {
                                        span { class: "badge-tag badge-tag-table", "{filters().len()} active" }
                                    }
                                }
                                button {
                                    class: "btn btn-primary",
                                    onclick: {
                                        let m_opt = active_model.clone();
                                        move |_| {
                                            let mut map = std::collections::HashMap::new();
                                            if let Some(ref m) = m_opt {
                                                for f in m.fields.iter().filter(|f| db::scalar(f)) {
                                                    if f.kind == schema::FieldType::Boolean {
                                                        map.insert(f.name.clone(), "false".to_string());
                                                    }
                                                }
                                            }
                                            add_inputs.set(map);
                                            show_add_modal.set(true);
                                        }
                                    },
                                    "+ Add Record"
                                }
                            }
                            div { class: "record-stat",
                                "{format_row_count(total_records())} records"
                            }
                        }

                        if show_filters() {
                            div { class: "filter-bar",
                                div { class: "filter-bar-header",
                                    div { class: "filter-title",
                                        span { class: "badge-tag badge-tag-filter", "RULES" }
                                        span { style: "font-size: 11px; color: var(--text-muted);", "Filter records matching all rules:" }
                                    }
                                    div { style: "display: flex; gap: 8px;",
                                        button {
                                            class: "btn btn-secondary",
                                            style: "padding: 3px 8px; font-size: 11px;",
                                            onclick: {
                                                let m_opt = active_model.clone();
                                                move |_| {
                                                    if let Some(ref m) = m_opt {
                                                        if let Some(first_field) = m.fields.iter().find(|f| db::scalar(f)) {
                                                            let mut current = filters();
                                                            current.push(db::FilterRule {
                                                                field: first_field.name.clone(),
                                                                op: db::FilterOp::Contains,
                                                                value: String::new(),
                                                            });
                                                            filters.set(current);
                                                        }
                                                    }
                                                }
                                            },
                                            "+ Add Filter"
                                        }
                                        if !filters().is_empty() {
                                            button {
                                                class: "btn btn-secondary",
                                                style: "padding: 3px 8px; font-size: 11px;",
                                                onclick: move |_| {
                                                    filters.set(Vec::new());
                                                    page.set(1);
                                                    reload_rows();
                                                },
                                                "Clear All"
                                            }
                                        }
                                    }
                                }
                                if filters().is_empty() {
                                    div { class: "filter-empty-hint",
                                        "No active filter rules. Click \"+ Add Filter\" to create column-level rules."
                                    }
                                } else {
                                    div { class: "filter-list",
                                        for (rule_idx, rule) in filters().iter().enumerate() {
                                            div { key: "rule-{rule_idx}", class: "filter-row",
                                                select {
                                                    class: "filter-select",
                                                    value: "{rule.field}",
                                                    onchange: move |e| {
                                                        let mut list = filters();
                                                        if let Some(r) = list.get_mut(rule_idx) {
                                                            r.field = e.value();
                                                        }
                                                        filters.set(list);
                                                        page.set(1);
                                                        reload_rows();
                                                    },
                                                    if let Some(ref m) = active_model {
                                                        for f in m.fields.iter().filter(|f| db::scalar(f)) {
                                                            option { value: "{f.name}", selected: rule.field == f.name, "{f.name}" }
                                                        }
                                                    }
                                                }
                                                select {
                                                    class: "filter-select",
                                                    value: "{rule.op.as_str()}",
                                                    onchange: move |e| {
                                                        let mut list = filters();
                                                        if let Some(r) = list.get_mut(rule_idx) {
                                                            r.op = db::FilterOp::from_str(&e.value());
                                                        }
                                                        filters.set(list);
                                                        page.set(1);
                                                        reload_rows();
                                                    },
                                                    option { value: "contains", selected: rule.op == db::FilterOp::Contains, "contains" }
                                                    option { value: "equals", selected: rule.op == db::FilterOp::Equals, "equals (=)" }
                                                    option { value: "starts_with", selected: rule.op == db::FilterOp::StartsWith, "starts with" }
                                                    option { value: "ends_with", selected: rule.op == db::FilterOp::EndsWith, "ends with" }
                                                    option { value: "gt", selected: rule.op == db::FilterOp::GreaterThan, "greater than (>)" }
                                                    option { value: "lt", selected: rule.op == db::FilterOp::LessThan, "less than (<)" }
                                                    option { value: "gte", selected: rule.op == db::FilterOp::GreaterOrEqual, "greater or equal (>=)" }
                                                    option { value: "lte", selected: rule.op == db::FilterOp::LessOrEqual, "less or equal (<=)" }
                                                    option { value: "is_null", selected: rule.op == db::FilterOp::IsNull, "is null" }
                                                    option { value: "is_not_null", selected: rule.op == db::FilterOp::IsNotNull, "is not null" }
                                                    option { value: "is_true", selected: rule.op == db::FilterOp::IsTrue, "is true" }
                                                    option { value: "is_false", selected: rule.op == db::FilterOp::IsFalse, "is false" }
                                                }
                                                if rule.op.requires_value() {
                                                    input {
                                                        class: "filter-input",
                                                        placeholder: "Filter value...",
                                                        value: "{rule.value}",
                                                        oninput: move |e| {
                                                            let mut list = filters();
                                                            if let Some(r) = list.get_mut(rule_idx) {
                                                                r.value = e.value();
                                                            }
                                                            filters.set(list);
                                                            page.set(1);
                                                            reload_rows();
                                                        }
                                                    }
                                                }
                                                button {
                                                    class: "filter-remove-btn",
                                                    onclick: move |_| {
                                                        let mut list = filters();
                                                        if rule_idx < list.len() {
                                                            list.remove(rule_idx);
                                                        }
                                                        filters.set(list);
                                                        page.set(1);
                                                        reload_rows();
                                                    },
                                                    "Remove"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        div { class: "table-area",
                            if let Some(ref m) = active_model {
                                table { class: "data-grid",
                                    thead {
                                        tr {
                                            for f in m.fields.iter().filter(|f| db::scalar(f)) {
                                                th {
                                                    key: "{f.name}",
                                                    onclick: {
                                                        let f_name = f.name.clone();
                                                        move |_| {
                                                            if sort_field() == Some(f_name.clone()) {
                                                                if sort_dir() == "asc" {
                                                                    sort_dir.set("desc".to_string());
                                                                } else {
                                                                    sort_field.set(None);
                                                                    sort_dir.set("asc".to_string());
                                                                }
                                                            } else {
                                                                sort_field.set(Some(f_name.clone()));
                                                                sort_dir.set("asc".to_string());
                                                            }
                                                            reload_rows();
                                                        }
                                                    },
                                                    span {
                                                        class: match f.kind {
                                                            schema::FieldType::Int32 | schema::FieldType::Int64 | schema::FieldType::Float => "badge-type badge-int",
                                                            schema::FieldType::Boolean => "badge-type badge-bool",
                                                            schema::FieldType::String => "badge-type badge-str",
                                                            schema::FieldType::DateTime | schema::FieldType::Date | schema::FieldType::Time => "badge-type badge-date",
                                                            _ => "badge-type badge-other"
                                                        },
                                                        "{f.kind:?}"
                                                    }
                                                    "{f.name}"
                                                    if f.id {
                                                        span { class: "badge-pk", "PK" }
                                                    }
                                                }
                                            }
                                            for rel in &active_relations {
                                                th {
                                                    key: "{rel.field_name}",
                                                    span { class: "badge-tag badge-tag-relation", "REL" }
                                                    "{rel.field_name}"
                                                    span { style: "color: var(--text-muted); font-size: 11px; margin-left: 4px;", "[ ]" }
                                                }
                                            }
                                            th { style: "width: 70px;", "Actions" }
                                        }
                                    }
                                    tbody {
                                        if is_loading() {
                                            for row_idx in 0..7 {
                                                tr { key: "skeleton-{row_idx}", class: "skeleton-row",
                                                    for (col_idx, _f) in m.fields.iter().filter(|f| db::scalar(f)).enumerate() {
                                                        td {
                                                            span {
                                                                class: "skeleton-shimmer",
                                                                style: match (row_idx + col_idx) % 5 {
                                                                    0 => "width: 45%;",
                                                                    1 => "width: 75%;",
                                                                    2 => "width: 58%;",
                                                                    3 => "width: 82%;",
                                                                    _ => "width: 36%;",
                                                                }
                                                            }
                                                        }
                                                    }
                                                    for _rel in &active_relations {
                                                        td {
                                                            span { class: "skeleton-shimmer", style: "width: 65px;" }
                                                        }
                                                    }
                                                    td {
                                                        span { class: "skeleton-shimmer skeleton-btn" }
                                                    }
                                                }
                                            }
                                        } else if rows().is_empty() {
                                            tr {
                                                td {
                                                    colspan: "{m.fields.iter().filter(|f| db::scalar(f)).count() + active_relations.len() + 1}",
                                                    style: "text-align: center; padding: 48px 16px; color: var(--text-muted);",
                                                    div { style: "display: flex; flex-direction: column; align-items: center; gap: 8px;",
                                                        span { class: "badge-tag badge-tag-table", "EMPTY" }
                                                        span { "No records found" }
                                                    }
                                                }
                                            }
                                        } else {
                                            for (row_idx, row) in rows().iter().enumerate() {
                                                tr { key: "{row_idx}",
                                                    for f in m.fields.iter().filter(|f| db::scalar(f)) {
                                                        td {
                                                            match row.get(&f.name) {
                                                                Some(Value::Null) | None => rsx! { span { class: "val-null", "NULL" } },
                                                                Some(Value::Bool(b)) => if *b {
                                                                    rsx! { span { class: "val-true", "true" } }
                                                                } else {
                                                                    rsx! { span { class: "val-false", "false" } }
                                                                },
                                                                Some(Value::String(s)) => rsx! { "{s}" },
                                                                Some(val) => rsx! { "{val}" }
                                                            }
                                                        }
                                                    }
                                                    for rel in &active_relations {
                                                        {
                                                            let rel_clone = rel.clone();
                                                            let source_val_opt = row.get(&rel.source_field).and_then(|v| match v {
                                                                Value::String(s) => Some(s.clone()),
                                                                Value::Number(n) => Some(n.to_string()),
                                                                _ => None,
                                                            });
                                                            rsx! {
                                                                td {
                                                                    key: "{rel.field_name}",
                                                                    if let Some(source_val) = source_val_opt {
                                                                        if rel.many {
                                                                            {
                                                                                let count = relation_counts()
                                                                                    .get(&rel.field_name)
                                                                                    .and_then(|c_map| c_map.get(&source_val))
                                                                                    .copied()
                                                                                    .unwrap_or(0);
                                                                                let r = rel_clone.clone();
                                                                                let sv = source_val.clone();
                                                                                rsx! {
                                                                                    button {
                                                                                        class: "btn-relation-pill",
                                                                                        onclick: move |_| open_relation(r.clone(), sv.clone()),
                                                                                        span { class: "relation-count-badge", "{count}" }
                                                                                        span { class: "relation-target-name", "{rel.target_model}" }
                                                                                    }
                                                                                }
                                                                            }
                                                                        } else {
                                                                            {
                                                                                let r = rel_clone.clone();
                                                                                let sv = source_val.clone();
                                                                                rsx! {
                                                                                    button {
                                                                                        class: "btn-relation-pill",
                                                                                        onclick: move |_| open_relation(r.clone(), sv.clone()),
                                                                                        span { class: "relation-count-badge", "1" }
                                                                                        span { class: "relation-target-name", "{rel.target_model} #{sv}" }
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                    } else {
                                                                        span { class: "val-null", "none" }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                    td {
                                                        // Quick delete action
                                                        if let Some(id_field) = m.fields.iter().find(|f| f.id) {
                                                            if let Some(id_val) = row.get(&id_field.name) {
                                                                button {
                                                                    class: "btn btn-danger",
                                                                    onclick: {
                                                                        let id_field_name = id_field.name.clone();
                                                                        let id_val_clone = id_val.clone();
                                                                        let m_clone = m.clone();
                                                                        move |_| {
                                                                            let m_c = m_clone.clone();
                                                                            let id_f = id_field_name.clone();
                                                                            let id_v = id_val_clone.clone();
                                                                            spawn(async move {
                                                                                if let Some(conn) = db() {
                                                                                    if let Ok(_) = db::delete_row_by_id(&conn, &m_c, &id_f, &id_v).await {
                                                                                        toast_text.set(Some("Record deleted".into()));
                                                                                        reload_rows();
                                                                                    }
                                                                                }
                                                                            });
                                                                        }
                                                                    },
                                                                    "Delete"
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            } else {
                                div { style: "padding: 24px; display: flex; flex-direction: column; gap: 14px;",
                                    div { class: "skeleton-shimmer", style: "width: 180px; height: 22px;" }
                                    div { class: "skeleton-shimmer", style: "width: 100%; height: 36px;" }
                                    div { class: "skeleton-shimmer", style: "width: 100%; height: 36px;" }
                                    div { class: "skeleton-shimmer", style: "width: 100%; height: 36px;" }
                                    div { class: "skeleton-shimmer", style: "width: 100%; height: 36px;" }
                                }
                            }
                        }

                        footer { class: "table-footer",
                            span {
                                "{format_row_count(total_records())} "
                                {if total_records() == 1 { "row" } else { "rows" }}
                                " (Page {page()} of {((total_records() as usize).div_ceil(page_size())).max(1)})"
                            }
                            div { class: "pager-btns",
                                button {
                                    class: "btn btn-secondary",
                                    disabled: page() <= 1,
                                    onclick: move |_| {
                                        if page() > 1 {
                                            page.set(page() - 1);
                                            reload_rows();
                                        }
                                    },
                                    "← Prev"
                                }
                                button {
                                    class: "btn btn-secondary",
                                    disabled: page() * page_size() >= (total_records() as usize),
                                    onclick: move |_| {
                                        page.set(page() + 1);
                                        reload_rows();
                                    },
                                    "Next →"
                                }
                            }
                        }
                    },

                    AppView::Schema => rsx! {
                        div { class: "schema-panel",
                            if let Some(ref m) = active_model {
                                div { class: "schema-table-card",
                                    table { class: "data-grid",
                                        thead {
                                            tr {
                                                th { "Field Name" }
                                                th { "DB Column" }
                                                th { "Type" }
                                                th { "Attributes" }
                                                th { "Nullable" }
                                            }
                                        }
                                        tbody {
                                            for f in &m.fields {
                                                tr { key: "{f.name}",
                                                    td { strong { "{f.name}" } }
                                                    td { code { "{f.column}" } }
                                                    td { "{f.kind:?}" }
                                                    td {
                                                        if f.id { span { class: "badge-pk", "@id" } }
                                                        if f.auto_increment { span { class: "table-tag", "auto_increment" } }
                                                    }
                                                    td { if f.optional { "Yes" } else { "No" } }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },

                    AppView::Sql => rsx! {
                        div { class: "sql-container",
                            div { class: "sql-box",
                                div { class: "sql-box-header",
                                    span { class: "badge-tag badge-tag-sql", style: "margin-right: 8px;", "SQL" }
                                    strong { "Query Editor" }
                                }
                                textarea {
                                    class: "sql-area",
                                    value: "{sql_input}",
                                    oninput: move |e| sql_input.set(e.value())
                                }
                                div { class: "sql-box-footer",
                                    span { style: "color:var(--text-muted); font-size:11px;", "Execute SQL directly against PostgreSQL" }
                                    button {
                                        class: "btn btn-primary",
                                        onclick: move |_| {
                                            let sql = sql_input();
                                            spawn(async move {
                                                if let Some(conn) = db() {
                                                    let start = std::time::Instant::now();
                                                    match db::execute_sql(&conn, &sql).await {
                                                        Ok((cols, r_data, aff)) => {
                                                            let dur = start.elapsed().as_millis();
                                                            sql_results.set(Some((cols, r_data, aff, dur)));
                                                        }
                                                        Err(e) => {
                                                            toast_text.set(Some(format!("SQL Error: {e}")));
                                                        }
                                                    }
                                                }
                                            });
                                        },
                                        "Run Query"
                                    }
                                }
                            }

                            if let Some((cols, r_data, aff, dur)) = sql_results() {
                                div { class: "sql-results-card",
                                    div { style: "padding: 10px 14px; border-bottom: 1px solid var(--border-default); display:flex; justify-content:space-between;",
                                        strong { "Results: {r_data.len()} rows, {aff} affected" }
                                        span { style: "color:var(--accent);", "{dur}ms" }
                                    }
                                    table { class: "data-grid",
                                        thead {
                                            tr {
                                                for col in cols {
                                                    th { key: "{col}", "{col}" }
                                                }
                                            }
                                        }
                                        tbody {
                                            for (idx, r) in r_data.iter().enumerate() {
                                                tr { key: "{idx}",
                                                    for cell in r {
                                                        td { "{cell}" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Relation Records Modal
            if let Some(rel) = relation_modal() {
                div {
                    class: "relation-modal-backdrop",
                    onclick: move |_| relation_modal.set(None),
                    div {
                        class: "relation-modal-dialog",
                        onclick: move |e| e.stop_propagation(),
                        div { class: "relation-modal-header",
                            div { style: "display: flex; align-items: center; gap: 8px;",
                                span { class: "badge-tag badge-tag-relation", "RELATION" }
                                strong { "Connected {rel.target_model} Records" }
                                span { style: "color: var(--text-muted); font-size: 12px;", "({rel.target_column} = {relation_modal_val()})" }
                            }
                            div { style: "display: flex; gap: 8px; align-items: center;",
                                button {
                                    class: "btn btn-primary",
                                    style: "font-size: 11px; padding: 4px 10px;",
                                    onclick: {
                                        let t_name = rel.target_model.clone();
                                        let t_col = rel.target_column.clone();
                                        let s_val = relation_modal_val();
                                        move |_| {
                                            if let Some(pos) = models().iter().position(|m| m.name == t_name) {
                                                selected_idx.set(pos);
                                                filters.set(vec![db::FilterRule {
                                                    field: t_col.clone(),
                                                    op: db::FilterOp::Equals,
                                                    value: s_val.clone(),
                                                }]);
                                                show_filters.set(true);
                                                relation_modal.set(None);
                                                page.set(1);
                                                reload_rows();
                                            }
                                        }
                                    },
                                    "Open in Table →"
                                }
                                button {
                                    class: "btn btn-secondary",
                                    style: "font-size: 11px; padding: 4px 10px;",
                                    onclick: move |_| relation_modal.set(None),
                                    "Close"
                                }
                            }
                        }
                        div { class: "relation-modal-body",
                            if relation_modal_loading() {
                                div { style: "padding: 24px; display: flex; flex-direction: column; gap: 12px;",
                                    div { class: "skeleton-shimmer", style: "width: 100%; height: 28px;" }
                                    div { class: "skeleton-shimmer", style: "width: 100%; height: 28px;" }
                                    div { class: "skeleton-shimmer", style: "width: 100%; height: 28px;" }
                                }
                            } else if relation_modal_rows().is_empty() {
                                div { style: "padding: 48px; text-align: center; color: var(--text-muted);",
                                    div { style: "display: flex; flex-direction: column; align-items: center; gap: 8px;",
                                        span { class: "badge-tag badge-tag-relation", "0 RECORDS" }
                                        span { "No connected {rel.target_model} records found matching {rel.target_column} = {relation_modal_val()}" }
                                    }
                                }
                            } else if let Some(tm) = models().iter().find(|m| m.name == rel.target_model) {
                                table { class: "data-grid",
                                    thead {
                                        tr {
                                            for f in tm.fields.iter().filter(|f| db::scalar(f)) {
                                                th {
                                                    key: "{f.name}",
                                                    span {
                                                        class: match f.kind {
                                                            schema::FieldType::Int32 | schema::FieldType::Int64 | schema::FieldType::Float => "badge-type badge-int",
                                                            schema::FieldType::Boolean => "badge-type badge-bool",
                                                            schema::FieldType::String => "badge-type badge-str",
                                                            schema::FieldType::DateTime | schema::FieldType::Date | schema::FieldType::Time => "badge-type badge-date",
                                                            _ => "badge-type badge-other"
                                                        },
                                                        "{f.kind:?}"
                                                    }
                                                    "{f.name}"
                                                    if f.id {
                                                        span { class: "badge-pk", "PK" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    tbody {
                                        for (r_idx, row) in relation_modal_rows().iter().enumerate() {
                                            tr { key: "{r_idx}",
                                                for f in tm.fields.iter().filter(|f| db::scalar(f)) {
                                                    td {
                                                        match row.get(&f.name) {
                                                            Some(Value::Null) | None => rsx! { span { class: "val-null", "NULL" } },
                                                            Some(Value::Bool(b)) => if *b {
                                                                rsx! { span { class: "val-true", "true" } }
                                                            } else {
                                                                rsx! { span { class: "val-false", "false" } }
                                                            },
                                                            Some(Value::String(s)) => rsx! { "{s}" },
                                                            Some(val) => rsx! { "{val}" }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        div { class: "relation-modal-footer",
                            span { style: "font-size: 11px; color: var(--text-muted);",
                                "{relation_modal_rows().len()} connected record(s) loaded"
                            }
                            button {
                                class: "btn btn-secondary",
                                onclick: move |_| relation_modal.set(None),
                                "Dismiss"
                            }
                        }
                    }
                }
            }

            // Add Record Modal
            if show_add_modal() {
                if let Some(m) = active_model.clone() {
                    div {
                        class: "relation-modal-backdrop",
                            onclick: move |_| show_add_modal.set(false),
                            div {
                                class: "add-modal-dialog",
                                onclick: move |e| e.stop_propagation(),
                                div { class: "add-modal-header",
                                    div { style: "display: flex; align-items: center; gap: 10px;",
                                        span { class: "badge-tag badge-tag-table", "NEW" }
                                        span { style: "font-weight: 700; font-size: 14px;", "Add Record to {m.name}" }
                                    }
                                    button {
                                        class: "btn btn-secondary",
                                        style: "font-size: 11px; padding: 4px 10px;",
                                        onclick: move |_| show_add_modal.set(false),
                                        "Cancel"
                                    }
                                }
                                div { class: "add-modal-body",
                                    for f in m.fields.iter().filter(|f| db::scalar(f)) {
                                        {
                                            let fname = f.name.clone();
                                            let fname_sel = fname.clone();
                                            let fname_inp = fname.clone();
                                            let fkind = f.kind.clone();
                                            let is_id = f.id;
                                            let is_auto = f.auto_increment;
                                            let is_opt = f.optional;
                                            let has_def = f.default.is_some();
                                            let cur_val = add_inputs().get(&fname).cloned().unwrap_or_default();
                                            
                                            rsx! {
                                                div { class: "field-group", key: "{fname}",
                                                    div { class: "field-label",
                                                        span { "{fname}" }
                                                        span {
                                                            class: match fkind {
                                                                schema::FieldType::Int32 | schema::FieldType::Int64 | schema::FieldType::Float => "badge-type badge-int",
                                                                schema::FieldType::Boolean => "badge-type badge-bool",
                                                                schema::FieldType::String => "badge-type badge-str",
                                                                schema::FieldType::DateTime | schema::FieldType::Date | schema::FieldType::Time => "badge-type badge-date",
                                                                _ => "badge-type badge-other"
                                                            },
                                                            "{fkind:?}"
                                                        }
                                                        if is_id {
                                                            span { class: "badge-pk", "PK" }
                                                        }
                                                        if is_auto {
                                                            span { class: "badge-tag badge-tag-filter", "AUTO-GENERATED" }
                                                        } else if is_opt {
                                                            span { class: "badge-tag badge-tag-filter", "OPTIONAL" }
                                                        } else if has_def {
                                                            span { class: "badge-tag badge-tag-filter", "DEFAULT" }
                                                        } else {
                                                            span { class: "badge-tag badge-tag-table", "REQUIRED" }
                                                        }
                                                    }
                                                    if is_auto {
                                                        input {
                                                            class: "field-input",
                                                            disabled: true,
                                                            placeholder: "Database generates this value automatically"
                                                        }
                                                    } else if fkind == schema::FieldType::Boolean {
                                                        select {
                                                            class: "field-input",
                                                            value: "{cur_val}",
                                                            onchange: move |e| {
                                                                let mut map = add_inputs();
                                                                map.insert(fname_sel.clone(), e.value());
                                                                add_inputs.set(map);
                                                            },
                                                            option { value: "false", "false" }
                                                            option { value: "true", "true" }
                                                        }
                                                    } else {
                                                        input {
                                                            class: "field-input",
                                                            placeholder: if is_opt { "Optional" } else { "Enter value..." },
                                                            value: "{cur_val}",
                                                            oninput: move |e| {
                                                                let mut map = add_inputs();
                                                                map.insert(fname_inp.clone(), e.value());
                                                                add_inputs.set(map);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                div { class: "add-modal-footer",
                                    button {
                                        class: "btn btn-secondary",
                                        onclick: move |_| show_add_modal.set(false),
                                        "Cancel"
                                    }
                                    button {
                                        class: "btn btn-primary",
                                        onclick: {
                                            let m_clone = m.clone();
                                            let toast_text = toast_text;
                                            let show_add_modal = show_add_modal;
                                            let reload_rows_clone = reload_rows;
                                            move |_| {
                                                if let Some(ref db_conn) = db() {
                                                    let db_c = db_conn.clone();
                                                    let m_inner = m_clone.clone();
                                                    let inputs = add_inputs();
                                                    let mut toast_text = toast_text;
                                                    let mut show_add_modal = show_add_modal;
                                                    let reload = reload_rows_clone;
                                                    spawn(async move {
                                                        match db::insert_row(&db_c, &m_inner, &inputs).await {
                                                            Ok(()) => {
                                                                toast_text.set(Some(format!("Successfully created new record in {}", m_inner.name)));
                                                                show_add_modal.set(false);
                                                                reload();
                                                            }
                                                            Err(err) => {
                                                                toast_text.set(Some(format!("Insert failed: {err}")));
                                                            }
                                                        }
                                                    });
                                                }
                                            }
                                        },
                                        "Save Record"
                                    }
                                }
                            }
                        }
                    }
                }

            // Notification Toast
            if let Some(msg) = toast_text() {
                div {
                    class: "toast-bar",
                    onclick: move |_| toast_text.set(None),
                    "{msg}"
                }
            }
        }
    }
}
