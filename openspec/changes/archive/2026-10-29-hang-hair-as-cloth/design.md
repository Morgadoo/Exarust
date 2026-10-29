## Decisions

- **Two records, one set-up.** The constructor configures every cloth from its marker's record or the built-in one; every shipped marker is the bare tag, so the level's banners keep the numbers Exarust already had (`LEVEL`). The hair attach re-runs the set-up with its own record on each root whose controller is a cloth, then writes 0.1 into a cloth word (1.0 by default) whose reader was not found in the cloth, sector or render code; it is noted, not reproduced. The record's `0x7000` bits (a class flag), its `0x4` bit (the draw callbacks) and word 10 are kept in `flags` but read by nothing here.
- **The particle mode.** Read from the body-contact slot's switch on `flags & 0xF00 == 0x100` and its particle routine; the post-body pass returns at once in that mode, so no push buffer is applied and no passes follow. The box test hands the sphere test the squared padded radius, which squares it again; reproduced as is (it only loosens the cull).
- **The owner.** The set-up takes the controller of the node's parent when it carries the followable flag (`0x10000000`): for a hair root, the head's body. Exarust finds the nearest body part above the cloth's part (the head). The owner's spheres take no padding and its character is offered to the cloth (the triangle mode's skip of the cloth's own character stays a guess for that mode only).
- **Frames.** The original keeps a cloth's particles in the world; Exarust keeps them in the node's frame. For an owned cloth, `carry_into_frame` re-expresses the particles when the node moves, gravity is taken into the node's frame, and the bodies' spheres and tetrahedra are handed over in the node's frame (barycentric weights are affine invariant; the rounding differs). Unowned cloths are unchanged, so the level's banners behave as before.
- **The body's motion.** The original reads the body's tetrahedron now and a tick ago at the particle's weights (`exa_physics::engine_math::barycentric_fast`, `vec4_mul_4x3_difference`); the parts' `positions` and `previous` are those tetrahedra.

## Risks

- A living character's body countdown is 256, so its hair never sleeps (as in the original); the cost is a few hundred particles against a dozen parts a tick per long-haired character, plus the mesh upload.
- If Exarust's head spheres are larger than the scalp, the owner's sphere pushes the pinned scalp vertices out every tick and they snap back in the next step's pins (the original has the same order). The GPU check below would show it as flickering hair roots.

## Verification

- Unit tests: the records' words and pass split, the double-precision retention, the particle contact's push, floor, stick and slide, the body motion at a particle, a particle-mode cloth woken by a pushing body, the record handed through `ClothParameters`, the frame carry.
- Real data: `hair hula01` builds two particle-mode cloths with pinned and free vertices.
- GPU (for the integrator): `EXA_DEBUG_CLOTH=1 play exanima01 --perf-log 5` against the same run with `EXA_HAIR_CLOTH=0`: the stage's ms and frame time should stay close, long hair should hang and swing with the head, and `--trace stand.csv --frames 2200 play arena` should keep pelvis_height near 101 cm (the hair does not touch the body solver).
