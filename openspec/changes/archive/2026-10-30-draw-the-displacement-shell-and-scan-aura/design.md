# Design

Rules: `crates/exa_game/src/thaum/vanish_live.rs`, `crates/exa_game/src/effects/scan_draw.rs`, `crates/exa_render/src/force_effects.rs`, [`docs/RENDERING.md`](../../../docs/RENDERING.md) (Force effects) and [`docs/THAUMATURGY.md`](../../../docs/THAUMATURGY.md).

## Decisions

- **One live component a vanish.** `LiveVanish` holds the phase table (`effects::vanish::Vanish`) and the shell (`effects::shell::Shell`) on the cast's effect node, so the shell's end hook reaches the table directly. It starts on its first fixed step (the effects' clock), as a shell with a zero stamp takes the clock it is first stepped on.
- **Power phases as state writes.** Instead of threading messages out of the casters' tick, the step compares the power's phase with the one it saw last: entering the fade-out writes the shell's closing state, entering the fade-in reopens it from now and restarts the table one entry on (reveal), entering the tail closes it with leave to finish. These are the writes the original's power makes at the same transitions.
- **Copies off the source.** The shell and depth copies are children of the caster, not of the source meshes, so hiding the caster's own meshes (keeping each one's previous visibility) does not hide them. Parts their apparel covers are not copied.
- **Depth pass among the blended draws.** Drawn opaque it would punch holes in the scene behind (the opaque phase is not sorted); drawn in the blended phase with colour writes off, depth writes on and a small negative sort bias it runs just before the shell, after the scene, as the original's post-draw pass does.
- **Fade pass.** The body is shown while the fade entry's amount is above one half and hidden below (a guess for the 100–200 ms blend).
- **Node frame.** The body node's frame is the caster's rotation at its root part's place; the fade plane comes from the body parts' spheres' height span, as the force shell's.
