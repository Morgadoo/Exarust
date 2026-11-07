## Context

The original keeps an explicit target per character; Exarust's player has
no target component, and the parry check already takes the nearest
attacking foe.

## Decisions

- The foe whose parry the starters aim at is the nearest other fighter in
  condition 2 (Exarust's stand-in for the target), measured chest to chest,
  ties by entity; its parry is kept on `Attacker::foe_parry` each fixed step
  in `FixedPreUpdate`, which the player's attack request runs after, so the
  value is the state the step starts from.
- Two weapons are told by the stance class 6 the held moveset gives exactly
  with class word `0x90` (the AI's own weapon class does not carry `0x90`).
- The countered-motion write is queued as a command of the starting system.
- The starters' end of an ongoing parry (clearing the parry flag, the grip
  links and the hands' bit when the start is not aimed) is not ported:
  Exarust's parry release is keyed on the flag at the check's start.
- The AI's crouch is its body's crouch; the chain passes no crouch (as
  before).
