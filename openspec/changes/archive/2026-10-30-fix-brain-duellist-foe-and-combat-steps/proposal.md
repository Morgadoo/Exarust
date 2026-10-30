# Proposal

## Why

Phase 6 (AI) of docs/PLAN.md. A GPU run with `--opponent-brain` showed a
brain-bearing duellist that never started a fight with the player and, when
forced in, left combat at once; and in combat mode holding W did not move
the player.

Three causes were found:

- The brain's target pass compared the duellist's target, the player's
  root entity, with contacts the perception keys by the player's fighter
  (the root's child), so no contact ever matched the foe: no level
  character with a brain could enter combat with the player, and a forced
  entry was left on the next pass. The duel tick also kept a second contact
  for the root that the brain never perceives.
- In a story level the run's player holds the story roles (Player, Human,
  the unknown), which no arena role rates: the original never puts an arena
  fighter there, so that part was a debug-setup artefact. The original's
  practice start gives the player Unaligned (as its practice opponent holds),
  and a new career gives it the company's Faction 1 (as hub staff and
  teammates hold); Exarust gave every non-story player Unaligned, which the
  Faction 1 staff rate −100.
- Since the body walks by its carry, a muscled body's capsule takes no
  velocity, so the interim combat footwork's step velocity was dropped and
  the capsule stayed with the unmoving body.

Evidence: REA (Ghidra) of the arena setup routines: the round spawn and the
match templates register each opponent through the add-role routine with
role 0x12 (two sides) or 0x10 (free for all) and each company entrant with
0x11; the practice start clears the player's roles and adds 0x10; the new
career clears them and adds 0x11. The shipped `charroles.rdb` gives the
attitudes (installed-game test). GPU runs of the play build.

## What Changes

- `exa_ai::brain`: the perception and the AI tick resolve a duellist's
  target to the fighter at or under it (`foe_of`); a debug-level trace
  (`exa_ai::brain=debug`) logs how a brain rates its target once a second.
- `exa_ai` duel tick: the foe's contact, its observed entity, its roles and
  id and the combatant's target use that fighter.
- `exa_ai::roles`: Faction 1 and Faction 2, `Roles::company`, `Roles::only`.
- `exarust`: the player is Unaligned in practice (and outside any mode),
  Faction 1 in the arena, the new-character roles in the story;
  `--opponent-brain` documents where it engages.
- `exa_game::stages::movement`: a muscled body's capsule takes the interim
  footwork's step velocity while it steps.

## Out of scope

- The original's real foot steps for combat footwork (still Exarust's
  interim capsule step).
- The duellist's "estimate 0.0 cm from the foe's chest" entry line.
- A debug duellist that engages a story player: the original has none.
