# Proposal

## Why

Phase 6 (AI) of docs/PLAN.md. The 47 statue golems of `exanima06` (race
word statue bit `0x08000000`) now carry the statue mark and their ally
sense sees nothing while it is set, but nothing ever cleared it: the golem
wake had no live caller, so they never woke. In the original the only live
caller is the alarm zone: the AI manager's frame checks the player's
trigger volumes, and stepping into an alarm zone without its pass item
wakes every statue golem within 800 cm of the zone.

Evidence (REA, Ghidra listing of `Exanima.exe`): the AI manager's frame
(the 3 cm step past the last checked point, then the point moves), its
zone check (the node radius, the box test at 0.1 in double for the current
and the checked point, the volume section's class `0x100` and low byte 2,
the player-only branch, the state-flag bit `0x20000` that ends the check,
the pass item at the section's word `0x10`), the inventory search (the
record's carried list and nested containers), the alarm trigger, its room
query and wake callback (the statue byte test, the contact list reset to
the player at concern 10), the golem wake, and the alarm reset of the
level starts. The ally sense's own call of the wake needs the statue byte
both set and clear, so it is dead. Corpus: a scan of every shipped level's
volume sections (only `exanima06` has alarm zones, 14 of them, flags
`0x1102`, passes 474–479, the key tokens A–F of `objdb.rdb`).

## What Changes

- `exa_formats::objdb::Volume::alarm_pass`: an alarm zone's pass item.
- `exa_world::alarm_volumes` and `LevelInfo::alarm_volumes`: the level's
  alarm zones as boxes in centimetres with their node radius and pass.
- `exa_ai::golem`: the zone check (`moved_enough`, `zone_entered`), the
  inventory search (`carries`), the alarm reset, the wake callback's
  contact reset; the words the wake clears and the query tests are the
  record's race word, not a node render word.
- `exa_ai::brain::Npc::guard_home`: the home the wake takes.
- `exa_game::golem_alarm`: the live zone check for the player and the
  alarm it rings (room query, wake, `golem_act`, contact reset, count),
  with the level start's reset.

## Out of scope

- The alarm return (awake golems sent home and asleep, the player put
  outside the `Alarm_` box) when the player falls.
- The zone check's other trigger kinds (the any-character routine of low
  byte `0x10`, the level transitions of bit `0x20`).
- The glow fade of the skin material `go`, the stance byte and posture
  word the wake clears, and the sleep scales (no guardian sleeps yet).
- What sets the state-flag bit `0x20000`.
