## Why

In `exarust --opponent play arena` the duellist swings every 2.2 s for a
minute and never lands a blow; in `play arenao1` its blows land and kill
the player. The duellist spawns behind the player in the practice arena:
the opponent's stand-in start there, written when the player started
facing +Z, is 2.5 m along +Z, and the change that gave the practice arena
the original's start (facing −Z) left it there. The player's target is
picked within 120° of the cursor's direction, so with the duellist straight
behind it the player has no target: it keeps its guard and its back to the
duellist, which closes to its striking distance (1.15 m between the
characters, Exarust's stand-in for the original's reach) and cuts at the
player's back.

From behind, the blade falls short. Light runs (RTX 4000, with a temporary
probe of the blade's nearest gap to the target's part spheres, the ones the
hit test uses): the duellist strikes from 1.08 m (chests 1.07–1.11 m apart),
and at its closest, at clip progress 0.71–0.76 with the tip at 8.5–9.8 m/s,
inside the attack window, the blade passes 15–21 cm short of the back of the
head, the right shoulder and the upper arm. Face to face, from the same
1.08 m (chests 0.97–1.06 m apart, the head and the guard leaning toward the
duellist), it passes 8 cm into the head and every strike lands, 22–30 cm
along the blade. The duellist placed behind the player in `arenao1`
(`--at 0,0,-4.5`) misses there too; the player turned to face it in `arena`
(`--cursor 480,470`) takes six blows of six on the head and is down after
15 s. So the level is not the cause; the duellist's place is.

Phase 4 (Combat and items), milestone M4 "first duel".

## What Changes

- `exa_world::start`: where a duellist stands, its arena ring slot, else
  2.5 m in front of the player's start, facing it (Exarust's choice: the
  original fixes no opponent's start in the practice arenas; its duels start
  face to face across the ring). `--at` moves the player and the duellist
  with it.
- Play mode spawns the duellist there, its body turned to face the player
  before the fight starts (the ring slots' facing is unchanged).
- Docs: the side-view example in `docs/DEBUGGING.md` frames the duel where
  it now stands.

Evidence: the starts in `docs/formats/RFC.md` (Starts: the practice
arenas' player start, the arena ring); the light runs above.

Out of scope:
- a duellist behind a player who turns its back still falls short: its
  striking distance is measured between the characters' roots, whatever way
  the target faces. The original's AI fights at its reach, turns its body
  through the swing and steps in with its footwork; that replaces the stand-in
  (C16, C21, L11);
- the original's practice mode and its sparring partner's start.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `player-movement`: in a level without an arena ring the opponent stands in
  front of the player, facing it.

## Impact

- `crates/exa_world` (`start.rs`: `duel_opponent` and its test)
- `crates/exarust` (`viewer.rs`: the duellist's place and facing)
- `docs/DEBUGGING.md`
