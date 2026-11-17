//! A table of read-only records: the base table's rows and cells, drawn with the app's type.

use nsh_base::{table as base_table, table_body, table_cell, table_head, table_header, table_row};
use nsh_core::element::Prop;
use nsh_core::prelude::*;

use super::tokens;

/// A table's props: its columns, its rows and what a screen reader calls it.
#[derive(Clone, PartialEq, Props)]
pub struct TableProps {
    /// One heading per column.
    pub headers: Prop<Vec<SharedString>>,
    /// The rows, already ordered; one cell per column.
    pub rows: Prop<Vec<Vec<SharedString>>>,
    /// What a screen reader announces the table as.
    #[prop(default)]
    pub accessibility_label: Prop<SharedString>,
}

/// A table of its rows under its headers, one cell per column.
///
/// The rows arrive already ordered; the screen decides what a column means, and this decides only
/// how the base table's rows read: a muted header, then one line per record.
#[component]
pub fn Table(props: TableProps) -> impl IntoElement {
    let mut head = table_row();
    for heading in props.headers.get_untracked() {
        head = head.child(
            table_head().child(text(heading).text_xs().text_color(tokens::muted_foreground)),
        );
    }
    let mut body = table_body();
    for row in props.rows.get_untracked() {
        let mut line = table_row().gap_2();
        for cell in row {
            line = line.child(
                table_cell()
                    .flex_1()
                    .child(text(cell).text_sm().text_color(tokens::foreground)),
            );
        }
        body = body.child(line);
    }
    base_table()
        .accessibility_label(move || Some(props.accessibility_label.get()))
        .gap_1()
        .child(table_header().child(head))
        .child(body)
}
