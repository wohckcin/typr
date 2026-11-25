//! Read-side snapshot assembly, settings reads, drafts, masks, and backlinks.

use std::collections::HashMap;

use rusqlite::params;

use crate::error::StoreError;
use crate::library_view::ViewMode;
use crate::model::{
    CardKind, CardRef, CardSummary, Deck, Id, MaskRect, MaskRow, SavedView, SettingKey, Settings,
    Snapshot, Sort, SortDir, ViewSpec,
};

use super::core::{Db, count_of, decode_tags, flag, mode_parse};

impl Db {
    /// Everything a view may read without a round trip.
    pub fn snapshot(&self) -> Result<Snapshot, StoreError> {
        let decks = self.load_decks()?;
        let mut cards = self.load_cards()?;
        self.attach_references(&mut cards)?;
        self.attach_tags(&mut cards)?;
        self.attach_masks(&mut cards)?;
        self.attach_fields(&mut cards)?;
        resolve_inherited_templates(&decks, &mut cards);
        let views = self.load_views()?;
        Ok(Snapshot {
            decks,
            cards,
            views,
            settings: self.settings()?,
            revision: 0,
        })
    }

    fn load_decks(&self) -> Result<Vec<Deck>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, parent_id, name, archived, reverse, template, COALESCE(view, 'grid')
             FROM deck ORDER BY name COLLATE NOCASE, id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Deck {
                id: row.get(0)?,
                parent: row.get(1)?,
                name: row.get(2)?,
                archived: row.get::<_, i64>(3)? != 0,
                reverse: flag(row.get(4)?),
                template: row.get(5)?,
                view: ViewMode::parse(&row.get::<_, String>(6)?),
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    fn load_cards(&self) -> Result<Vec<CardSummary>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT c.id, c.deck_id, c.sides, c.archived, c.trashed_at, c.updated_at, c.markdown,
                    c.search_text, COUNT(sp.card_id), SUM(sp.card_id IS NOT NULL AND sp.due_at IS NULL), MIN(sp.due_at),
                    c.reverse, c.kind, c.image, c.caption, c.template, c.pinned, c.created_at,
                    (SELECT COUNT(*) FROM review_log r WHERE r.card_id = c.id AND r.phase <> 'learn'),
                    c.name, c.position
             FROM card c
             LEFT JOIN studyable_prompt sp ON sp.card_id = c.id
             GROUP BY c.id
             ORDER BY c.updated_at DESC, c.id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(CardSummary {
                id: row.get(0)?,
                deck: row.get(1)?,
                kind: CardKind::parse(&row.get::<_, String>(12)?),
                image: row.get(13)?,
                caption: row.get(14)?,
                masks: Vec::new(),
                sides: count_of(row.get(2)?),
                tags: Vec::new(),
                archived: row.get::<_, i64>(3)? != 0,
                reverse: flag(row.get(11)?),
                trashed_at: row.get(4)?,
                updated_at: row.get(5)?,
                created_at: row.get(17)?,
                reviews: count_of(row.get(18)?),
                pinned: row.get::<_, i64>(16)? != 0,
                preview: preview(&row.get::<_, String>(6)?),
                name: row.get(19)?,
                position: row.get(20)?,
                markdown: row.get(6)?,
                search_text: row.get(7)?,
                prompts: count_of(row.get(8)?),
                unscheduled: count_of(row.get(9)?),
                due_at: row.get(10)?,
                fields: Vec::new(),
                // A card's own template, if it has one; the cards that inherit are filled in
                // once every deck is in hand.
                template: row.get(15)?,
                template_from: None,
                references: Vec::new(),
                backlinks: Vec::new(),
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// One join answers both directions: what each card refers to, and who refers to it.
    /// The name comes from the card as it is now, so a rename shows up in both lists.
    fn attach_references(&self, cards: &mut [CardSummary]) -> Result<(), StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT r.from_card, f.name, t.id, t.name
             FROM reference r
             JOIN card f ON f.id = r.from_card
             JOIN card t ON t.id = r.to_card
             ORDER BY r.from_card, t.id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, Id>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Id>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;
        let mut outgoing: HashMap<Id, Vec<CardRef>> = HashMap::new();
        let mut incoming: HashMap<Id, Vec<CardRef>> = HashMap::new();
        for row in rows {
            let (from, from_name, to, to_name) = row?;
            outgoing.entry(from).or_default().push(CardRef {
                id: to,
                name: to_name,
            });
            incoming.entry(to).or_default().push(CardRef {
                id: from,
                name: from_name,
            });
        }
        for summary in cards {
            if let Some(targets) = outgoing.remove(&summary.id) {
                summary.references = targets;
            }
            if let Some(referrers) = incoming.remove(&summary.id) {
                summary.backlinks = referrers;
            }
        }
        Ok(())
    }

    fn attach_tags(&self, cards: &mut [CardSummary]) -> Result<(), StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT ct.card_id, t.name FROM card_tag ct JOIN tag t ON t.id = ct.tag_id
             ORDER BY t.name COLLATE NOCASE",
        )?;
        let pairs = stmt.query_map([], |row| {
            Ok((row.get::<_, Id>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut by_card: HashMap<Id, Vec<String>> = HashMap::new();
        for pair in pairs {
            let (card, name) = pair?;
            by_card.entry(card).or_default().push(name);
        }
        for summary in cards {
            if let Some(names) = by_card.remove(&summary.id) {
                summary.tags = names;
            }
        }
        Ok(())
    }

    /// Only a diagram card carries rectangles, so the second pass is keyed by the cards that
    /// need them and leaves every text card alone.
    fn attach_masks(&self, cards: &mut [CardSummary]) -> Result<(), StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT card_id, group_no, x, y, w, h FROM mask ORDER BY card_id, group_no, y, x",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, Id>(0)?,
                MaskRow {
                    group: count_of(row.get::<_, i64>(1)?),
                    rect: MaskRect::new(row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?),
                },
            ))
        })?;
        let mut by_card: HashMap<Id, Vec<MaskRow>> = HashMap::new();
        for row in rows {
            let (card, mask) = row?;
            by_card.entry(card).or_default().push(mask);
        }
        for summary in cards {
            if let Some(masks) = by_card.remove(&summary.id) {
                summary.masks = masks;
            }
        }
        Ok(())
    }

    fn attach_fields(&self, cards: &mut [CardSummary]) -> Result<(), StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT card_id, name, value FROM field ORDER BY card_id, name")?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, Id>(0)?,
                (row.get::<_, String>(1)?, row.get::<_, String>(2)?),
            ))
        })?;
        let mut by_card: HashMap<Id, Vec<(String, String)>> = HashMap::new();
        for row in rows {
            let (card, field) = row?;
            by_card.entry(card).or_default().push(field);
        }
        for summary in cards {
            if let Some(fields) = by_card.remove(&summary.id) {
                summary.fields = fields;
            }
        }
        Ok(())
    }

    fn load_views(&self) -> Result<Vec<SavedView>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, deck, query, tags, mode, pinned_only, sort_key, sort_dir
             FROM view ORDER BY name COLLATE NOCASE, id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(SavedView {
                id: row.get(0)?,
                name: row.get(1)?,
                spec: ViewSpec {
                    deck: row.get(2)?,
                    query: row.get(3)?,
                    tags: decode_tags(&row.get::<_, String>(4)?),
                    mode: mode_parse(&row.get::<_, String>(5)?),
                    pinned_only: row.get::<_, i64>(6)? != 0,
                    sort: Sort::parse(&row.get::<_, String>(7)?).unwrap_or_default(),
                    dir: SortDir::parse(&row.get::<_, String>(8)?).unwrap_or_default(),
                },
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn settings(&self) -> Result<Settings, StoreError> {
        let mut settings = Settings::default();
        let mut stmt = self.conn.prepare("SELECT key, value FROM settings")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (key, text) = row?;
            // A key this build does not know is a setting a later build wrote; leaving it in
            // place is what lets a library move between builds without losing it.
            if let Some(key) = SettingKey::parse(&key) {
                let value = key.parse_value(&text)?;
                key.assign(&mut settings, value);
            }
        }
        Ok(settings)
    }

    /// The text as it was left, if it was left unsaved.
    pub fn draft(&self, card: Id) -> Result<Option<String>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT text FROM editor_draft WHERE card_id = ?1")?;
        let mut rows = stmt.query(params![card])?;
        match rows.next()? {
            Some(row) => Ok(Some(row.get(0)?)),
            None => Ok(None),
        }
    }

    /// The groups a diagram card asks about, in ascending order. A group exists when a rectangle
    /// names it, so removing the last rectangle of a group takes the group away with it.
    pub fn mask_groups(&self, card: Id) -> Result<Vec<u32>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT group_no FROM mask WHERE card_id = ?1 ORDER BY group_no")?;
        let rows = stmt.query_map(params![card], |row| row.get::<_, i64>(0))?;
        Ok(rows
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(count_of)
            .collect())
    }

    /// Every rectangle of a diagram card, ordered by group and then top-left first.
    pub fn masks(&self, card: Id) -> Result<Vec<MaskRow>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT group_no, x, y, w, h FROM mask WHERE card_id = ?1
             ORDER BY group_no, y, x",
        )?;
        let rows = stmt.query_map(params![card], |row| {
            Ok(MaskRow {
                group: count_of(row.get::<_, i64>(0)?),
                rect: MaskRect::new(row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?),
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// The cards whose text refers to `card`.
    ///
    /// Read off one snapshot rather than a query per card, so a caller and the list cannot disagree
    /// about which references exist.
    pub fn backlinks(&self, card: Id) -> Result<Vec<CardSummary>, StoreError> {
        Ok(self
            .snapshot()?
            .cards
            .into_iter()
            .filter(|other| other.references.iter().any(|target| target.id == card))
            .collect())
    }
}

/// The template a card without one of its own inherits: the nearest deck above it that
/// sets one, resolved here rather than by a query per card.
fn resolve_inherited_templates(decks: &[Deck], cards: &mut [CardSummary]) {
    let chains: HashMap<Id, (Option<Id>, Option<&str>)> = decks
        .iter()
        .map(|deck| (deck.id, (deck.parent, deck.template.as_deref())))
        .collect();
    for summary in cards {
        if summary.template.is_some() {
            continue;
        }
        let mut cursor = Some(summary.deck);
        let mut hops = 0;
        while let Some(id) = cursor {
            hops += 1;
            if hops > 64 {
                break;
            }
            let Some((parent, template)) = chains.get(&id) else {
                break;
            };
            if let Some(template) = template {
                summary.template = Some((*template).to_string());
                summary.template_from = Some(id);
                break;
            }
            cursor = *parent;
        }
    }
}

/// The first line of a card, cut to what a list row can show.
fn preview(markdown: &str) -> String {
    let line = markdown
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("");
    let mut cut: String = line.chars().take(80).collect();
    if line.chars().count() > 80 {
        cut.push('…');
    }
    cut
}
