## Why

The character build hangs long hair as cloth, and the cloth system now runs live by default (face-flag pins, sleep, contacts), but Exarust still drew every hair style rigid on the head. The build gives each hair root whose controller is a cloth a built-in parameter record of its own, and that record switches the cloth into a constraint scheme Exarust did not have: the particle mode, whose body contact pushes particles straight off body spheres instead of pushing triangles.

Phase 3 (bodies) and Phase 5 (character surface) in docs/PLAN.md: SG_CharacterGen's hair attach and TVCCloth's set-up and particle mode.

## What Changes

- `exa_physics::cloth_record`: the cloth parameter record (`3DD0C001` block) the set-up copies: mode word, level slide, per-second retention (taken to one tick in double), mean drag, padding, gravity scale, pass count, rustle. The built-in record (`LEVEL`, what every shipped marker falls back to) and the hair's (`HAIR`).
- `exa_physics::cloth`: the step reads its record (drag, retention, gravity scale, passes in the step and after the body push, padding of the plane push); the level contact reads its padding and slide.
- `exa_physics::cloth_body::touch_particles` and `sector::ObjectContacts::gather_particle_bodies`: the particle-mode body contact.
- `exa_world::cloth`: `ClothParameters` on a part sets its cloth's record; a particle-mode cloth's owner (the nearest body part above it) keeps it awake by its countdown, pushes it without padding, and carries its particles in the world when it moves the node; gravity is taken into the node's frame.
- `exa_character::hair`: each cloth-marked node of a style becomes a cloth (`hang_as_cloth`), the roots with `HAIR`. `EXA_HAIR_CLOTH=0` turns it off.

## Evidence

REA assembly: the hair attach's root loop, the cloth constructor's record choice, the set-up's field copies, the integrate slot (gravity scale, retention, drag, the mode-gated pass loop, the owner countdown), the post-body pass's mode gate, the body-contact slot's mode switch and its particle routine, the level contact's padding and slide, the tetrahedron weight and point-motion helpers, and both records' bytes in the executable.
