## Why

Roadmap task B4 with B1 (docs/roadmap/phase-b-close.md), the `arenapart`
lane: the arena unit's open rows. The modes audits found company deaths in a
match dropped (no record, no count, the dead fighter kept), kept fighters not
reset between rounds, the day's special matches neither saved nor restored,
the day's log never cleared, a hired combatant's kit list empty, the hire
list kept over an empty saved list, no continue fixups after a load, the
week's prize and the season's rewards never reaching the arsenal, the
entrants' health rounded in single precision and the loadout points line not
following the dragged rank slider.

## Evidence

REA (Exanima.exe), checked against the assembly where order and constants
matter:

- The fall resolution: a fatal fall of id 0 is the permadeath branch; of an
  employee, the employee whose record id matches is logged (0x60000 with his
  name's sixteen bytes), the death count rises and the removal runs with his
  list index; no match logs nothing. The removal drops the order entry,
  releases the kit (0x400 kit, or 0x200 own items, hands and worn items),
  clears a merchant's stock and figure, removes the employee and drops the
  id (index plus one) from the entries and the log's saves. Record ids are
  not renumbered until the next numbering.
- Before a match: the first specialist (class above combatant) under 700
  health logs 0x60000 and counts a death, under 65000 logs 0xA0000; both run
  the removal.
- The kept-body reset: yellow takes red, the shield arm's stamina becomes
  1.0, condition 0, the motion word -1, clip 3 started, the repose at the
  place, the held items re-placed, and for a non-player the AI's clocks set
  to now + 10000 (the first plus the high half of the stepped roll times
  10000). The next standard round resets the kept opponent, then the
  continuing fighter.
- The day's end zeroes the log count after the wages and before the
  restocks; no step before it in that routine writes the log.
- The hire: a combatant's kit is gathered from his six loadouts (the first
  hand pair's set items, then worn items but item 8, each once) and stowed
  with 0x400; a specialist's own items with 0x200.
- The arena chunk load clears the hire list's count before the chunk is
  read; Manage Company draws a new list when it opens on fewer than one.
- The day chunk writes the six specials' id, rank and flags and a zero word;
  the reader finds each id among the seven ranked templates (the last when
  none matches), rolls it again at its index and puts back the rank (a
  rolled rank above 0 without flag 2), the window (not the Challenger) and
  the flags.
- The continue fixups after the arena load: the physician's weapon, the
  numbering, a tournament generated when its prize word is 0, the unfit
  dropped, the session hook.
- The week verdict stows the prize free of any holder when the company
  takes the week, and a swept season's four reward items after it.
- The entrant health set: red = yellow = single(double(max) times
  (double(word) times 1/65535)).
- The rank slider's unfinished arm writes the points line of the dragged
  rank (that rank's loadout cost over its limit).

## What Changes

- `arena_match::book_death`, `remove_dead`, `roster_name`; `ArenaMatch::died`;
  the match tick's and the sweep's deaths name the fighter and run the
  removal; `ArenaCareer::remove_employee`, `drop_employee`,
  `leave_before_match`, `staff_left`.
- `hub_forms::apply_staff_changes` takes the career: the won items, the
  joinings (`Staff::admit_stowing`, `staff::kit_of_record`) and the leavings.
- `match_layout::reset_kept`, `scene_bodies::SceneCommand::Reset` and
  `reset_kept_bodies`; the next round and the bracket round send it.
- End Day clears the log and queues a taken week's prize and a season's
  rewards (`ArenaCareer::won_items`).
- `arena_tables::apply` clears the hire list first; the hub's Manage Company
  click refills an empty list.
- `arena_tables::from_live` writes the live day board's specials;
  `arena_tables::restore_specials` rebuilds them.
- `arena_session::continue_career` runs after the arena slot load.
- `SceneBodyPlan::health_word`, `SceneBodyPlan::start_health`.
- `HubForms::dragged_rank` and the loadout kit's points line.

## Out of scope

- Re-placing a kept living body at its round place and its clip and held
  items (the body placement belongs to the motile dynamics).
- The board's Choose button (the event lineup in the loadout form's mode 3)
  and its close hook: the board model has no such control yet.
- The manager's own match copy, the wind wake at a match start, the
  apparel quality markers (no item instance layers), the shop's reward
  duplicate message, the arena save's id preparation and remap (Exarust's
  world tier keeps its ids), the recruit through the character creator.
