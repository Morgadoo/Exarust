# Proposal

## Why

Phase 6 (AI) of docs/PLAN.md. A review found that nothing in the running
game delivered a blow to an AI combatant: the combatant base's hit rule,
the motile AI's hit reaction and its threat estimate were tested functions
with no caller. In the original, the damage routine calls the struck body's
behaviour once per contact, and for an AI that call lowers its courage,
marks the attacker's swing (whose practice takes the struck character's
rating as its lesson) and raises its concern for the attacker, writing the
attacker contact's flags word 3.

Evidence: REA (Ghidra listing) of the damage routine's three behaviour
calls (first in the clash branch with kind 1, first in the block branch
with kind 2, and on flesh with kind 0 after the damage call, the pain
voices and the "DamageTaken" script event, before the body's hit hook and
the player camera), the behaviour base's count, the combatant base's hit
rule (courage factor, the down/dead gate, the attacker's swing-word mark and
its practice lesson), the motile AI's override (base first, then the
contact loop: flags word 3, cap gate, threat estimate, concern) and the
constructor that sets the courage from the confidence trait.

## What Changes

- `exa_combat`: `BlowNoticed { target, attacker, kind, down }` written at
  the three points of the original's behaviour call; the striker's wound
  credit moves with it, before the player's blow feedback.
- `exa_ai::struck::react_to_blows` (PostUpdate, after the combat's frame
  systems): the combatant base (`combatant::take_hit`: courage on the
  brain's morale with its confidence, the attacker's swing mark, the lesson
  to a duellist's practice or through `Taught` to the player's), then
  `Combatant::react_to_hit` with the threat estimate over the brain's
  nearby list, computed only below the cap. One info line per reaction.
- `Combatant::react_to_hit` writes the contact's flags word 3 before the cap
  gate; the threat estimate takes an iterator (no allocation per hit).
- The duplicate courage, struck rating and mark fields on `Combatant` are
  removed: the courage is the brain's morale, the mark the striker's swing
  word, the lesson the practice's.
- `exa_game::player_practice::take_lessons` applies `Taught`.
- `exarust --opponent-brain`: the debug duellist also takes a level
  character's brain, so its hit reactions can be watched in a duel.

## Out of scope

- The base's call that ends the struck body's Spirit Form or Control Mind
  when its state has the mind bit: Exarust's mind powers keep no such bit on
  the body.
- A struck player's behaviour (the player's own courage and the AI
  attacker's lesson from the player's rating): the player publishes no
  combat rating.
- Duellists spawned without a brain (the arena's and the plain debug
  duellist) have no contacts, so only the base part runs for them.
