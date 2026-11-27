//! CSV, the format another tool can edit in a spreadsheet.
//!
//! The header is fixed and named: `deck, sides, tags, fields, markdown`. The `deck` cell records
//! where a card came from and is read for information only; an import lands in the deck its caller
//! chose. `tags` is a JSON array of strings and `fields` a JSON object, so a tag or field value
//! holding a comma, a quote or a newline survives the round trip. `markdown` is quoted by the
//! ordinary CSV rules, which is what keeps its newlines inside one cell.

use std::collections::BTreeMap;

use crate::command::{Command, Outcome};
use crate::error::StoreError;
use crate::hidden;
use crate::model::{CardKind, Id, ImportReport};

use super::target::TransferTarget;

/// The header every CSV must name. Order is free; a missing column is refused by name.
pub const HEADER: [&str; 5] = ["deck", "sides", "tags", "fields", "markdown"];

/// Write the text cards of `deck` and its descendants to `target`.
///
/// A diagram has no text to carry and no column to carry its picture, so it is left out rather
/// than written as an empty row. An existing target is refused: a failed export must not have
/// overwritten the file the user meant to keep.
pub fn export(
    db: &impl TransferTarget,
    deck: Id,
    target: &std::path::Path,
) -> Result<usize, StoreError> {
    if target.exists() {
        return Err(StoreError::Exists {
            path: target.to_path_buf(),
        });
    }
    let snapshot = db.snapshot()?;
    if !snapshot.decks.iter().any(|row| row.id == deck) {
        return Err(StoreError::Missing {
            what: "deck",
            id: deck,
        });
    }
    let under = snapshot.subtree(Some(deck));
    let names: BTreeMap<Id, &str> = snapshot
        .decks
        .iter()
        .map(|row| (row.id, row.name.as_str()))
        .collect();

    let mut out = String::from("deck, sides, tags, fields, markdown\n");
    let mut written = 0;
    for card in &snapshot.cards {
        if !under.contains(&card.deck)
            || card.trashed_at.is_some()
            || card.kind == CardKind::Diagram
        {
            continue;
        }
        let fields: BTreeMap<&str, &str> = card
            .fields
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
            .collect();
        let tags = serde_json::to_string(&card.tags).map_err(|error| StoreError::Malformed {
            path: target.to_path_buf(),
            reason: format!("the tags of card {} could not be written: {error}", card.id),
        })?;
        let fields = serde_json::to_string(&fields).map_err(|error| StoreError::Malformed {
            path: target.to_path_buf(),
            reason: format!(
                "the fields of card {} could not be written: {error}",
                card.id
            ),
        })?;
        let row = [
            names
                .get(&card.deck)
                .copied()
                .unwrap_or_default()
                .to_string(),
            card.sides.to_string(),
            tags,
            fields,
            card.markdown.clone(),
        ];
        for (column, cell) in row.iter().enumerate() {
            if column > 0 {
                out.push(',');
            }
            out.push_str(&quoted(cell));
        }
        out.push('\n');
        written += 1;
    }
    std::fs::write(target, out).map_err(|source| StoreError::Write {
        path: target.to_path_buf(),
        source,
    })?;
    Ok(written)
}

/// Read `source` into `deck`, mapping its sides, tags and fields.
///
/// Every row is parsed and checked before the first card is written, so a bad row near the end
/// cannot leave half the file imported even though the transaction would roll it back anyway.
pub fn import(
    db: &impl TransferTarget,
    deck: Id,
    source: &std::path::Path,
    now_ms: i64,
) -> Result<ImportReport, StoreError> {
    let text = std::fs::read_to_string(source).map_err(|error| StoreError::Read {
        path: source.to_path_buf(),
        source: error,
    })?;
    let rows = parse(text.strip_prefix('\u{feff}').unwrap_or(&text)).map_err(|reason| {
        StoreError::Malformed {
            path: source.to_path_buf(),
            reason,
        }
    })?;
    let Some(header) = rows.first() else {
        return Err(StoreError::Malformed {
            path: source.to_path_buf(),
            reason: "the file is empty".to_string(),
        });
    };
    let columns = Columns::new(header).map_err(|reason| StoreError::Malformed {
        path: source.to_path_buf(),
        reason,
    })?;
    let mut cards = Vec::new();
    for (number, row) in rows.iter().enumerate().skip(1) {
        let line = number + 1;
        if row.iter().all(|cell| cell.trim().is_empty()) {
            continue;
        }
        if row.len() != header.len() {
            return Err(StoreError::Malformed {
                path: source.to_path_buf(),
                reason: format!(
                    "line {line} holds {} cells, but the header names {} columns",
                    row.len(),
                    header.len()
                ),
            });
        }
        let markdown = columns.get(row, 4);
        #[expect(clippy::map_err_ignore, reason = "the cell is quoted in the reason")]
        let sides =
            columns
                .get(row, 1)
                .trim()
                .parse::<i64>()
                .map_err(|_| StoreError::Malformed {
                    path: source.to_path_buf(),
                    reason: format!(
                        "line {line} says {:?} sides, which is not a number",
                        columns.get(row, 1)
                    ),
                })?;
        let faces = hidden::faces(markdown);
        if sides != faces {
            return Err(StoreError::Malformed {
                path: source.to_path_buf(),
                reason: format!("line {line} says {sides} sides, but its markdown holds {faces}"),
            });
        }
        let tags: Vec<String> =
            json_cell::<Vec<String>>(columns.get(row, 2), Vec::new()).map_err(|reason| {
                StoreError::Malformed {
                    path: source.to_path_buf(),
                    reason: format!("line {line} has tags that are not a JSON array: {reason}"),
                }
            })?;
        let fields: BTreeMap<String, String> =
            json_cell::<BTreeMap<String, String>>(columns.get(row, 3), BTreeMap::new()).map_err(
                |reason| StoreError::Malformed {
                    path: source.to_path_buf(),
                    reason: format!("line {line} has fields that are not a JSON object: {reason}"),
                },
            )?;
        cards.push((markdown.to_string(), tags, fields));
    }

    let mut report = ImportReport::default();
    for (markdown, tags, fields) in cards {
        let outcome = db.staged(&Command::CreateCard { deck, markdown }, now_ms)?;
        let Outcome::Card(id) = outcome else {
            return Err(StoreError::Malformed {
                path: source.to_path_buf(),
                reason: "a card command did not return a card".to_string(),
            });
        };
        if !fields.is_empty() {
            db.staged(
                &Command::SetFields {
                    card: id,
                    fields: fields.into_iter().collect(),
                },
                now_ms,
            )?;
        }
        if !tags.is_empty() {
            db.staged(&Command::SetTags { card: id, tags }, now_ms)?;
        }
        report.cards += 1;
    }
    Ok(report)
}

/// The header's columns, in the file's own order.
#[derive(Debug)]
struct Columns {
    index: [usize; 5],
}

impl Columns {
    fn new(header: &[String]) -> Result<Self, String> {
        let mut index = [0; 5];
        let mut missing = Vec::new();
        for (slot, name) in HEADER.iter().enumerate() {
            match header
                .iter()
                .position(|column| column.trim().eq_ignore_ascii_case(name))
            {
                Some(column) => index[slot] = column,
                None => missing.push(*name),
            }
        }
        if !missing.is_empty() {
            return Err(format!("the header is missing {}", missing.join(", ")));
        }
        Ok(Self { index })
    }

    fn get<'a>(&self, row: &'a [String], slot: usize) -> &'a str {
        row.get(self.index[slot]).map_or("", String::as_str)
    }
}

fn json_cell<T: serde::de::DeserializeOwned>(cell: &str, empty: T) -> Result<T, String> {
    if cell.trim().is_empty() {
        return Ok(empty);
    }
    serde_json::from_str(cell).map_err(|error| error.to_string())
}

/// One cell, quoted only when it has to be.
fn quoted(cell: &str) -> String {
    if cell.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", cell.replace('"', "\"\""))
    } else {
        cell.to_string()
    }
}

/// RFC 4180's field and record rules, with a leading byte-order mark already removed.
fn parse(text: &str) -> Result<Vec<Vec<String>>, String> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut cell = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    while let Some(character) = chars.next() {
        if quoted {
            match character {
                '"' if chars.peek() == Some(&'"') => {
                    chars.next();
                    cell.push('"');
                }
                '"' => quoted = false,
                _ => cell.push(character),
            }
            continue;
        }
        match character {
            '"' if cell.is_empty() => quoted = true,
            ',' => row.push(std::mem::take(&mut cell)),
            '\r' if chars.peek() == Some(&'\n') => {
                chars.next();
                row.push(std::mem::take(&mut cell));
                rows.push(std::mem::take(&mut row));
            }
            '\n' => {
                row.push(std::mem::take(&mut cell));
                rows.push(std::mem::take(&mut row));
            }
            _ => cell.push(character),
        }
    }
    if quoted {
        return Err("a quoted cell is never closed".to_string());
    }
    if !cell.is_empty() || !row.is_empty() {
        row.push(cell);
        rows.push(row);
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_quoted_cell_keeps_commas_quotes_and_newlines() {
        let cells = ["plain", "a,b", "say \"hi\"", "two\nlines", ""];
        let line = cells.map(quoted).join(",");
        assert_eq!(
            parse(&line).expect("the line parses"),
            vec![cells.map(str::to_string).to_vec()]
        );
    }

    #[test]
    fn a_missing_column_is_named() {
        let header = ["deck", "tags", "fields", "markdown"].map(str::to_string);
        let error = Columns::new(&header).expect_err("sides is missing");
        assert!(
            error.to_string().contains("sides"),
            "the reason names the missing column: {error}"
        );
    }

    #[test]
    fn columns_may_arrive_in_another_order() {
        let header = ["markdown", "fields", "deck", "tags", "sides"].map(str::to_string);
        let columns = Columns::new(&header).expect("every column is named");
        let row = ["hola", "{}", "Spanish", "[]", "1"].map(str::to_string);
        assert_eq!(columns.get(&row, 4), "hola");
        assert_eq!(columns.get(&row, 1), "1");
    }
}
