## Why

Roadmap task B4 (lane `powers2`, the second pass over the triage's port
lanes L06a powers gameplay and L13 UI menus and arena screens, Q03 and Q04
of the 2026-11-18 queue). The open rows named a flattened knockback stand-in
for the force contacts, a torrent frame that looked along the raw aim
instead of the player behaviour's turn spring, a revived corpse that kept
its living voice, an ending dome whose hum was reset each frame, a summoned
demon whose body was never brought back or shown, and a hire message box
that took every pointer hit.

## Evidence

REA (Exanima.exe 0.9.5), the instructions read where constants, order or
branches matter:

- The blast's character contact (shared by the wave's class, whose class
  slots for the contacts are the blast's) clears the struck body's
  knockback kind, then calls the knockback step routine with the gathered
  impulse's direction and twice its capped size and no chest velocity. The
  routine zeroes the direction's height, scales it by twice the amount,
  cuts it to the step length and starts the step at the chest plus it. The
  torrent's contact calls the same routine without the clear.
- The torrent's start resets a turn spring (the player behaviour's cursor
  follower) at the aim; each tick the spring follows the aim's heading and
  steps, and the frame looks along its flat direction.
- Raise Dead's revival sets the record's feral level to 0.2 and picks the
  voice again from the record: the undead voice.
- The dome frame writes no base hum while ending and adds the glow to the
  hum voice's words each frame.
- The summon's control brings its demon's body back (the displacement
  powers' bring-back) when the shell starts to open and shows it (the
  settling ends) at the release, before the wake.
- The message box opener makes the box modal only with its 0x80 bit.
- The body tick's potion check and the power list's control walk, which
  the focus settle calls, are already ported (the regeneration's potion
  expiry and the casters' power walk).

## What changes

- `exa_physics::body::{knock_push, Ragdoll::knock_along,
  Ragdoll::clear_knockback_kind}`; `thaum::blast_live::knock_character` and
  `thaum::torrent_live::knock_torrent_character` knock through them.
- `thaum::torrent_live::turned_view` and `LiveTorrent::turn`.
- `mind::runtime::repick_voice` from `raise_corpse`.
- `effects::force_dome::Dome::hum_words`.
- `thaum::summon_live::release_demons` brings back and shows the demon.
- `hub_forms::MESSAGE_MODAL`: `HubForms::input` lets the pointer past a box
  without it.

## Impact

Characters struck by a blast, wave or torrent step back further (up to a
step length). A turning caster's torrent swings round after its aim. A
revived corpse groans with the undead voice. An ending dome's hum climbs. A
summoned demon settles for its fade-in. Lockstep traces change in scenes
where a force effect strikes a character or a torrent turns.
