# Decisions

Decisions that shape the [roadmap](README.md). An open decision carries a
recommendation and what it blocks. When the owner answers, move it to
"Taken" with the date.

## Open

### DEC-1. What 100% means
**Question:** is the project complete when every function is in a final
ledger state (live, engine-replaced, unreachable, not-applicable, approved
deviation), and the scenario matrix passes against retail?

**Recommendation:** yes. Without closing states for engine-replaced and
unreachable code, no amount of porting reaches 100%: the count includes GL
plumbing, threads and dead code.

**Blocks:** A6 (the published number), B2, B3.

### DEC-2. Steam
**Question:** link the Steamworks library, or close the Steam code as a
kept deviation?

**Facts:** Steam API glue is rated 7%, Steam Input 75% (the action sets
are reproduced on gilrs; Steam is not linked). The retail game ships
`steam_api64.dll`. Linking it ties the build to Steam's licence terms and
to a running Steam client.

**Recommendation:** keep it as a deviation for now: gamepads work without
Steam, and Steam adds achievements and the overlay only. Revisit at
packaging (task D5 of Phase D).

**Blocks:** part of B4 (platform), D3 of Phase D.

### DEC-3. Character creator
**Question:** do SG_CharacterGen, SG_Character_Create and TCharCreation
stay with the owner's other session?

**Facts:** about 9 KB of the estimated gap. This session does not touch
those files.

**Recommendation:** audit them in Phase A like everything else (read
only), and leave the ports with the other session.

**Blocks:** B6.

**Note (2026-11-12):** the slot-kind fix (round 203) left three creator-side items for the other session: `create.rs` `SaveKind` names are swapped (numbers correct, not wired); the `continue_label` comment; the character-creation spec's continue-button wording.

**Note (2026-11-15):** the arena New Career skips the company name and emblem dialog (F446). The menu's New Career is answered in the creator screen, so the fix (open the hub's company form first, and let its NEXT open the creator) is left to the other session.

### DEC-4. Start and budget of Phase A
**Question:** start the audits, and at what scale?

**Facts:** 90 audit lanes and 90 reviews; an estimated 30 to 45 million
tokens and two to three working days at eight lanes at a time. The three
pilot packages (A3) cost about 1 million tokens and correct this estimate.

**Recommendation:** approve the pilot now; decide the rest on its numbers.

**Blocks:** A3 onward.

### DEC-5. Kept deviations
**Question:** confirm each difference kept on purpose.

| item | why it was kept |
|---|---|
| Supersample jitter pattern | a rotated grid, kept by an earlier decision for picture quality |
| Path planner's shared stamp table | reproducing it would break determinism |
| Acceleration trees (kd-trees, block trees) | Rapier colliders give the same answers |
| Screen-saver refusal, work-area fit | no meaning on Linux |
| Quad-buffer stereo | not reachable on current hardware |

**Recommendation:** approve all five; each becomes a `deviation` row with
this entry as its reference.

**Blocks:** B5.

### DEC-6. Composite terrain draw
**Question:** make the original's composite terrain draw the default?

**Facts:** it is ported and opt-in. No retail outdoor reference exists to
confirm the picture.

**Recommendation:** decide after C2 captures outdoor references.

**Blocks:** part of B4 (world), two terrain components' last gaps.

### DEC-7. Approximate float instructions
**Question:** reproduce the original's approximate reciprocal and
reciprocal-square-root instructions (`rcpss`, `rcpps`, `rsqrtss`), or keep
Exarust's exact results as a deviation?

**Facts:** the physics audits (physics-05, physics-06) found the original
uses these in the slab test, weight normalisations, the sweep set-up and
other helpers, where Exarust computes exactly. These instructions are only
specified to about 12 bits, and Intel and AMD processors return different
values, so the original itself does not behave the same on every machine.
Exarust already has the approximations (`engine_math::reciprocal_sqrt`,
`reciprocal_approx3`) but does not use them there. The same audits found
related float details: constants in double where Exarust uses f32, a few
summation orders, and NaN branch choices; those are exact behaviour and
should be matched.

**Recommendation:** match the double constants, the summation orders and
the NaN branches (they are deterministic). For the approximate instructions,
use one fixed software approximation equal to a common Intel result, so
runs repeat on every machine; record it as a deviation if the owner prefers
exact maths.

**Blocks:** part of B4 (physics); about a dozen `partial` rows.

### DEC-8. Shader templates versus embedded WESL
**Question:** the original builds its GL programs from a shader template
library (the `.flb` file, a template expander with conditions and loops, a
program cache keyed by feature words, a 29-variant warm-up). Exarust draws
with its own embedded WESL shaders; the template expander, the library
reader and the cache are ported but only the tool and tests call them. Close
these as engine-replaced, or drive the game's shaders from the templates?

**Recommendation:** engine-replaced, on one condition: a lane shows that
every program variant the original builds for 0.9.5's data (every feature
word that occurs) has a WESL path that produces the same result, and lists
any that do not as `partial`. Driving wgpu from GL templates would add a
translation layer for no visible gain.

**Taken 2026-11-15** as recommended, under the owner's "complete the plan"
goal; the lane `wireshader` checks the condition.

### DEC-9. Calm's write past the contacts
**Question:** the original's Calm cast loops over a character's contacts up
to the count inclusive, so it writes one slot past the live contacts.
Reproduce that write, or keep Exarust's loop over the live contacts only?

**Facts:** found by the `wiremind` lane (round 211). The extra slot is
inside the contact list's storage, so the original does not crash; what it
changes depends on what the slot held (a stale contact, or nothing).

**Recommendation:** reproduce it only if the slot can hold a stale contact
that later code reads (a lane checks the list's growth and readers);
otherwise record it as a deviation.

**Blocks:** one `partial` row (Calm).

### DEC-10. Desktop platform differences
**Question:** keep these Windows behaviours of the original as deviations,
or reproduce them?

| item | the original | Exarust |
|---|---|---|
| Single-instance guard | refuses a second copy of the game | several copies run (the check grid and the lockstep checks run four at once) |
| Window icon | loaded from the executable | on GNOME Wayland the icon comes from the desktop entry; winit cannot set one |
| Adaptive vsync | the setting asks the driver for adaptive vsync | wgpu offers no adaptive mode |

**Facts:** found by the `engrt` lane (round 220). None changes the game's
picture or rules.

**Recommendation:** keep all three as deviations: the single-instance guard
as an opt-in flag for release builds (task D5), the icon through the desktop
entry at packaging, adaptive vsync as the nearest wgpu present mode.

**Blocks:** six `missing` rows of RF_OS_Functions and RF_Main.

### DEC-11. The tendrils' reads past their data
**Question:** reproduce two reads of the tendril effects that run past the
data they mean, or keep Exarust's stand-ins as deviations?

| item | the original | Exarust |
|---|---|---|
| Wisp strip past a lit last point | when a strand's oldest ring node is lit, the strip is one point longer than the strand: that point and its attributes come from the strip's scratch buffer as an earlier strand or frame left it (the world origin before any), and at ten points its copy lands on the smoothed strip's first slot | the last point stands in for the missing one |
| Arc respawn with no high sphere | the respawn draws source spheres until one stands more than 10 cm up in its body's frame; with none it never returns | the respawn starts nothing |

**Facts:** found by the `tendrils` lane (2026-11-16). The first is reachable
whenever a body's wisps move fast enough that a ring node is still lit when
it becomes the oldest; what it draws depends on draw order (a faint strip
end toward another strand's tail, or toward the origin). The second needs a
shock weapon whose spheres all sit at or below 10 cm; no shipped weapon's
spheres were dumped to check.

**Recommendation:** keep both as deviations: the first reproduces a stale
buffer whose content depends on draw order, the second a hang.

**Blocks:** one `partial` row (the wisps' strip draw).

## Taken

| date | decision |
|---|---|
| 2026-11-15 | DEC-8 taken as recommended (shader templates engine-replaced where the WESL paths cover every variant). |
| 2026-11-12 | DEC-1 to DEC-4 taken as recommended when the owner set the goal "complete the plan": the ledger definition of 100%; Steam kept as a deviation for now; the character creator audited read-only, ports left to the other session; Phase A starts with the three-package pilot, the rest sized from its numbers. |
| 2026-11-11 | Lockstep traces may change when a change makes timing follow the simulation as the original does (round 201); each change is explained in the round's notes. |
| 2026-11-11 | The bar's casts draw from the simulation's random streams (round 197), as the original shares them; one sweep scene's outcome changed. To be confirmed by B0.4. |
| 2026-11-11 | Project paused after round 202 at the owner's request. |
