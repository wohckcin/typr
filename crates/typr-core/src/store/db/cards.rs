//! Card mutations, diagrams/masks, templates, references, prompts, and tags.

use rusqlite::{OptionalExtension, params};

use crate::command::{Command, Outcome};
use crate::edits;
use crate::error::StoreError;
use crate::hidden::{self, Shape};
use crate::model::{Id, MaskRect, folded};
use crate::template;

use super::{Db, check_mask};

impl Db {
    pub(super) fn dispatch_diagram(
        &self,
        command: &Command,
        now_ms: i64,
    ) -> Result<Option<Outcome>, StoreError> {
        let outcome = match command {
            Command::CreateDiagram {
                deck,
                image,
                caption,
            } => self.create_diagram(*deck, image, caption, now_ms)?,
            Command::SetDiagram {
                card,
                image,
                caption,
            } => self.set_diagram(*card, image, caption, now_ms)?,
            Command::AddMask { card, group, rect } => {
                check_mask(*card, *rect)?;
                self.place_mask(*card, *rect, *group)?;
                self.sync_prompts(*card, &self.shape_of(*card)?)?;
                Outcome::Changed(1)
            }
            Command::SetMaskGroup { card, rect, group } => {
                self.set_mask_group(*card, *rect, *group)?
            }
            Command::RemoveMask { card, rect } => self.remove_mask(*card, *rect)?,
            Command::RemoveMaskGroup { card, group } => self.remove_mask_group(*card, *group)?,
            Command::ClearDiagram { card } => self.clear_diagram(*card, now_ms)?,
            Command::MergeMasks { card, rects, group } => self.merge_masks(*card, rects, *group)?,
            Command::MaskGroups { card } => Outcome::MaskGroups(self.mask_groups(*card)?),
            Command::Masks { card } => Outcome::Masks(self.masks(*card)?),
            _ => return Ok(None),
        };
        Ok(Some(outcome))
    }

    pub(super) fn dispatch_card(
        &self,
        command: &Command,
        now_ms: i64,
    ) -> Result<Option<Outcome>, StoreError> {
        let outcome = match command {
            Command::CreateCard { deck, markdown } => self.create_card(*deck, markdown, now_ms)?,
            Command::SaveCard { id, markdown } => self.save_card(*id, markdown, now_ms)?,
            Command::MoveCard { id, deck } => {
                let changed = self.conn.execute(
                    "UPDATE card SET deck_id = ?2, updated_at = ?3 WHERE id = ?1",
                    params![id, deck, now_ms],
                )?;
                // The card may have landed under a deck that sets a template, or left one behind.
                self.rerender(*id, now_ms)?;
                Outcome::Changed(changed)
            }
            Command::MoveCardTo { card, position } => self.move_card_to(*card, *position)?,
            Command::SetFields { card, fields } => self.set_fields(*card, fields, now_ms)?,
            Command::SetCardTemplate {
                card,
                template,
                name,
            } => self.set_card_template(*card, template.as_deref(), name.as_deref(), now_ms)?,
            Command::SetTags { card, tags } => self.set_tags(*card, tags)?,
            Command::SetCardReverse { card, reverse } => {
                // NULL is "inherit", so the nearest deck's value applies again.
                let changed = self.conn.execute(
                    "UPDATE card SET reverse = ?2 WHERE id = ?1",
                    params![card, reverse.map(i64::from)],
                )?;
                if changed == 0 {
                    return Err(StoreError::Missing {
                        what: "card",
                        id: *card,
                    });
                }
                Outcome::Changed(changed)
            }
            _ => return Ok(None),
        };
        Ok(Some(outcome))
    }

    pub(super) fn dispatch_card_state(
        &self,
        command: &Command,
        now_ms: i64,
    ) -> Result<Option<Outcome>, StoreError> {
        let outcome = match command {
            Command::ArchiveCard { id, archived } => {
                let changed = self.conn.execute(
                    "UPDATE card SET archived = ?2, updated_at = ?3 WHERE id = ?1",
                    params![id, i64::from(*archived), now_ms],
                )?;
                Outcome::Changed(changed)
            }
            Command::EmptyTrash => {
                let removed = self
                    .conn
                    .execute("DELETE FROM card WHERE trashed_at IS NOT NULL", [])?;
                Outcome::Changed(removed)
            }
            Command::TrashCard { id } => {
                let changed = self.conn.execute(
                    "UPDATE card SET trashed_at = ?2 WHERE id = ?1",
                    params![id, now_ms],
                )?;
                self.rebuild_card_name(*id)?;
                Outcome::Changed(changed)
            }
            Command::RestoreCard { id } => {
                let changed = self.conn.execute(
                    "UPDATE card SET trashed_at = NULL WHERE id = ?1",
                    params![id],
                )?;
                self.rebuild_card_name(*id)?;
                Outcome::Changed(changed)
            }
            Command::SetPinned { card, pinned } => Outcome::Changed(self.conn.execute(
                "UPDATE card SET pinned = ?2 WHERE id = ?1",
                params![card, i64::from(*pinned)],
            )?),
            Command::SetDraft { card, text } => self.set_draft(*card, text.as_deref(), now_ms)?,
            Command::Draft { card } => Outcome::Draft(self.draft(*card)?),
            _ => return Ok(None),
        };
        Ok(Some(outcome))
    }

    fn create_card(&self, deck: Id, markdown: &str, now_ms: i64) -> Result<Outcome, StoreError> {
        let shape = hidden::shape(markdown);
        self.conn.execute(
            "INSERT INTO card (deck_id, markdown, sides, search_text, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
            params![
                deck,
                markdown,
                hidden::faces(markdown),
                folded(markdown),
                now_ms
            ],
        )?;
        let id = self.conn.last_insert_rowid();
        self.sync_prompts(id, &shape)?;
        // A card created straight into a templated deck says what its template says, not
        // what the caller typed; without fields it renders empty, which is the truth.
        self.rerender(id, now_ms)?;
        self.conn.execute(
            "DELETE FROM editor_draft WHERE card_id IN (0, ?1)",
            params![id],
        )?;
        Ok(Outcome::Card(id))
    }

    fn save_card(&self, id: Id, markdown: &str, now_ms: i64) -> Result<Outcome, StoreError> {
        // A diagram's prompt comes from its rectangles, so text written over one would be
        // stored and never asked.
        if self.kind_of(id)? == "diagram" {
            return Err(StoreError::NotEmpty {
                card: id,
                holds: "a picture",
            });
        }
        // A templated card's text is rendered from its fields, so a hand-written one would
        // survive only until the next field edit overwrote it.
        if self.template_of(id)?.is_some() {
            return Err(StoreError::NotEmpty {
                card: id,
                holds: "a template",
            });
        }
        if !edits::has_content(markdown) {
            return Err(StoreError::EmptyCard { card: id });
        }
        let shape = hidden::shape(markdown);
        let changed = self.conn.execute(
            "UPDATE card SET markdown = ?2, sides = ?3, search_text = ?4, updated_at = ?5 WHERE id = ?1",
            params![id, markdown, hidden::faces(markdown), folded(markdown), now_ms],
        )?;
        self.sync_prompts(id, &shape)?;
        self.conn
            .execute("DELETE FROM editor_draft WHERE card_id = ?1", params![id])?;
        self.rename(id)?;
        Ok(Outcome::Changed(changed))
    }

    fn create_diagram(
        &self,
        deck: Id,
        image: &str,
        caption: &str,
        now_ms: i64,
    ) -> Result<Outcome, StoreError> {
        self.conn.execute(
            "INSERT INTO card
                (deck_id, kind, image, caption, markdown, sides, search_text,
                 created_at, updated_at)
             VALUES (?1, 'diagram', ?2, ?3, '', 0, '', ?4, ?4)",
            params![deck, image, caption, now_ms],
        )?;
        let id = self.conn.last_insert_rowid();
        self.sync_prompts(id, &hidden::shape(""))?;
        self.conn.execute(
            "DELETE FROM editor_draft WHERE card_id IN (0, ?1)",
            params![id],
        )?;
        Ok(Outcome::Card(id))
    }

    fn set_diagram(
        &self,
        card: Id,
        image: &str,
        caption: &str,
        now_ms: i64,
    ) -> Result<Outcome, StoreError> {
        // Text already on the card would be hidden rather than kept, so the switch waits
        // until it is cleared.
        let holds_text = self.conn.query_row(
            "SELECT markdown <> '' OR has_cloze = 1 FROM card WHERE id = ?1",
            params![card],
            |row| row.get::<_, bool>(0),
        )?;
        if holds_text {
            return Err(StoreError::NotEmpty {
                card,
                holds: "text",
            });
        }
        let changed = self.conn.execute(
            "UPDATE card SET kind = 'diagram', image = ?2, caption = ?3, updated_at = ?4
             WHERE id = ?1",
            params![card, image, caption, now_ms],
        )?;
        if changed == 0 {
            return Err(StoreError::Missing {
                what: "card",
                id: card,
            });
        }
        self.sync_prompts(card, &self.shape_of(card)?)?;
        Ok(Outcome::Changed(changed))
    }

    fn set_mask_group(&self, card: Id, rect: MaskRect, group: u32) -> Result<Outcome, StoreError> {
        check_mask(card, rect)?;
        let changed = self.conn.execute(
            "UPDATE mask SET group_no = ?2 WHERE card_id = ?1 AND x = ?3 AND y = ?4 AND w = ?5 AND h = ?6",
            params![card, group, rect.x, rect.y, rect.w, rect.h],
        )?;
        if changed == 0 {
            return Err(StoreError::Missing {
                what: "mask",
                id: card,
            });
        }
        self.sync_prompts(card, &self.shape_of(card)?)?;
        Ok(Outcome::Changed(changed))
    }

    fn remove_mask(&self, card: Id, rect: MaskRect) -> Result<Outcome, StoreError> {
        let changed = self.conn.execute(
            "DELETE FROM mask WHERE card_id = ?1 AND x = ?2 AND y = ?3 AND w = ?4 AND h = ?5",
            params![card, rect.x, rect.y, rect.w, rect.h],
        )?;
        if changed == 0 {
            return Err(StoreError::Missing {
                what: "mask",
                id: card,
            });
        }
        self.sync_prompts(card, &self.shape_of(card)?)?;
        Ok(Outcome::Changed(changed))
    }

    fn remove_mask_group(&self, card: Id, group: u32) -> Result<Outcome, StoreError> {
        let changed = self.conn.execute(
            "DELETE FROM mask WHERE card_id = ?1 AND group_no = ?2",
            params![card, group],
        )?;
        if changed == 0 {
            return Err(StoreError::Missing {
                what: "group",
                id: card,
            });
        }
        self.sync_prompts(card, &self.shape_of(card)?)?;
        Ok(Outcome::Changed(changed))
    }

    fn clear_diagram(&self, card: Id, now_ms: i64) -> Result<Outcome, StoreError> {
        if self.kind_of(card)? != "diagram" {
            return Err(StoreError::Missing {
                what: "diagram",
                id: card,
            });
        }
        // The picture, its rectangles and the caption go; the file stays in the attachments
        // folder, and the prompt rows are retired rather than deleted, so history reads.
        self.conn
            .execute("DELETE FROM mask WHERE card_id = ?1", params![card])?;
        self.conn.execute(
            "UPDATE card SET kind = 'text', image = NULL, caption = NULL, markdown = '',
                sides = 0, search_text = '', updated_at = ?2
             WHERE id = ?1",
            params![card, now_ms],
        )?;
        self.sync_prompts(card, &hidden::shape(""))?;
        Ok(Outcome::Changed(1))
    }

    fn merge_masks(&self, card: Id, rects: &[MaskRect], group: u32) -> Result<Outcome, StoreError> {
        let mut changed = 0;
        for rect in rects {
            check_mask(card, *rect)?;
            changed += self.place_mask(card, *rect, group)?;
        }
        if changed == 0 {
            return Err(StoreError::Missing {
                what: "mask",
                id: card,
            });
        }
        self.sync_prompts(card, &self.shape_of(card)?)?;
        Ok(Outcome::Changed(changed))
    }

    fn set_draft(&self, card: Id, text: Option<&str>, now_ms: i64) -> Result<Outcome, StoreError> {
        match text {
            Some(text) => {
                self.conn.execute(
                    "INSERT INTO editor_draft (card_id, text, updated_at) VALUES (?1, ?2, ?3)
                     ON CONFLICT (card_id) DO UPDATE SET text = excluded.text, updated_at = excluded.updated_at",
                    params![card, text, now_ms],
                )?;
            }
            None => {
                self.conn
                    .execute("DELETE FROM editor_draft WHERE card_id = ?1", params![card])?;
            }
        }
        Ok(Outcome::Ok)
    }

    /// The fields a card fills its template from, ordered by name.
    fn fields_of(&self, card: Id) -> Result<Vec<(String, String)>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT name, value FROM field WHERE card_id = ?1 ORDER BY name")?;
        let rows = stmt.query_map(params![card], |row| Ok((row.get(0)?, row.get(1)?)))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// The template a card's text comes from, and the deck it is inherited from when it is not the
    /// card's own: `None` in the second place means the card carries it.
    ///
    /// Inheritance walks up the deck tree the way reverse does, so the nearest deck that sets a
    /// template wins. A card with no template anywhere answers `None`, which is what makes a
    /// library without templates behave exactly as it did before them.
    fn template_of(&self, card: Id) -> Result<Option<(String, Option<Id>)>, StoreError> {
        let row: Option<(Option<String>, Id)> = self
            .conn
            .query_row(
                "SELECT template, deck_id FROM card WHERE id = ?1",
                params![card],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let Some((own, deck)) = row else {
            return Ok(None);
        };
        if let Some(template) = own {
            return Ok(Some((template, None)));
        }
        let mut cursor = Some(deck);
        let mut hops = 0;
        while let Some(id) = cursor {
            // A cycle the schema does not allow still cannot be allowed to hang a render.
            hops += 1;
            if hops > 64 {
                return Ok(None);
            }
            let (template, parent): (Option<String>, Option<Id>) = self.conn.query_row(
                "SELECT template, parent_id FROM deck WHERE id = ?1",
                params![id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;
            if let Some(template) = template {
                return Ok(Some((template, Some(id))));
            }
            cursor = parent;
        }
        Ok(None)
    }

    /// The name of the deck a card sits in, which `<<Deck>>` renders.
    fn deck_name_of(&self, card: Id) -> Result<Option<String>, StoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT d.name FROM card c JOIN deck d ON d.id = c.deck_id WHERE c.id = ?1",
                params![card],
                |row| row.get(0),
            )
            .optional()?)
    }

    /// Write a card's text from its template and fields, then name it from the text and fields it
    /// just wrote.
    ///
    /// `card.markdown` is a cache of `render(template, fields)`: every mutation that can change
    /// either calls this, so `sides`, `search_text` and the review can go on reading one plain
    /// string. A card with no template in force keeps the text it holds — which is what makes
    /// clearing a template leave the cards readable — but it is still named here, because a rename
    /// and the references that point at it have to move together.
    pub(super) fn rerender(&self, card: Id, now_ms: i64) -> Result<(), StoreError> {
        // A diagram is asked from its rectangles, and its text is not what a template is for;
        // writing rendered text over it would queue text prompts it never shows. It has no name
        // either: a `[[reference]]` is text, and a diagram holds a picture.
        if self.kind_of(card)? == "diagram" {
            return Ok(());
        }
        if let Some((template, _from)) = self.template_of(card)? {
            let pairs = self.fields_of(card)?;
            let deck = self.deck_name_of(card)?;
            let text = template::render_card(&template, &pairs, deck.as_deref());
            let shape = hidden::shape(&text);
            // A value the template does not use is still part of what the card says, so search has
            // to reach it.
            let mut search = folded(&text);
            for (_, value) in &pairs {
                search.push(' ');
                search.push_str(&folded(value));
            }
            self.conn.execute(
                "UPDATE card SET markdown = ?2, sides = ?3, search_text = ?4, updated_at = ?5 WHERE id = ?1",
                params![card, text, hidden::faces(&text), search, now_ms],
            )?;
            self.sync_prompts(card, &shape)?;
        }
        self.rename(card)
    }

    /// Give a card the name its text and name template imply.
    ///
    /// Runs inside the save's transaction, after the text is written: the name is what `[[…]]`
    /// resolves against, so a card whose text is new and whose name is old would answer to a name
    /// it no longer says.
    fn rename(&self, card: Id) -> Result<(), StoreError> {
        let (markdown, current, template): (String, String, Option<String>) = self.conn.query_row(
            "SELECT markdown, name, name_template FROM card WHERE id = ?1",
            params![card],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        let first = crate::sides::split(&markdown)
            .first()
            .copied()
            .unwrap_or("");
        let pairs = self.fields_of(card)?;
        let wanted =
            crate::name::for_card(first, template.as_deref(), &template::Fields::new(&pairs));
        let name = self.unique_name(card, &wanted)?;
        if name != current {
            self.conn.execute(
                "UPDATE card SET name = ?2 WHERE id = ?1",
                params![card, name],
            )?;
            // A card that gains or loses a name changes what `[[that name]]` means everywhere it is
            // written, so the rows that pointed at it — and the ones that could not — are rebuilt.
            // Skipping this when the name is unchanged keeps a deck-wide re-render from scanning
            // the library once per card.
            self.rebuild_named(&current)?;
            self.rebuild_named(&name)?;
        }
        self.rebuild_references(card)?;
        Ok(())
    }

    /// Rebuild one card's reference rows from its text, inside the save's transaction.
    ///
    /// The rows are a second record of what the text says, so deleting and re-inserting them in the
    /// same transaction as the text is what keeps a backlink from outliving the words that made it.
    pub fn rebuild_references(&self, card: Id) -> Result<(), StoreError> {
        self.conn
            .execute("DELETE FROM reference WHERE from_card = ?1", params![card])?;
        let markdown: String = self.conn.query_row(
            "SELECT markdown FROM card WHERE id = ?1",
            params![card],
            |row| row.get(0),
        )?;
        for name in crate::reference::links(&markdown) {
            let Some(target) = self.resolve(card, name)? else {
                continue;
            };
            self.conn.execute(
                "INSERT OR IGNORE INTO reference (from_card, to_card) VALUES (?1, ?2)",
                params![card, target],
            )?;
        }
        Ok(())
    }

    /// The card `name` refers to from `from`, if any.
    ///
    /// The referring card's own deck comes first, then its ancestors, then the whole library: a
    /// deck of notes refers to its own "Introduction" rather than a distant one. Folding is the
    /// rule names are made unique with, so the two cannot disagree about whether two names are the
    /// same, and ties go to the oldest card, so one text always opens one card.
    fn resolve(&self, from: Id, name: &str) -> Result<Option<Id>, StoreError> {
        let wanted = folded(name);
        let mut deck = self
            .conn
            .query_row(
                "SELECT deck_id FROM card WHERE id = ?1",
                params![from],
                |row| row.get::<_, Id>(0),
            )
            .optional()?;
        while let Some(id) = deck {
            if let Some(found) = self.oldest_named(Some(id), &wanted)? {
                return Ok(Some(found));
            }
            deck = self
                .conn
                .query_row(
                    "SELECT parent_id FROM deck WHERE id = ?1",
                    params![id],
                    |row| row.get::<_, Option<Id>>(0),
                )
                .optional()?
                .flatten();
        }
        self.oldest_named(None, &wanted)
    }

    /// The oldest card answering to `wanted`, in one deck or anywhere; a trashed card does not
    /// answer to its name, so a reference never points into the trash.
    fn oldest_named(&self, deck: Option<Id>, wanted: &str) -> Result<Option<Id>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name FROM card
             WHERE (?1 IS NULL OR deck_id = ?1) AND trashed_at IS NULL AND name <> ''
             ORDER BY id",
        )?;
        let rows = stmt.query_map(params![deck], |row| {
            Ok((row.get::<_, Id>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (id, name) = row?;
            if folded(&name) == wanted {
                return Ok(Some(id));
            }
        }
        Ok(None)
    }

    /// Re-resolve the cards that name `card`, after it enters or leaves the trash.
    fn rebuild_card_name(&self, card: Id) -> Result<(), StoreError> {
        let name: Option<String> = self
            .conn
            .query_row(
                "SELECT name FROM card WHERE id = ?1",
                params![card],
                |row| row.get(0),
            )
            .optional()?;
        self.rebuild_named(&name.unwrap_or_default())
    }

    /// Re-resolve every card whose text names `name`.
    fn rebuild_named(&self, name: &str) -> Result<(), StoreError> {
        if name.is_empty() {
            return Ok(());
        }
        let wanted = folded(name);
        // Only a card that could hold a reference is worth reading: the same cheap prefilter the
        // index cannot help with, because a reference is inside the text rather than a column.
        let named = {
            let mut stmt = self.conn.prepare(
                "SELECT id, markdown FROM card WHERE instr(markdown, '[') > 0 ORDER BY id",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, Id>(0)?, row.get::<_, String>(1)?))
            })?;
            let mut found = Vec::new();
            for row in rows {
                let (id, markdown) = row?;
                if crate::reference::links(&markdown)
                    .iter()
                    .any(|link| folded(link) == wanted)
                {
                    found.push(id);
                }
            }
            found
        };
        for id in named {
            self.rebuild_references(id)?;
        }
        Ok(())
    }

    /// `name` made unique among the cards in its deck, which is the scope a reference resolves in
    /// first.
    ///
    /// The card being named is left out of the comparison, so a suffixed name survives its own
    /// saves instead of gaining a second suffix each time.
    fn unique_name(&self, card: Id, name: &str) -> Result<String, StoreError> {
        if name.is_empty() {
            return Ok(String::new());
        }
        let deck: Id = self.conn.query_row(
            "SELECT deck_id FROM card WHERE id = ?1",
            params![card],
            |row| row.get(0),
        )?;
        let mut stmt = self
            .conn
            .prepare("SELECT name FROM card WHERE deck_id = ?1 AND id <> ?2 ORDER BY id")?;
        let rows = stmt.query_map(params![deck, card], |row| row.get::<_, String>(0))?;
        let taken = rows.collect::<Result<Vec<_>, _>>()?;
        let taken = taken.iter().map(String::as_str).collect::<Vec<_>>();
        Ok(crate::name::unique(name, &taken))
    }

    /// Replace a card's fields and its rendered text with them.
    fn set_fields(
        &self,
        card: Id,
        fields: &[(String, String)],
        now_ms: i64,
    ) -> Result<Outcome, StoreError> {
        self.require_card(card)?;
        self.conn
            .execute("DELETE FROM field WHERE card_id = ?1", params![card])?;
        let mut changed = 0;
        for (name, value) in fields {
            let name = name.trim();
            if name.is_empty() {
                continue;
            }
            changed += self.conn.execute(
                "INSERT INTO field (card_id, name, value) VALUES (?1, ?2, ?3)
                 ON CONFLICT (card_id, name) DO UPDATE SET value = excluded.value",
                params![card, name, value],
            )?;
        }
        self.rerender(card, now_ms)?;
        Ok(Outcome::Changed(changed))
    }

    /// Give a card its own text template and name template, or `None` to fall back for either.
    fn set_card_template(
        &self,
        card: Id,
        template: Option<&str>,
        name: Option<&str>,
        now_ms: i64,
    ) -> Result<Outcome, StoreError> {
        let changed = self.conn.execute(
            "UPDATE card SET template = ?2, name_template = ?3, updated_at = ?4 WHERE id = ?1",
            params![card, template, name, now_ms],
        )?;
        if changed == 0 {
            return Err(StoreError::Missing {
                what: "card",
                id: card,
            });
        }
        self.rerender(card, now_ms)?;
        Ok(Outcome::Changed(changed))
    }

    /// Put a card at `position` in its deck, renumbering the whole deck around it.
    ///
    /// The order is the deck's, not a timestamp's, so every card of the deck is rewritten in one
    /// pass: a card inserted at 3 pushes the ones after it down by one, and a card moved to the end
    /// leaves no gap behind. A `position` past the end puts the card last rather than failing, so a
    /// stale keypress cannot refuse the move the user can see.
    ///
    /// Only `card.position` changes. No prompt, no review record and no `updated_at` is touched:
    /// reordering says where a card is read, not when it was last edited.
    ///
    /// The caller's transaction is `Db::apply`'s, so a failure halfway leaves the deck as it was.
    fn move_card_to(&self, card: Id, position: u32) -> Result<Outcome, StoreError> {
        let deck: Id = self
            .conn
            .query_row(
                "SELECT deck_id FROM card WHERE id = ?1",
                params![card],
                |row| row.get(0),
            )
            .optional()?
            .ok_or(StoreError::Missing {
                what: "card",
                id: card,
            })?;
        let mut order = {
            let mut stmt = self
                .conn
                .prepare("SELECT id FROM card WHERE deck_id = ?1 ORDER BY position, id")?;
            let rows = stmt.query_map(params![deck], |row| row.get::<_, Id>(0))?;
            rows.collect::<Result<Vec<_>, _>>()?
        };
        let from = order
            .iter()
            .position(|each| *each == card)
            .ok_or(StoreError::Missing {
                what: "card",
                id: card,
            })?;
        order.remove(from);
        let to = usize::try_from(position)
            .unwrap_or(order.len())
            .min(order.len());
        order.insert(to, card);
        {
            let mut stmt = self
                .conn
                .prepare("UPDATE card SET position = ?2 WHERE id = ?1")?;
            for (index, each) in order.iter().enumerate() {
                let index = i64::try_from(index).unwrap_or(i64::MAX);
                stmt.execute(params![each, index])?;
            }
        }
        Ok(Outcome::Changed(order.len()))
    }

    /// Refuse a card id that is not there, so a command never reports a write it did not make.
    fn require_card(&self, card: Id) -> Result<(), StoreError> {
        let found = self
            .conn
            .query_row(
                "SELECT 1 FROM card WHERE id = ?1",
                params![card],
                |_| Ok(()),
            )
            .optional()?;
        found.ok_or(StoreError::Missing {
            what: "card",
            id: card,
        })
    }

    /// The card's kind, or `text` for a card that is no longer there: a missing card is reported by
    /// the command that asked, not by this read.
    fn kind_of(&self, card: Id) -> Result<String, StoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT kind FROM card WHERE id = ?1",
                params![card],
                |row| row.get(0),
            )
            .optional()?
            .unwrap_or_else(|| "text".to_string()))
    }

    /// Whether the card is a diagram, and what it hides, in one read.
    fn shape_of(&self, card: Id) -> Result<Shape, StoreError> {
        let markdown: String = self.conn.query_row(
            "SELECT markdown FROM card WHERE id = ?1",
            params![card],
            |row| row.get(0),
        )?;
        Ok(hidden::shape(&markdown))
    }

    /// Put one rectangle in one group. A rectangle belongs to one group, so an existing row with
    /// the same corners is replaced rather than duplicated under a second group.
    fn place_mask(&self, card: Id, rect: MaskRect, group: u32) -> Result<usize, StoreError> {
        self.conn.execute(
            "DELETE FROM mask WHERE card_id = ?1 AND x = ?2 AND y = ?3 AND w = ?4 AND h = ?5",
            params![card, rect.x, rect.y, rect.w, rect.h],
        )?;
        self.conn.execute(
            "INSERT INTO mask (card_id, group_no, x, y, w, h) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![card, group, rect.x, rect.y, rect.w, rect.h],
        )?;
        Ok(1)
    }

    /// The prompt keys a card holds. A card that became a note keeps them: its schedule, due date
    /// and history stay intact and come back when it has two sides again.
    pub fn prompt_keys(&self, card: Id) -> Result<Vec<String>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT key FROM prompt WHERE card_id = ?1 ORDER BY key")?;
        let rows = stmt.query_map(params![card], |row| row.get::<_, String>(0))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Give a card the prompts its shape asks for: `fwd` for two or more sides, one `cloze:<n>` per
    /// hidden-text group. A prompt that exists is left exactly as it was, so editing text never resets
    /// a schedule, and nothing is deleted: a group whose spans are gone keeps its history, is marked
    /// retired and stops queueing, and asking for it again clears the mark.
    fn sync_prompts(&self, card: Id, shape: &Shape) -> Result<(), StoreError> {
        // A two-sided card is asked from both ends, so `rev` is written beside the face keys and has
        // to appear in the claim set below: a key the shape does not claim is retired by this very
        // call, which would leave the reverse row unanswerable from the moment it was written.
        // Nothing here reads a reverse setting, so toggling reverse never writes a prompt.
        // A diagram card is asked about its groups instead, so its keys come from the mask table and
        // the text shape it carries (which is empty) adds nothing.
        let mut keys = shape.claimed_keys();
        if self.kind_of(card)? == "diagram" {
            keys.extend(
                self.mask_groups(card)?
                    .into_iter()
                    .map(|group| format!("mask:{group}")),
            );
        }
        for key in &keys {
            self.conn.execute(
                "INSERT INTO prompt (card_id, key, phase, due_at, interval_ms, reps, lapses)
                 VALUES (?1, ?2, 'learn', NULL, 0, 0, 0)
                 ON CONFLICT (card_id, key) DO NOTHING",
                params![card, key],
            )?;
        }
        // A key the shape no longer claims cannot be answered: masking it hides nothing, so it would
        // ask the question and print the answer. The row stays for its history.
        self.conn.execute(
            "UPDATE prompt SET retired = 1 WHERE card_id = ?1",
            params![card],
        )?;
        for key in &keys {
            self.conn.execute(
                "UPDATE prompt SET retired = 0 WHERE card_id = ?1 AND key = ?2",
                params![card, key],
            )?;
        }
        self.conn.execute(
            "UPDATE card SET has_cloze = ?2 WHERE id = ?1",
            params![card, i64::from(shape.hides_text())],
        )?;
        Ok(())
    }

    /// Replace a card's tags. Matching is case-insensitive and the first casing ever written wins,
    /// so `Grammar` stays `Grammar` even when a later card types `grammar`.
    pub fn set_tags(&self, card: Id, tags: &[String]) -> Result<Outcome, StoreError> {
        self.conn
            .execute("DELETE FROM card_tag WHERE card_id = ?1", params![card])?;
        for tag in tags {
            let name = tag.trim();
            if name.is_empty() {
                continue;
            }
            self.conn.execute(
                "INSERT INTO tag (name) VALUES (?1) ON CONFLICT DO NOTHING",
                params![name],
            )?;
            let id: Id = self.conn.query_row(
                "SELECT id FROM tag WHERE name = ?1",
                params![name],
                |row| row.get(0),
            )?;
            self.conn.execute(
                "INSERT OR IGNORE INTO card_tag (card_id, tag_id) VALUES (?1, ?2)",
                params![card, id],
            )?;
        }
        Ok(Outcome::Ok)
    }

    /// The tags currently in use, each in its first-written casing.
    pub fn tag_names(&self) -> Result<Vec<String>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT name FROM tag ORDER BY name COLLATE NOCASE")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Cards carrying a tag, matched without regard to case.
    pub fn cards_with_tag(&self, tag: &str) -> Result<Vec<Id>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT ct.card_id FROM card_tag ct JOIN tag t ON t.id = ct.tag_id
             WHERE t.name = ?1 ORDER BY ct.card_id",
        )?;
        let rows = stmt.query_map(params![tag.trim()], |row| row.get::<_, Id>(0))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
}
