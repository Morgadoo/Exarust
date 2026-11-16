## Why

Roadmap task B4 (lane `equip`): the character equipment routines' open
rows. The body mark a golem's core drop clears and its take-off sets was not
modelled at the view, so a core crystal dropped on a golem's figure left the
golem coreless and the take-off neither marked it nor laid it down; the
character view's re-dress had no caller (the loadout form's close and the
trade system run it); and the equipment jobs' finalisers were modelled with
the wrong reading of the body finaliser (the player's light refreshed for
every body).

## Evidence

REA (Exanima.exe 0.9.5), read in the assembly:

- The coreless mark is the body rule bit the armour pass sets on a golem
  race and clears for a worn core; the golem wake skips a body with it, the
  rise and the red regeneration read it too.
- The view's core drop clears the mark on the world body it stands for; the
  release of the shown item takes it off the worn list, sets the mark, fades
  the body's skin material parameter `go` to 0 and, unless the body's brain
  rests as a statue, starts its collapse; then the view shows nothing.
- The loot window's equipment drag-off: when the body, without the item,
  passes the core test for it, the same mark, fade and collapse.
- The re-dress: the underwear rule, the apparel re-dress, then the record's
  other set swapped with its out set, the out set written with the hand
  models' items (a two-hander once), the hand-scene job built, the sets
  swapped back. The loadout form's close runs it on the roster and
  specialist pages for the player and every level body whose record word
  has a company role; the trade system for the player after a trade.
- The finalisers: a body's own runs effects cleared, apparel rebuilt,
  effects refreshed, the player's light switch only when the body is the
  player, then its view sync; a view's runs the same on its body, its
  callback, the sync of the world body it stands for, then the player's
  light switch for any body; the light one the apparel and the sync; the
  attachment one the attachment's release and the sync.
- The swap with the body's view open is refused while the view's job slot
  is held.

## What Changes

- `equip::core_shown` (the core drop) and `equip::core_taken_off` (the
  shown take-off: the mark, the logged glow fade, the collapse posted to the
  next step unless the body rests as a statue); `equip::core_left_equipment`
  for the loot window's equipment.
- The armour pass reruns only when the worn list or the out set changed
  (`equip::ArmourPassSeen`): a pack change runs none.
- `equip::Redress` and `equip::redress_bodies`, asked by the loadout form's
  close (`HubForms::closed_loadout`, `equip::redress_at_loadout_close`) and
  by a confirmed hub trade.
- `exa_items::doll::finish` answers each finaliser's ordered steps.
- `mind::runtime::lay_down` takes the caller's name for its log.

Not changed: the glow fade itself (no live skin material takes it); the
swap guard (Exarust holds no job across frames).
