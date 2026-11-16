## Why

Roadmap task B4 with B1 (lane `aisearch`): the companion's search activity
(TAISearchActivity). Its ledger rows were open on the executors' details
(the shield a dead body grips, the carried take, the drop's checks), on
the re-arm's hands (the stow and the wield went straight into the hands
instead of through the inventory's routines), on the put-on's handling
sound, on the comment that was logged but never said, on the leader test
(a dead player counted), on the calm walk, the settle and the idle look
the tick asks the AI for, and on the place a container add gives a new
entry. Findings F277 and F378 name two of them.

## Evidence

REA (Exanima.exe 0.9.5), read in the instructions where constants, order or
branches matter:

- The worn take: a character's gripped shield whose item is the one taken
  is let go through the shield drop, the second hand cleared, the held
  carried entry removed and the shield's node taken out of the scene; else
  the worn list's entry goes, the underwear rule runs and the body refits.
- The carried take removes the last carried entry of the item that carries
  the held mark (an unmarked entry is never removed) and clears the fourth
  hand when it holds the item; the carried loot reader skips marked
  entries. So a looted carried item stays listed on the corpse.
- The drop job: a locked pack waits; the first carried entry of the item
  is required (none stops the job); the container add needs a tagged
  database id, the container section, the item's model byte within the
  container's largest item, and writes the new entry with no flags at the
  panel's width and height times 40; the carried entry then goes.
- The wear job needs its object present, runs the equip conflict pass,
  plays the piece's handling sound (its sound index names the foley class
  set; volume 0.3, pitch `0.8 + 0.2·draw` in double precision), puts it on,
  refits and runs the take.
- The re-arm marks the main weapon's carried entry, queues the wield job
  (the take into the hand, only for an item with a weapon section) and
  calls the planned arms with the next best (one-handed: a light stowed
  first moves beside it; two-handed: the second stowed item is cleared).
- The leader: the first contact with attention not below 50 (a NaN passes
  the compare), a character, not in the dead state, holding the Player role.
- The comment stores the phrase and activates the Explorer role's
  `ContextPhrase` topic toward role 30 (the Leader relation the recruitment
  gives) with mode 1 near the speaker.
- The tick: the path and the approach write the AI's walk urgency 0 before
  planning; the arrival calls the settle routine (look and heading from the
  body's heading, a new sway draw); the settle and loot states turn the
  body (the idle look) when no attention takes the tick.

## What changes

- `exa_ai::search`: `Corpse::shield`, `take_worn` with the shield,
  `remove_held_entry`, `take_carried` as the original, `drop_item` without
  the hand fallback and with the tag check, `Apparel::sound`, the wear's
  presence check, `find_leader`'s NaN compare; the wield only marks and the
  stow changes nothing in the model.
- `exa_game::npc_search`: the shield read from the body's `ShieldHold` and
  taken out of the world; `stow` (planned arms) and `wield` (the take into
  the hand) on the possessions; `HandlingSounded` and `SearchComment`
  handed to the frame; `bark_search_comments`; the leader from the
  contacts' bodies and roles; the calm walk, the settle and the idle look;
  `sync_container` places a new entry at the panel's corner.
- `exa_audio`: `HandlingSounded` played by `handling_sounds`.

## Out of scope

The perceived goal entity (the search still sets its look-at goal
directly) and the target pass's use of the activity's notice floor, which
needs it; the stop, reset and restart entry points (their callers are the
AI's removal hook and restart, not wired); the gathering's connected-cell
stand-in; the look routine's in-place normalisation of the body vector;
the drop and wield jobs' deferral to the main thread (Exarust carries them
out in the same step); the engine's shared random stream for the sound.
