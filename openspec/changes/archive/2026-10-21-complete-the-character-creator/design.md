# Design

## Context

The rules are plain functions in `exa_game::create` that take the state
the original's routines read and return what they write, so each can be
pinned by a test with concrete numbers. The form, the item database, the
save manager and the sound system stay outside: where a routine asks them
something, the answer is a parameter (`preset_loaded`, `record_of`,
`base_of`, `created`, `sample_exists`, `availability`). Screen captions live
in `exa_ui::creation` so the game rules and the UI name the same strings.

Format page extended: `docs/formats/OTHER.md`, Character creation.

## Decisions

- **Record loading.** Every preset is a two-key settings path whose second
  key is shared; only the first follows the background (the arena one
  below the unknown background). A named background reads its template by
  name first, so its gear is the template's whether or not a preset was
  saved. Recruiting builds a new record and skips the gear filter.
- **Starting skills** write only the three slots' points and first five
  techniques, because the original writes those bytes and nothing else;
  the free creator repeats this whenever it builds the equipment page.
- **Experience.** The skills page converts the account's experience to
  worth, takes off the worth stored by the last commit, and converts back.
  The arena commit adds that stored worth again. The training skill that
  takes the budget's leftover is the experience unit's rule.
- **Powers.** The count the final button tests is the record's list of
  known thaumaturgy powers; a record with any opens the power tree.
- **Integer widths.** The preview throttle, the debounces and the preview
  hover compare zero-extended words in 64 bits; the worth sums in 32 bits;
  the pitch is worked in doubles; voice and list steps wrap.

## Open reverse-engineering questions

- The meaning of the `0x60` state the hair checkbox gets for a male record
  with a face variant: answered so far only as "holds the checked bit the
  sex radio reads"; a capture of the running creator would show it.
- The word the arena commit subtracts (`spent_before`) is written outside
  this unit; REA on its writers will name it.
- Mode 2's alternative kit flag is named `intro` after the tables it
  selects; REA on its writers will confirm what it marks.
- The physique sliders' ranges and the randomise buttons are in the
  creation form and character generator, not these routines; they are
  answered with REA on those components.
