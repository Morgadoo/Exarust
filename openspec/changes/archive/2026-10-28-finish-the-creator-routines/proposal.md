# Proposal

## Why

Phase 7 (Game modes and content, character creation) lists the character
screen's unit routines at 65%. The charcreate2 pass modelled the rest of
the unit: the preview's icon list, material pass and dummy head, the panel
binding, the power tree callbacks, the continue page's character view, the
dialogs, the icon sheet, the creator form's opening and the full-health
seed. Its review against the disassembly found the record's new-character
mark, the equipment page's back switch, the preview loops and the leave
routine read wrong, and a duplicate of the existing underwear rule.

## What Changes

- Entry: every record the screen makes is marked new (recruiting always;
  free creator and continue when the screen had none; story only for a
  fresh record that reads a saved preset or starts the unknown
  background). The entry writes four `0xffff` halves: the three pools and
  the save seed. The underwear pass follows the filter and is the existing
  `CharacterData::ensure_underwear`.
- Back switch: only the first built of weapons, skills and lobby is
  hidden (the skills slide `0x2d` only when skills is that one), then the
  versus skills and equipment forms when present; a non-continued
  character's appearance panels are shown again with both tab words set
  to `0x110`. Leaving goes to the arena for a recruit, else to the menu.
- Preview and continue view: the material loop covers the list's highest
  index plus one items; the icon sheet's size is eight GUI units; the
  continue view refreshes its first listed item only when its record
  carries flag `0x100`; the sex radio opens audio only when not ready.
- New modelled routines: preview icon list, material, dummy head, panel
  binding, character refresh, the power tree's open/close/deferred close,
  the name warning box (code `0xc1`), the reset-to-portal and delete
  dialogs, the continue view and page, the save steps, the icon sheet and
  the creator form's opening.

## Evidence

REA and `context/re/exe.asm` on the character screen unit's routines and
their callees (the record constructor and default-record init, the
underwear routine, the record loader). No game bytes are copied.

## Out of scope

Painting the form, model and sound loading, the save manager, preset
serializer and object database, the power tree's own screen behind the
callbacks, and wiring: the game has no creator screen yet, so these stay
tested functions (the story start keeps using `background_preset` and
`creation_defaults`).
