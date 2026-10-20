# Proposal

## Why

Phase 6's NPC brain rates a contact, idles and fights, but nothing gives a
level's characters what they are: the role database, their roles and
relations and the attitude that follows were not read, the idle activity was
never chosen from the roles, and the companion's search (idle activity 6)
did not exist. The AI manager's frame rules and its approach clamp were also
missing.

## What Changes

- Read `charroles.rdb`: every role's head, init script, variables and
  entries with their sections; find a level's locale in `locales.rdb`.
- Hold roles as the original does: adding with inherited parents (newest
  first, counted), removing explicit roles and counting parents down, the
  new story character (Player, Human, background) and the arena fighter
  (Unaligned).
- Keep relations (global and per locale), relation roles with values, the
  familiarity, the last attitude and the move of a former level character.
- Work out the attitude of A toward B (relation roles first, then the first
  matching entry of A's roles, blended by the role weight), the encounter
  scripts that match, and the entry finder for dialogues and conditions.
- Apply roles: run the init scripts' assignments newest role first, the
  mind-control follow, the idle state per activity, the idle-activity
  registry (the search as 6), the wait, and the home-side point.
- The search activity: gathering over the 600 cm room grid, picking,
  the look sweep, the forced path, arrival, kneeling, looting worn items,
  packs and containers, weighing apparel and weapons, comments to the
  leader, re-arming and tidying.
- The AI manager's rules: dormant ticks, rising undead, the 3 cm zone
  check, zone deeds, the group alert and the approach clamp.
- Wire it: the level's living characters carry their roles, relations and
  a brain that applies the roles, rates the player from both sides' roles
  every frame, wakes a sleeper, and (human frame) duels the player once
  that rating turns hostile; the player carries the new-character or arena
  roles.

## Capabilities

### New Capabilities

- `npc-roles`: the role database, roles, relations, attitude, encounters.
- `npc-search`: the companion's search activity.
- `npc-manager`: the AI manager's frame, zones, alert and approach clamp.

### Modified Capabilities

- `npc-idle`: the activity chosen from the roles, the wait and the
  home-side point.

## Impact

- `exa_formats::roles`, `exa_ai::{roles, activity, search, manager, brain,
  idle}`, `exarust` (spawning placed characters and the player).
- `docs/formats/OTHER.md` (Roles), `crates/exa_ai/README.md`,
  `crates/exa_formats/README.md` if it lists parsers.
- Evidence: REA/Ghidra reading of the original's role database routines,
  apply-roles, the search activity and its item routines, the AI manager;
  a parse of the shipped `charroles.rdb` (49 roles, each ending at its
  size) and `locales.rdb` (17 locales).
- Out of scope: a script engine (only the init assignments run; encounter
  and dialogue scripts are returned, not run), the mind-power attitude
  overrides, the perception's group blend, walking the roam and patrol in
  the running game (no navigation grid there yet), and feeding the search
  real containers, corpses and inventories (no item instances in the
  running game yet).

## Phase

Phase 6, the NPC brain: roles, relations, activity selection and the
companion's search.
