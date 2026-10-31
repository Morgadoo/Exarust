# Design

**Ids.** `PLAYER_ID` stays a constant: the original's first free id in the
emptied world database is always 1, so the new character's id is
`0x4000_0001`. `first_free_id` keeps the database's rule (walk the ascending
ids, step past each one met, stop at the first above) for callers that add
more characters. A loaded save's player takes `world_character_id` of its
record's saved number. Level characters keep their record ids; the
original's local tag on them is out of scope, so a relation table could
still meet a level character and the player under the same low 28 bits
(each brain keeps its own table, and none holds both today).

**Opening record.** `create::load_preset` reads the chunk at the two keys
from `SettingsStore` and parses it with `CharacterRecord::parse`, then
applies the preset load's resets. `creator_screen::entry_record` takes the
preset alongside the template and follows `create::plan_background`: a saved
preset becomes the record (the unknown background's gear reset, a named
background's template gear moved in), else the new record or the template.
`story_opening_record` is that record for the unknown background, used by
the viewer when a story level starts with no committed record and no save.
The user folder comes from `--save-dir` (`SaveFolder`), the only place
Exarust knows it.

**Highlight.** The original's highlight is the panel form's hover itself,
which the pointer also sets, so the pad's steps and Select act on
`DialogueWindow::view.hover`; the engine's own `Panel::highlight` stays as
the engine-side rule but nothing mirrors it any more. The pad mode lives in
the engine (`Panel::by_pad`, set by `talk_with` on success and cleared by a
narrator open) because the layout that reads it is driven by the engine's
list. A pad panel's new list is not re-hit-tested against a pointer that
did not move, so the layout's first-entry highlight stands.

**Fade.** `DialogueWindow` holds the base form's `FormFade` and its visible
flag; `show`, `hide` and `fade_alpha` are the form's show, hide and fade
step. The game drives them from the engine's open flag on the real-time
clock and paints with `line_skin::shown_palette(alpha)`, the skin hook's two
scaled groups.

**Open questions, answered.** Which clock the fade reads: the system
milliseconds the frame clock is derived from (REA, its writers). Which skin
the inks come from: the active skin, whose hook scales the palette (REA, the
global alpha setter). The fade mode of the panel: 0, as nothing writes it
(REA, every access of the field is a read).

Formats: `docs/formats/SETTINGS.md` (settings store keys).
