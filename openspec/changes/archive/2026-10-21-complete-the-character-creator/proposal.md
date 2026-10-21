# Proposal

## Why

The character screen shared by story, arena recruiting and versus was only
partly reproduced: Exarust had its record helpers but not the story
presets, the entry flow, the two-page creator, the starting kit, the
experience budget and worth, the voice choice or the continue chooser. A
review of the first implementation against the disassembly then found
rules that differed from the original (preset keys, the gear a named
background wears, the recruit filter, voice names and pitch rounding, the
skills page's experience, the kit tables, the continue toggle and others).

## What Changes

- The record the screen edits: the mode's gear filter, the story
  background presets and their settings keys, the template a named
  background wears, the free creator's preset, the entry plan per screen
  mode, full health and the story record's cleared experience.
- The starting skills, the slot clear, the experience budget with its
  training-skill leftover, the experience worth and the skills page's
  global and new global experience.
- The starting kit's seven tables by mode, the match against what the
  record holds and the kit item resolution.
- The appearance page's sex, voice prefix, voice and variant selection,
  sample names, pitch, preview throttle and non-repeating variant draw,
  hair checkbox and face variant captions.
- The two pages' captions, equipment panels per mode, the name warning,
  the power tree for a record with thaumaturgy powers, the commits per
  mode and the back action.
- The continue screen's save and checkpoint toggle, stepping, list pick,
  deletion and layout; the creator icon sheet.
- Corrections to earlier requirements: the slot clear reaches the sixth
  slot, the first known power is `0x101`, the continue toggle is enabled
  below the availability mask, and the preview throttle and link debounce
  compare in 64 bits.

## Capabilities

### New Capabilities

### Modified Capabilities

- `character-creation`: the screen flow, presets, kit, skills, voices and
  continue chooser, and the corrected record move, slot clear, defaults,
  continue button, preview throttle and link copy.

## Impact

- `exa_game::create` (rules), `exa_ui::creation` (labels and captions),
  `exa_audio::Lcg` re-exported for the variant draw.
- `docs/formats/OTHER.md` (Character creation), `docs/UI.md`,
  `crates/exa_game/README.md`, `crates/exa_ui/README.md`,
  `crates/exa_character/README.md`.
- Evidence: REA (Ghidra 12.1.4) on the 90 unit routines of the character
  screen (17,196 bytes) and their callees: the record and settings
  helpers of the character data unit, the experience unit's two
  converters, the audio environment's play routine, the object database
  lookups and the save manager. The pack's assembly was read wherever
  the pseudo-code hides an unsigned or 64-bit compare, a float
  conversion or a call's argument count. No corpus files are added; the
  kit tables' shape and the preset keys were checked against the
  installation without copying game bytes.
- Out of scope: drawing the form and its controls, the name text box, the
  model and dummy-head preview, item preview materials and icons, loading
  and playing the voice samples, the save manager and preset
  serialization, the item database the kit consults, the power tree, the
  reset-to-portal and name-warning dialogs, the continue list's model
  loading, Steam input, and wiring any of this into the running game.

## Phase

Phase 7, game modes and content: character creation (class, skills,
starting kit, naming), the screen that starts the story, arena and versus
modes.
