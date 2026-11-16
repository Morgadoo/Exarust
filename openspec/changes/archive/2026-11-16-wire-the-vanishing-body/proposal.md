## Why

Roadmap task B4 with B1 (lane `dispfx`): the displacement powers' unit
routines. The ledger left the node-tree disable with its grab drop and the
enable with its settling uncalled (the latter two mislabelled as render
walks), the vanished bit never written on the running body, the Vanish and
Long Door after-image a flag that nothing drew, the body image's size
unscaled, and the Long Door's collapsing come-back unported.

## Evidence

REA (Exanima.exe 0.9.5), read in the assembly where order and constants
matter:

- Vanish's and the Long Door's control: at the charge's end the shell's
  state word becomes 2 with a scaled stamp, the body's state flags gain
  0x800 and the node-tree disable with the grab drop runs on the body's
  node (Vanish sets the bit first, the Long Door after); at the fade-out's
  end the tendril image is frozen and faded and forgotten; at the
  come-back the node-tree enable runs, the body is placed (the Long Door
  at its own ground point when its condition has 0x800), the shell opens,
  and the after-image callback is queued; at the fade-in's end the
  settling clear runs, the flags lose 0x800, and the image fades (or goes
  to the constant that lent the shell).
- The node-tree disable (sets 0x800 on every body down the node tree and,
  on a living body, drops the first link with 0x1000 and without 0x100000,
  lowering the grab layer on the link's other body and its own 0x40); the
  enable (a living body loses 0x900, gains 0x1000 with a 1000 countdown;
  any other loses 0x800); the clear (a living body loses 0x1000).
- The vanish start puts the image up after the shell when the class is
  not the AI's; the image routine scales its size word (12) by the body's
  size.
- Readers of the vanished bit: the instant Displacement gate, the AI's
  Long Door advice, the AI combat tick (drops a vanished target), the
  player tick (skips its footprint stamp).

## What changes

- `exa_game::thaum::vanish_body` (new): the turns' message and system.
- `exa_game::thaum::power::displacement`: `ImageStep` in each vanish step;
  the image slot an entity.
- `exa_game::thaum::cast`: the status word's bit, the turns, the Long
  Door's collapsing come-back, the cast's first image, the body image's
  size.
- `exa_combat`: `StateFlags::VANISHED`, `BiteHold::drop_first_grab`.
- `exa_physics`: `Ragdoll::set_kinematic`.
- `exa_ai`: the duellist drops a vanished target.
- `exa_game::footprints`: a vanished player's footprint stays.

## Out of scope

The Summon's disable, enable and clear on its demon (a kinematic demon
without the held body's sleep word would meet no floor); the image over a
shell lent by a Constant Displacement; the scaled stamp (`Clock::scale_since`),
whose base against the frame clock is not established; the after-image's
deferral to the next frame (Exarust makes it in the same step).
