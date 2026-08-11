//! JSON response rendered with gpui-component's `DataTable`.
//!
//! `DataTable` gives us resizable / movable / sortable columns and virtual
//! scrolling for free. The trade-off is a uniform row height, so cells render a
//! single truncated line and the full value is shown in the detail panel below
//! the table when a cell is selected (see `render_json_table_detail`).

use std::collections::HashSet;

use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable as _,
    button::{Button, ButtonVariants},
    table::{Column, ColumnSort, TableDelegate, TableState},
};

/// Column cap for array-of-objects responses. Wide records would otherwise
/// build thousands of columns from a single ragged payload.
const MAX_COLUMNS: usize = 40;
/// Row cap for array responses. Virtual scrolling handles the rendering, but
/// flattening every row of a megabyte-scale array is not worth the latency.
const MAX_ROWS: usize = 5_000;

const KEY_COLUMN_WIDTH: Pixels = px(180.);
/// Kept narrow enough to fit the panel without horizontal scrolling on open;
/// long values live in the detail panel, and the column is resizable anyway.
const VALUE_COLUMN_WIDTH: Pixels = px(380.);
const RECORD_COLUMN_WIDTH: Pixels = px(160.);

/// A JSON value flattened into a grid of string cells.
pub struct JsonTableDelegate {
    columns: Vec<Column>,
    /// Every row of the payload; cells are positionally aligned with `columns`.
    rows: Vec<Vec<String>>,
    /// Indices into `rows` that pass the current search query, in display order.
    visible: Vec<usize>,
    query: String,
    collapsed: HashSet<String>,
    /// True when column 0 is a field-name/index column rather than data.
    key_column: bool,
    /// Rows dropped by `MAX_ROWS`, reported in the toolbar so the cap is visible.
    truncated_rows: usize,
}

impl JsonTableDelegate {
    /// Build a delegate for `value`, or `None` if it holds nothing tabular.
    pub fn from_json(value: &serde_json::Value) -> Option<Self> {
        let (columns, rows, key_column, truncated_rows) = match value {
            serde_json::Value::Object(object) => {
                let rows: Vec<Vec<String>> = flatten_json_object(object)
                    .into_iter()
                    .map(|(key, value)| vec![key, value])
                    .collect();
                (
                    vec![
                        Column::new("field", "Field")
                            .width(KEY_COLUMN_WIDTH)
                            .min_width(px(80.))
                            .fixed_left()
                            .sortable(),
                        Column::new("value", "Value")
                            .width(VALUE_COLUMN_WIDTH)
                            .min_width(px(120.))
                            .sortable(),
                    ],
                    rows,
                    true,
                    0,
                )
            }
            serde_json::Value::Array(values) if values.is_empty() => return None,
            serde_json::Value::Array(values) => {
                let truncated_rows = values.len().saturating_sub(MAX_ROWS);
                let values = &values[..values.len().min(MAX_ROWS)];

                if values.iter().any(|value| value.is_object()) {
                    let mut keys: Vec<String> = Vec::new();
                    for object in values.iter().filter_map(|value| value.as_object()) {
                        for (key, _) in flatten_json_object(object) {
                            if !keys.contains(&key) {
                                keys.push(key);
                            }
                            if keys.len() >= MAX_COLUMNS {
                                break;
                            }
                        }
                        if keys.len() >= MAX_COLUMNS {
                            break;
                        }
                    }

                    let mut columns = vec![
                        Column::new("#", "#")
                            .width(px(64.))
                            .min_width(px(40.))
                            .fixed_left()
                            .movable(false)
                            .sortable(),
                    ];
                    columns.extend(keys.iter().map(|key| {
                        Column::new(key.clone(), key.clone())
                            .width(RECORD_COLUMN_WIDTH)
                            .min_width(px(60.))
                            .sortable()
                    }));

                    let rows = values
                        .iter()
                        .enumerate()
                        .map(|(idx, value)| {
                            let flat = value
                                .as_object()
                                .map(flatten_json_object)
                                .unwrap_or_default();
                            let mut row = Vec::with_capacity(keys.len() + 1);
                            row.push(idx.to_string());
                            row.extend(keys.iter().map(|key| {
                                flat.iter()
                                    .find(|(flat_key, _)| flat_key == key)
                                    .map(|(_, value)| value.clone())
                                    .unwrap_or_default()
                            }));
                            row
                        })
                        .collect();

                    (columns, rows, true, truncated_rows)
                } else {
                    let rows = values
                        .iter()
                        .enumerate()
                        .map(|(idx, value)| vec![idx.to_string(), json_cell_text(value)])
                        .collect();
                    (
                        vec![
                            Column::new("#", "#")
                                .width(px(64.))
                                .min_width(px(40.))
                                .fixed_left()
                                .movable(false)
                                .sortable(),
                            Column::new("value", "Value")
                                .width(VALUE_COLUMN_WIDTH)
                                .min_width(px(120.))
                                .sortable(),
                        ],
                        rows,
                        true,
                        truncated_rows,
                    )
                }
            }
            _ => (
                vec![
                    Column::new("value", "Value")
                        .width(VALUE_COLUMN_WIDTH)
                        .min_width(px(120.)),
                ],
                vec![vec![json_cell_text(value)]],
                false,
                0,
            ),
        };

        let mut this = Self {
            columns,
            rows,
            visible: Vec::new(),
            query: String::new(),
            collapsed: HashSet::new(),
            key_column,
            truncated_rows,
        };
        this.reindex();
        Some(this)
    }

    /// Replace the search query. Returns true when the visible set changed.
    pub fn set_query(&mut self, query: &str) -> bool {
        let query = query.trim().to_lowercase();
        if query == self.query {
            return false;
        }
        self.query = query;
        self.reindex();
        true
    }

    /// Number of rows matching the current query.
    pub fn visible_count(&self) -> usize {
        self.visible.len()
    }

    /// Total number of rows held, ignoring the query.
    pub fn total_count(&self) -> usize {
        self.rows.len()
    }

    /// Rows dropped because the payload exceeded `MAX_ROWS`.
    pub fn truncated_rows(&self) -> usize {
        self.truncated_rows
    }

    /// The display name of a column, for the detail panel heading.
    pub fn column_name(&self, col_ix: usize) -> Option<SharedString> {
        self.columns.get(col_ix).map(|column| column.name.clone())
    }

    /// The row's key column value (field name or index), if the table has one.
    pub fn row_label(&self, row_ix: usize) -> Option<&str> {
        if !self.key_column {
            return None;
        }
        self.cell(row_ix, 0).filter(|value| !value.is_empty())
    }

    /// The untruncated value of a visible cell.
    pub fn cell(&self, row_ix: usize, col_ix: usize) -> Option<&str> {
        let row = self.rows.get(*self.visible.get(row_ix)?)?;
        row.get(col_ix).map(String::as_str)
    }

    /// The whole row re-serialised as a JSON object, for "copy row".
    pub fn row_json(&self, row_ix: usize) -> Option<String> {
        let row = self.rows.get(*self.visible.get(row_ix)?)?;
        let object: serde_json::Map<String, serde_json::Value> = self
            .columns
            .iter()
            .zip(row.iter())
            .map(|(column, value)| {
                (
                    column.key.to_string(),
                    serde_json::Value::String(value.clone()),
                )
            })
            .collect();
        serde_json::to_string_pretty(&object).ok()
    }

    fn has_children(&self, path: &str) -> bool {
        self.rows.iter().any(|row| {
            row.first()
                .is_some_and(|candidate| is_descendant_path(candidate, path))
        })
    }

    fn is_collapsed(&self, path: &str) -> bool {
        self.collapsed.contains(path)
    }

    fn toggle_collapsed(&mut self, path: &str) {
        if !self.has_children(path) {
            return;
        }
        if !self.collapsed.remove(path) {
            self.collapsed.insert(path.to_string());
        }
        self.reindex();
    }

    pub fn visible_csv(&self) -> String {
        let mut out = String::new();
        out.push_str(
            &self
                .columns
                .iter()
                .map(|column| csv_cell(column.name.as_ref()))
                .collect::<Vec<_>>()
                .join(","),
        );
        for row_ix in &self.visible {
            if let Some(row) = self.rows.get(*row_ix) {
                out.push('\n');
                out.push_str(
                    &row.iter()
                        .map(|cell| csv_cell(cell))
                        .collect::<Vec<_>>()
                        .join(","),
                );
            }
        }
        out
    }

    /// Recompute `visible` from the query, then apply the active column sort.
    fn reindex(&mut self) {
        self.visible = (0..self.rows.len())
            .filter(|&ix| {
                if self.key_column {
                    if let Some(path) = self.rows[ix].first() {
                        if self
                            .collapsed
                            .iter()
                            .any(|collapsed| is_descendant_path(path, collapsed))
                        {
                            return false;
                        }
                    }
                }
                self.query.is_empty()
                    || self.rows[ix]
                        .iter()
                        .any(|cell| cell.to_lowercase().contains(&self.query))
            })
            .collect();

        let Some((col_ix, sort)) =
            self.columns
                .iter()
                .enumerate()
                .find_map(|(ix, column)| match column.sort {
                    Some(sort @ (ColumnSort::Ascending | ColumnSort::Descending)) => {
                        Some((ix, sort))
                    }
                    _ => None,
                })
        else {
            return;
        };

        let rows = &self.rows;
        self.visible.sort_by(|&a, &b| {
            let left = rows[a].get(col_ix).map(String::as_str).unwrap_or("");
            let right = rows[b].get(col_ix).map(String::as_str).unwrap_or("");
            let ordering = match (left.parse::<f64>(), right.parse::<f64>()) {
                (Ok(left), Ok(right)) => left
                    .partial_cmp(&right)
                    .unwrap_or(std::cmp::Ordering::Equal),
                _ => left.to_lowercase().cmp(&right.to_lowercase()),
            };
            if sort == ColumnSort::Descending {
                ordering.reverse()
            } else {
                ordering
            }
        });
    }
}

impl TableDelegate for JsonTableDelegate {
    fn columns_count(&self, _: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _: &App) -> usize {
        self.visible.len()
    }

    fn column(&self, col_ix: usize, _: &App) -> Column {
        self.columns
            .get(col_ix)
            .cloned()
            .unwrap_or_else(|| Column::new("", ""))
    }

    fn perform_sort(
        &mut self,
        col_ix: usize,
        sort: ColumnSort,
        _window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) {
        for (ix, column) in self.columns.iter_mut().enumerate() {
            if column.sort.is_none() {
                continue;
            }
            column.sort = Some(if ix == col_ix {
                sort
            } else {
                ColumnSort::Default
            });
        }
        self.reindex();
        cx.notify();
    }

    fn move_column(
        &mut self,
        col_ix: usize,
        to_ix: usize,
        _window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) {
        if col_ix >= self.columns.len() || to_ix >= self.columns.len() {
            return;
        }
        let column = self.columns.remove(col_ix);
        self.columns.insert(to_ix, column);
        for row in &mut self.rows {
            if col_ix < row.len() && to_ix < row.len() {
                let cell = row.remove(col_ix);
                row.insert(to_ix, cell);
            }
        }
        cx.notify();
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let value = self.cell(row_ix, col_ix).unwrap_or_default();
        let strong = self.key_column && col_ix == 0;
        let cell = div()
            .w_full()
            // min_w(0) is what actually lets the cell shrink below its text's
            // intrinsic width; without it the text sets the floor and spills
            // past the column edge instead of ellipsing.
            .min_w(px(0.))
            .truncate()
            .text_sm()
            .when(strong, |el| el.font_weight(FontWeight::MEDIUM));

        if strong {
            let (depth, leaf, nested) = field_path_view(value);
            let has_children = self.has_children(value);
            let collapsed = self.is_collapsed(value);
            let path = value.to_string();
            cell.child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .min_w(px(0.))
                    .pl(px((depth.min(6) * 14) as f32))
                    .child(if has_children {
                        Button::new(("json-field-toggle", row_ix))
                            .icon(if collapsed {
                                IconName::ChevronRight
                            } else {
                                IconName::ChevronDown
                            })
                            .tooltip(if collapsed {
                                "Expand field"
                            } else {
                                "Collapse field"
                            })
                            .ghost()
                            .xsmall()
                            .on_click(cx.listener(move |state, _, _, cx| {
                                state.delegate_mut().toggle_collapsed(&path);
                                state.refresh(cx);
                                cx.notify();
                            }))
                            .into_any_element()
                    } else if nested {
                        div()
                            .w(px(24.))
                            .child(
                                Icon::new(IconName::ChevronRight)
                                    .size(px(12.))
                                    .text_color(cx.theme().muted_foreground.opacity(0.45)),
                            )
                            .into_any_element()
                    } else {
                        div().w(px(24.)).into_any_element()
                    })
                    .child(div().min_w(px(0.)).truncate().child(single_line(leaf))),
            )
        } else {
            cell.child(
                div()
                    .pl_2()
                    .min_w(px(0.))
                    .truncate()
                    .child(single_line(value)),
            )
        }
    }

    fn cell_text(&self, row_ix: usize, col_ix: usize, _: &App) -> String {
        self.cell(row_ix, col_ix).unwrap_or_default().to_string()
    }
}

/// Collapse a value to one line so it fits a uniform-height table cell.
fn single_line(value: &str) -> String {
    let collapsed = value.replace(['\r', '\n', '\t'], " ");
    if collapsed.len() <= 400 {
        return collapsed;
    }
    let end = collapsed
        .char_indices()
        .map(|(ix, _)| ix)
        .take_while(|&ix| ix <= 400)
        .last()
        .unwrap_or(0);
    format!("{}…", &collapsed[..end])
}

fn field_path_view(path: &str) -> (usize, &str, bool) {
    let depth = path.matches('.').count() + path.matches('[').count();
    let leaf_start = path
        .rfind('.')
        .map(|ix| ix + 1)
        .or_else(|| path.rfind('['))
        .unwrap_or(0);
    (depth, &path[leaf_start..], depth > 0)
}

fn csv_cell(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

/// Flatten nested objects into `parent.child` rows, and arrays into
/// `parent[0]` rows when they contain objects.
pub fn flatten_json_object(
    object: &serde_json::Map<String, serde_json::Value>,
) -> Vec<(String, String)> {
    let mut rows = Vec::new();
    for (key, value) in object {
        flatten_json_value(key, value, &mut rows);
    }
    rows
}

fn flatten_json_value(path: &str, value: &serde_json::Value, rows: &mut Vec<(String, String)>) {
    match value {
        serde_json::Value::Object(object) => {
            if object.is_empty() {
                rows.push((path.to_string(), "{}".to_string()));
                return;
            }
            rows.push((path.to_string(), format!("{{{} fields}}", object.len())));
            for (key, value) in object {
                flatten_json_value(&format!("{path}.{key}"), value, rows);
            }
        }
        serde_json::Value::Array(values) => {
            if values.is_empty() {
                rows.push((path.to_string(), "[]".to_string()));
                return;
            }
            if values.iter().all(|value| !value.is_object()) {
                rows.push((path.to_string(), json_cell_text(value)));
                return;
            }
            rows.push((path.to_string(), format!("[{} items]", values.len())));
            for (idx, value) in values.iter().enumerate() {
                flatten_json_value(&format!("{path}[{idx}]"), value, rows);
            }
        }
        _ => rows.push((path.to_string(), json_cell_text(value))),
    }
}

fn is_descendant_path(candidate: &str, parent: &str) -> bool {
    candidate.len() > parent.len()
        && candidate.starts_with(parent)
        && matches!(candidate.as_bytes().get(parent.len()), Some(b'.' | b'['))
}

pub fn json_cell_text(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(value) => value.to_string(),
        serde_json::Value::Number(value) => value.to_string(),
        serde_json::Value::String(value) => value.clone(),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
        }
    }
}
