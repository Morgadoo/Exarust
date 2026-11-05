# Design

Rules page: [`docs/THAUMATURGY.md`](../../../../docs/THAUMATURGY.md),
Mind in the running game.

## The walk

The original's list walk asks each entry's control with event 0 and
removes an entry that answers bit 2. Exarust's walk does the same over
`MindBody::powers`:

- The timed classes read the scene clock (Exarust's body clock in
  milliseconds, which the casts also stamp): Confusion and Enrage compare
  the unsigned 32-bit elapsed time with 15 000, Trust and Suspicion the
  signed 64-bit difference with 20 000; Sense Minds' instant entry reads
  the real clock its cast stamped.
- Confusion and Enrage clear their bit on the body that carries them. The
  casts write a target's bits on its mind; on a body with a power caster
  the walk ORs them into the caster's word first and copies the three
  bits back after.
- Loyalty's and Command Dead's casters answer 2 at their end after ending
  the target's marker whose caster they are (Command Dead then lays the
  body down: logged, the running game raises no corpse). Their markers'
  end ends the caster's entry whose target they are and re-arms the AI;
  Loyalty's also plays `ThmMnd_E01` at 0.15.

## The casters' state

The original's casts allocate the caster's power and never set its state
word (0, running); the ready state (1) is set only by a save's load, whose
first tick resolves the target id and links the marker. Exarust's classes
start ready, so the casts now set them running; otherwise the first tick
would link a second marker.

## The pick

The pick asks the player's AI's sight method, which the level installs as
its light-reach callback (`Rooms::light_reach`): the same room, or a door
link of one room into the other point's cell, through an open door, whose
door point lies within reach of the other point. Without a level the
method is the default that answers yes. The rules word's bit 0x10 is set
by the armour pass with the masked bit for a worn mind ward.

## The spirit screen

The screen is not a drawn quad: its frame hook steps the value, writes the
eased value into the post processor's desaturation, blends two
scene-environment values toward the spirit realm's, and uploads its noise
picture. Exarust drives `ExaPost::desaturate` from the step on the real
clock; the stop at the way home's black puts it back to 0.
