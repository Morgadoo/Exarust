# Proposal

## Why

Phase 7 (Game modes and content, story act 1): the dialogue wiring of round
44 talks to Derrin, but four parts of the original were still missing. The
player carried Exarust's guessed id 1, which level record 1 shares, so the
cast and the Talk target had to guard against it; a story player who skipped
the creator had no name, so lines naming the player showed an empty name;
the gamepad could not move or pick a reply; and the reply panel drew at a
fixed interface alpha of 1, without the form's show and hide fade (the
round 42 dialogue gap).

## What Changes

- `exa_ai::brain::PLAYER_ID` is the original's new-character id: a story
  start empties both character databases and adds the new character to the
  world one under its first free id from 1, so the id is 1 with the world
  database's tag, `0x4000_0001`. A loaded save's player keeps its saved
  world number with the tag. The shared-id guards in the script cast, the
  Talk target and the bubble placement go.
- A story player who skipped the creator (`play exanima01`) starts from the
  creator's opening record for the unknown background: the background's
  preset saved in the user folder's `Exanima.set` (`--save-dir`), parsed as
  a character record, its race byte and powers cleared, its hands and lists
  dropped to the braies, full health. The creator itself now opens a story
  background on its saved preset as well. Without a preset there is no
  record (the original refuses to begin a nameless character).
- The gamepad's dialogue control: the pad reader turns the move stick into
  up and down steps while the reply panel is open; with the panel open and
  the power bar closed, Select picks the highlighted entry, else a step down
  moves the highlight forward, else a step up moves it back; the pad's Talk
  closes an open panel and otherwise talks in the pad mode, whose panel
  highlights its first entry on the open and on every layout of a non-empty
  list.
- The reply panel fades: the talk's show starts the base form's 256-tick
  fade on the GUI clock (system milliseconds), the close starts its hide
  fade, and the draw runs at the fade's alpha (the box at alpha × 160, the
  line skin's palette alphas scaled), keeping the rows through the hide.
- `--pad-press` (debug): a scripted gamepad for checks without one.

## Evidence

REA on `Exanima.exe` 0.9.5: the character databases' add (an id of 0 takes
the first free id from 1; the stored id ORs the database's tag), the story
start (both databases emptied, then the player added to the world one), the
creator's background loader and preset load (settings keys
`0x6431BC02 + background`, `0x6431CC01`; the race byte, known powers and
power slots cleared), the creator's refusal of a nameless character, the
player controls' pad routine (Select, step down, step up, Talk), the move
and confirm routines of the highlight, the talk routine's mode word and the
layout's first-entry highlight, the base form's show, hide and fade step
(256 ticks, mode 0 hands the alpha to the skin) and the panel's draw (box
at `round(alpha · 160)`, both skin scales reset after a fade), and the skin
hook that scales the palette's two alpha groups. Corpus: the user's
`Exanima.set` holds the unknown background's preset (a version-7 record
named "Tester").

## Out of scope

Level characters carrying the local database's tag (`0x8000_0000`) in their
ids; the unknown background's creation defaults (two starting powers) on the
opening record; input on the panel during its hide fade; the pad routine's
other actions (use, power bar, inventory) and its player-state gate.
