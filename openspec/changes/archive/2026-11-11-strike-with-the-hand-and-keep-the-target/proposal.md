## Why

Phase 4 (Combat and items) in docs/PLAN.md, TMotilePart, the fists gap
lane (round 162). The round-154 rating left the two largest reachable
TMotilePart items open: fists still struck through Exarust's blade sweep
rather than through the part's hit event (the striking part's class slot
the pair stage calls on contact), and the attack starters' aim and the
parry check took the nearest fighter in combat as a stand-in for the
character's combat target.

## Evidence

REA (Ghidra and the objdump listing of `Exanima.exe` 0.9.5):

- The striking part's hit-event handler acts only when its own class word
  has the weapon bit and the strike-chain bit, its character differs from
  the struck body's, and the point faces the striking side (always for a
  character whose weapon class has no hand bits). It then classifies the
  struck body's class word: the held bit is a block (refused once the
  swing's shield bit is set), the weapon bit a clash, the living-limb bit
  a wound (refused while a hit waits, on an engaged arm and on the shield
  hand). The pending kind, struck body, point and striking part are
  stored for the post-step's damage routine.
- The unarmed set-up makes both hands the held items and gives them the
  weapon bit. The swing routine's window opening ORs the strike-chain bit
  into each striker's own class word (only the motion's hand with the
  fists, both hands for a clawed overhead): the striker is the hand part
  itself, not a chain. So a fist strikes by touch, and a fist is struck as
  a held weapon (a clash).
- The character's combat target is one field: the player's behaviour
  writes it every combat-mode tick from its auto-target pick (the visible
  character within 500 cm with the best (500 − d)(cos + 0.5) against the
  cursor), the AI's combat entry and target pass write it, and leaving
  combat clears it. The attack starters read it (a shield or two-weapon
  pick against the target's attack only when it is in condition 2), and
  so does the parry check (the threat is the target's attack).

## What Changes

- `exa_combat::block`: the striking-part classifier runs for a fist's own
  hand as for a held weapon body (`strike_events`, `side_contact`); a
  struck fist is a clash on that fist.
- `exa_combat::impact::sync_fist_striking`: the fist's hand part carries
  the strike-chain bit while its slot's window is open.
- `detect_hits`: no sweep for a fist carried by a part of its wielder's
  body.
- `exa_combat::CombatTarget` on each fighter, written by the character
  tick's attack stage (`exa_game` stages: the player's auto-target, the
  brain's pick, a brainless duellist's foe in combat via
  `exa_ai::Duellist::combat_target`); `parry::aim_foe_parries` and
  `parry::run_parries` read it.

## Out of scope

- The add-to-world pin and contact re-scale by the character size, the
  muscle setup's slack-bit gate on the smoothing, the living step's placed
  and frozen branches.
- The player's target is picked in the behaviour after the step's
  starters aim (one tick late for the aim) [inf: the original's behaviour
  order of the pick against the attack request was not traced].
- The character creator.
