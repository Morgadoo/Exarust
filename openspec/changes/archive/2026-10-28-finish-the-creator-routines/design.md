# Design

## Context

The rules stay plain functions in `exa_game::create` taking what the
original's routines read (record presence, whether a preset loaded, which
forms exist) and returning what they write.

## Decisions

- **New-character mark.** The mark is set by the record constructor the
  loaders call when the screen's record slot is empty, and the record
  loader does not overwrite it, so the plan marks exactly the paths that
  create a record. A named background without a preset adopts a template
  the caller owns, and an unlisted background gets the plain constructor,
  which does not mark.
- **Built panels.** The equipment page's forms persist once made, so the
  back switch takes a `BuiltPanels` set (weapons, skills, lobby, versus
  skills, versus equipment) rather than the panels one mode builds.
- **Underwear.** `exa_character` already owns the rule (race byte zero,
  no worn item covering the groin, item 8 appended), used by the arena's
  loadouts; the entry plan only flags that it runs.
- **Continue view loop.** The instructions re-read the list's first entry
  on every pass (no index register), so the model counts passes and the
  ones that refreshed the first item and the extra item.

## Risks

None for the running game: nothing here is wired, and the story start's
callers (`background_preset`, `creation_defaults`) are unchanged.
