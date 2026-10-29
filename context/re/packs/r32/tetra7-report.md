# tetra7: the seventh pass over TTetraRBD and RF_Dynamics_Volumes

Lane `tetra7` (branch `cl/tetra7`). Paths are relative to the worktree.
Physics lengths are metres in Exarust; laws the original states in
centimetres are worked in centimetres where this pass touched them.

This pass started from the remaining gaps that tetra2–tetra5 and their
reviews left open (docs/PLAN.md): the group link-mobility blend, the
breakable grab's removal with its parent's chain, the node-clock draw
fraction, plus the breakable grab's own inexactness found while reading it
against the assembly.

## (a) Every function in functions.md, mapped

### TTetraRBD (46)

| function (role) | bytes | Rust item / status |
|---|---:|---|
| weighted correction on four particles | 175 | already: `body::Part::push` (Exarust scales the vector first; metres make bit-exact sums moot) |
| restore one edge | 111 | already: `body::keep_rigid` (tetra4's `(x²+z²)+y²` and `(len−rest)/(len+len)`) |
| interpolate two body states | 118 | already: `Part::drawn` (a + (b − a)·t) |
| per-particle gravity | 113 | already: `body_rigid::gravity_shares`, the step |
| world-contact pass | 665 | already: `Part::hold_contacts`, `contact::rub` |
| constructor | 289 | already: `Part::new`, `Part::rigid`, `Ragdoll::new`/`rigid` |
| destructor | 366 | not applicable (Rust ownership) |
| clone/template link-array sharing | 96 | not done: no run-time clone shares template arrays in Exarust |
| deep clone | 1112 | already as derived `Clone`; sound-slot copy and parent-link transform have no counterpart |
| add a plain body (order key `round(100/m)`) | 67 | already: `pairs::Pairing::key` |
| add a world-linked body (sound sources) | 112 | already (tetra3/5): stored slots become scrape sources (`prop::BodySounds`) |
| AnsiString release | 26 | not applicable (RTL glue) |
| setup (gravity weights, edges, inverse, bounds, sound slots) | 559 | already: `Part::rigid`, `gravity_shares`, `Pairing::new`, slot registration |
| place at a node matrix | 296 | already: `Ragdoll::place_at` |
| place, edges, bounds, then slot 36 | 286 | partial: `place_at`, `Pairing::new` (controller hookup is ECS) |
| group-mask recursion entry | 37 | already: `pairs::mask_groups` |
| group masks over a tree | 256 | **completed**: masks in `pairs::mask_groups`; the link-mobility blend is now `pairs::blend_link_mobilities` (one pass per body with children, children's first; child = parent·0.5 + child·0.5 with the parent's pre-blend value), called by `Ragdoll::rigid` after the links' shares are taken |
| Verlet step | 1101 | already: `Ragdoll::integrate` and the frozen branch |
| VMT stub | 4 | not applicable |
| point by the body matrix | 33 | already: `Part::point`/`transform` |
| point by the previous matrix | 33 | already: `Part::point_at_start`, `slip` |
| two-step point transform | 78 | partial: `Part::point`/`slip` (its caller is the character code) |
| translate by an offset | 94 | already: `Ragdoll::push_weighted` |
| restore all six edges | 137 | already: `keep_rigid` |
| post-step | 1065 | already: `Ragdoll::post_step` plus the scrape/roll sources (tetra3/5) |
| VMT stub (rebuild ran) | 151 | not applicable |
| inverted-tetra rebuild | 712 | already: `Ragdoll::repair_inverted` |
| link iteration ×count | 66 | already: `Ragdoll::iterate_links`, `link_iterations` (tetra5) |
| recompute the link shares | 206 | **new**: `Ragdoll::recompute_link_shares` (enabled body links, `flags & (BODY|DISABLED) == BODY`, from `Part::link_mobility`). Character init's call is equivalent to `Ragdoll::new`'s shares, so it has no live caller |
| plain link iteration | 3213 | already for the pivot, slack, soft weld and world branches; **this pass**: the soft weld now splits by the two parts' link mobilities as they stand (`weld_shares` over `Part::link_mobility`, which carries the blend); the breakable grab follows the original (see below); the removal of every breakable grab to the broken part with its layers is `Ragdoll::release_grabs` + `Ragdoll::lower_grab_layer` |
| translate and rebuild scene matrices | 190 | already: `Ragdoll::translate` |
| VMT stub | 4 | not applicable |
| draw interpolation and matrices | 277 | already: `Part::drawn`, `draw_bodies` |
| create a link record at run time | 455 | partial: origin pivots in `Ragdoll::rigid`, hand drag `Ragdoll::grab`; a generic adder (slack stored squared, softness 0.001 when zero, shares from the link mobilities) is not built because a body's links are grouped per part (`links_of` ranges) |
| wake to a countdown | 307 | already: `Ragdoll::wake_to`/`touch` |
| countdown/sleep | 100 | already |
| VMT stub (impact callback) | 60 | not applicable (stub) |
| VMT stub | 7 | not applicable |
| impact sound and reaction | 450 | already (tetra3/5): `body::impact_sound`, `exa_audio::bodies` |
| VMT stub | 4 | not applicable |
| bounds sphere and box | 573 | already: `pairs::Pairing::new` |
| interpolate scene nodes between ticks | 572 | already: `draw_bodies` |
| point times body matrix helper | 146 | already: `Part::transform`, link points |
| a body's render fraction | 181 | **new**: `body::body_draw_fraction` (tick lag in 64 bits rounded to single plus the clock fraction, 0 while resting) — tested, no caller (drawing keeps Bevy's overstep) |
| node-clock render fraction | 169 | **new**: `body::clock_fraction` (three branches, unsigned compares, ordered −2 clamp) — tested, no caller |

### RF_Dynamics_Volumes (7) — unchanged this pass

| function | bytes | Rust item |
|---|---:|---|
| grid construction and near-surface fill | 2757 | already: `volume::fit`, `fill_near_surface` |
| solid fill by six rays | 1292 | already: `volume::fill_solid` |
| corner masks | 566 | already: `volume::mark_corners` |
| thin-mesh fallback | 2608 | already: `volume::fallback_spheres` |
| volume and centre of mass | 571 | already: `volume::summarise` |
| float quicksort | 208 | already: `volume::sort_by_key` |
| sphere extraction | 6604 | already: `volume::extract_spheres`, `corner_sphere` |

The weapon builder (`shape::build_weapon`, called live from
`exa_character`) runs the fit; the limb and root builders and the 443
object-library bodies no option set reproduces are still open.

### The breakable grab, as now reproduced (`body::pull_grab`, `GrabPull`)

Read from the assembly of the plain link iteration's 0x1000 branch and the
bite's link builder:

- gap d = struck point − weapon point, in cm; |d|² = (x² + y²) + z²; the
  entry test `|d|² ≥ ε` is ordered (NaN does nothing);
- a disabled struck body breaks it (argument `struck_disabled`);
- the re-anchor (0x200000) runs only after that test (the old port did it
  first);
- v = d dotted with the rows of the weapon's body matrix (`settled`),
  q = ((vx·ax + vy·ay) + vz·az) + 1; strain −= (q/|d|)·2 + 1 when q > 0
  (ordered), else 1;
- break when strain < 0 (ordered; a NaN strain pulls — the decompiler's
  `0 < s || s == 0` would break on NaN, the COMISS/JP does not);
- pull only when |d|² > ε (strict), f = strength/(|d|·0.001 + 1) in double,
  weapon += share·f·d, struck += share'·f·d with the share routine's
  m/(m + m') and −m'/(m + m') over the link mobilities (`grab_shares`; the
  old port used 0.5/0.5 and metres in q).

## (b) Ambiguities and guesses

1. **Multi-pass blend.** Registration runs the mask builder for every body
   that has children and no mask yet, after its children registered; the
   recursion itself never checks masks, so a body d levels down is blended
   d times. Read from the call order (child loop, then the mask call) and
   the builder; no shipped prop was checked at run time. Most shipped
   articulated props are one level deep, where this equals a single blend.
2. **Characters are unblended.** Character init sets every limb's link
   mobility afresh (0.175 × mass value) and recomputes the shares after the
   masks (it also gives the feet the union of their masks), so the blend is
   overwritten; this matches the research notes' unblended knee/hip shares
   and resolves tetra2's "evidence conflict".
3. **Disabled body state.** Exarust has no disabled (0x800) body state, so
   the struck body's disable is a parameter; `BiteHold` passes false.
4. **Weapon matrix.** The grab dots d with the weapon body's stored matrix;
   Exarust's `settled` is the Gram–Schmidt rotation of the last tick, not
   the original's affine matrix, so q differs where the tetrahedron is
   strained.
5. **Render fraction offset.** The body's render-fraction routine calls the
   clock routine without setting its offset argument (whatever the
   register holds); `clock_fraction` takes it as a parameter.
6. **Door link mobility.** The door body overrides its contact mobility
   after registration; its link mobility keeps the registered value. A door
   is a root with world links only, so nothing reads it.
7. **Units.** The body solver is in metres; the grab's strain law and pull
   are worked in cm, but pushes scale the metre gap, so sums are not
   bit-identical to the original's.

## (c) Commits

| commit | subject |
|---|---|
| `0f2ba02d` | Blend the props' link mobilities and follow the original's breakable grab |
| `fda5599a` | Keep the node clock's render fraction and a body's draw fraction |
| `cb30526f` | Archive the link-mobility and grab change and add the tetra7 report |

Checks before each commit (remote worker, real output): `cargo fmt --all
-- --check` exit 0; `cargo clippy -p exa_physics -p exa_world -p
exa_combat --all-targets -- -D warnings` exit 0; `cargo test -p
exa_physics -p exa_world -p exa_combat`: 810 + 539 + 290 passed, 0 failed
(5 ignored); `openspec validate --all --strict` 85/85 after archiving.
`exa_combat` was included because `bite::grab_of`/`BiteHold` moved to the
new grab API.

## (d) What is missing

- A generic run-time link adder (the slack-squared quirk, softness 0.001
  when zero) and the parent-node attach path that uses it.
- Clone sharing of template link arrays (96 B) and the clone's sound-slot
  copy and parent-link transform.
- A disabled body state, so the grab's struck-body disable can be live.
- Live bites: `BiteHold` and the grab are tested but no combat path opens a
  bite yet.
- Calling `clock_fraction`/`body_draw_fraction` from drawing (Bevy's
  overstep is the same fraction of the fixed tick).
- The two-step point transform (78 B) for the character code.
- RF_Dynamics_Volumes: the limb and root builders and the 443 unexplained
  object-library bodies.

## (e) Estimated share of bytes reproduced

- **TTetraRBD: about 68 %** (from 64 %). Added this pass: the blend half
  of the 256 B mask builder, the 206 B share recompute, the breakable
  grab's exact law and its removal pass within the 3,213 B iteration
  (≈ 450 B), and the 169 B + 181 B render-fraction routines as tested
  functions.
- **RF_Dynamics_Volumes: about 90 %**, unchanged.
