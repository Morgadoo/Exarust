# Proposal

## Why

Backlog item P1: the record's health helpers (`CharacterData::save_health`,
`restore_health`) had no caller. A story save wrote the player's record as
it stood at spawn, so a loaded save put the player back unhurt: its yellow
and red health, focus, the heal pool a bandage still owes and a drunk
potion were lost.

Evidence (REA, Ghidra listing of `Exanima.exe` 0.9.5, and the objdump
listing):
- The story save writer, before it writes, stores every scene
  character's body health into the record its body points at (the
  record's health store: yellow and red as shares of the maximum, red 0
  unless positive, focus, seed 0xFFFF, the heal-pool and potion entries,
  and the body's condition word into a live-record field the file does
  not hold). It is the store's only caller; the arena writer does not
  call it.
- The player spawn's last writes load the record's health onto the new
  body (the record's health load); the character spawn from a placement
  does too. The placement reader fills a living, non-dead record's yellow
  and red pools when its red pool is 0 before that spawn; the player
  spawn has no such rule.
- The value the store copies out of the body is its condition word (1
  normal, 2 combat, 0x200 down, 0xA00 dead), not an untraced value.

## What Changes

- `exa_character::record`: the copied body value is the condition word
  (`CharacterData::condition`, `Health::condition`, a `u32`); the load is
  also a free function over a record (`restore_record_health`).
- `exa_game::save_live`: `body_health` (a fighter, its state flags,
  condition and focus as the record's `Health`), `record_with_health`
  (the save's store) and `restore_body_health` (the spawn's load: pools,
  focus, heal pool, potion flag and expiry). Both story save paths (new
  slot and the flow's save and checkpoint steps) store the player body's
  health into the record they write, and log it.
- `exarust` viewer: a player spawned from a record (a loaded save's or
  the creator's) loads the record's health onto its fighter, caster focus
  and state flags, and logs it; `--health` still overrides.

## Impact

`exa_character`, `exa_game`, `exarust`; docs/formats/SAVES.md and RFC.md,
the `exa_character` README.
