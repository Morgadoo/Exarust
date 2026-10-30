# Proposal

## Why

Wiring backlog items G1 and G4. The story scripts' `GameEvent` handler was
ported, but the flow gave it a fixed player without the memory role on a
background other than the unknown one, so only the background unlock acted
and every other effect was logged. The story experience ran its tick from
the flow's discovery pass, but its level-start hook and tick looked for the
player's record on the player's root entity (it lives on the body), a story
player without a save had no record at all, and nothing read the technique
word the conversion produces.

Phase: Phase 6 (story mode: the story's scripted events and experience) of
docs/PLAN.md; no PLAN.md edit in this change.

Evidence (REA, Ghidra on `Exanima.exe`, checked against the assembly): the
story event handler of the game-main unit and every routine it calls (the
body's voice request, the post-process flash setter and its draw, the
narrator topic request, the power grant, the witness event, the mind and
force awakening effects, the sound start), the record reader and writer
(the class word beside the experience, the powers chunk's first word read
into the potential float), the story start (it clears the fragment word),
the session words the story save writes and reads (start background,
memory flags), the body's voice fields, and the experience unit's hook
registration and kill/pickup call sites.

## What Changes

- `exa_game::game_event`: the handler's effects are named for what the
  original calls: a voice request (line 2, volume and pitch factors) instead
  of a "motion", narrator topics with a delay instead of a duration, a
  fragment word set whole by an awakening; the raise-dead line takes the
  result of the power grant for Raise Dead; helpers read and write the
  record's potential and class word.
- `exa_game::flow`: the story state keeps the start background and the
  fragments seen; a new story clears the fragments; the hook applies the
  fragment bits itself.
- `exa_game::game_event_live` (new): reads the live player (roles, record,
  voice), runs the grant, applies the action request, voice line, narrator
  topic, record writes and sounds in the handler's order; `--game-event`
  debug aid.
- `exa_game::save_live`: a story save writes the start background and the
  memory flags; a load restores them into the flow.
- `exa_game::experience_live`: the hooks find the record on the player's
  body; the level start puts the technique word on the body; a changed word
  reaches the body's caster and fighter.
- `exarust`: a story player without a save carries its record.
- `exa_audio::PointSounded`: voice flags.
- `exa_formats::characters`, docs/formats/RFC.md: the class word and the
  potential named.

## Out of scope

- The post-process flash's draw, the witness event broadcast, and the mind
  and force awakenings' body effects (logged).
- The experience's item-kind and kill hooks: their call sites take a world
  object flagged as not yet discovered, not a picked-up item or a dying
  character as the port assumed; they need their own trace.
- The explored-cell grid and the story start's record writes (G2, the
  story-start lane).
