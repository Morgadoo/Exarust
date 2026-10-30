# Proposal

## Why

Phase 7 (Game modes and content, story act 1): round 40 wired the dialogue
keys, but nothing filled the Talk key's focus, so it could not start a
conversation; the reply panel and the speech bubbles had draw data but were
never painted; the panel took no pointer input; and the first level had
nobody who talks, as the story start's companion was never placed.

## What Changes

- `exa_game::dialogue_live`: on the frame the Talk key is pressed, the talk
  routine's target query (the player's nearby characters, not collapsing
  or dead, the chest nearest the cursor on screen through the bubble
  projection) fills `DialogueFocus`.
- The reply panel is made on the first talk (400 × 120, centred, 152 pixels
  above the bottom), stays shown while the engine's panel is open (an
  emptied list leaves the box), holds the engine's pick for a second, and
  takes the pointer: its bands hover, a released left click picks.
- After the open screens, the bubbles (placed over the body position lifted
  120 cm, or 75 % down for the narrator) and then the panel (rounded box of
  radius 2 at alpha 160, "N." and wrapped text in the label or highlight
  ink) are painted in the main GUI font.
- `exa_game::script_cast` makes the player body's character id the session's
  player, and keeps a level character sharing that id out of the cast.
- The viewer places the unknown background's companion, Derrin
  (`xac_derrin`), at the story start's companion point in `exanima01`.

## Evidence

REA on `Exanima.exe` 0.9.5: the Talk key's talk routine (target query,
panel creation and placement, open), the target query's loop (behaviour
nearby list, condition bit 0x800 skip, chest point through the view and
viewport projection, squared distance to the integer cursor, start value
`0x7f7fc99e`, ordered compare), the panel's constructor (400, 120), hit
test, input (click → engine pick) and draw (eased height, rounded fill
radius 2 at `round(ui_alpha · 160)`, palette inks at the label and
highlight groups, main font, rows), the engine's pick (hold of 1000 ms),
close (hide) and step (list and pick emptied, form kept), and the bubble
placement (body position + 120). Corpus: the role database's dialogues
(only the Derrin roles, Parted and the golems carry any; XA_Derrin_Meet
aims at XA_Unknown, which a new unknown-background character holds) and
`actors.rpk`'s `xac_derrin` (Human, Activity Wait, XA_Derrin_Meet).

## Out of scope

The form system's own mouse-leave (the hover clears off the panel, a
guess), the interface-wide alpha (1 in play), gamepad highlight input,
the original's new-character id (Exarust's player id 1 is shared by level
record 1; the cast and the focus guard against it), the player's name
without a created record, and the rest of the story start (its light,
pose and wake).
