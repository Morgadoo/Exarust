## Context

The original rebuilds the player body from the player's record at every
match start, so a spectated match simply builds none. Exarust's player
survives level changes; the bench reproduces the absence by despawning it
and reusing the save load's respawn path (`SavedPlayer`) at the hub.

## Decisions

- The bench runs after the level change's world sync promoted the local
  items, and promotes again before the record is taken, so the record's
  item ids are world ids that survive the next level's database.
- The fighters' duel target is Exarust's stand-in for the brain's target
  pass (which rates every other side hostile): the nearest living fighter of
  another side, written through `FixedCommands`.
- The session ends when a player stands again and no arena scene runs (the
  hub's Character mode start).

## References

docs/re/playerctl-map.md (spectate entry), context/re/arena-layout.md.
