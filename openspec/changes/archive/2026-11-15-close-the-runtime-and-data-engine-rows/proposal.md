## Why

Roadmap tasks B1 to B3 for the runtime, data and core sections of the
function ledger (lane `engrt`): wire the ported items whose caller Exarust
has, close what the engine replaces and what no 0.9.5 path reaches, and
leave the rest open with what is missing. The audits found the content
stream's queue reset unused, the free simulation loop sleeping where the
original yields, and several copies of the engine's maths helpers that
round, wrap or treat NaN apart from the original.

## What the original does

REA on Exanima.exe 0.9.5, checked in the assembly:

- The quit handler and the window procedure's close branch call the content
  stream's reset hook just before they set the quitting flag: every ring
  (jobs, completions, object, character, attachment and streamed results)
  is emptied and the streamed list cleared. Nothing else calls it.
- The engine's sleep yields the time slice for zero milliseconds
  (`Sleep(0)`) and waits on a relative waitable timer otherwise.
- The arctangent the light flood takes for a mesh's centre and for an
  opening's ends is the engine's fast octant arctangent; the half angle is
  the library's in double precision.
- The engine's angle helper compares the gap in double precision against
  the double π and steps once by the f32 full turn; the search sweep's own
  wraps compare in double too, and the parry pick turns the wrapped gap
  into its share in double (×1/π, +0.5).
- The engine's floor and ceiling helpers give the integer's smallest value
  for NaN or a value out of range.
- The engine's sine and cosine form both results by mask arithmetic, so the
  cosine is NaN where the unpicked polynomial is not finite.
- The squared distance of two 4-float points sums the squares in pairs,
  `(dx² + dz²) + (dy² + dw²)`.
- The grid reader converts the older plane layout when the header's third
  word is 600; the scene export writes 0 there, as every shipped grid holds.
- The project's file-name helper cuts at the last backslash only; the path
  resolver keeps a path whose first colon is past its first byte.

## What Changes

- `exa_game::content_requests`: a `Last` system resets and stops the stream
  when the run is asked to end (`AppExit`).
- `exa_physics::sim_app::free_wait`: a zero wait yields after the inbox is
  read instead of sleeping 200 µs.
- `exa_world::light_flood` takes `engine_math::atan2` and the double
  arctangent; `exa_ai::search::wrap` and `exa_combat::parry::wrap_pi`
  follow the engine's angle helper, and the parry share runs in double.
- `exa_ai::rooms`, `exa_ai::search`, `exa_world::rooms` and `exa_render::gi`
  take `engine_math::floor_to_i32` / `ceil_to_i32`.
- `exa_assets::polyhedra::fast_sin_cos` is `engine_math::sin_cos`;
  `exa_formats::factory::sin_cos` forms its results by the same masks.
- `exa_physics::contact` sums the eviction gap in the engine's pairs.
- `exa_rpk`: the resource name cuts at backslashes only; the drive test
  looks at the first colon.
- A shipped-data test pins the grid's older-layout word at 0.

## Impact

- Crates: `exa_game`, `exa_physics`, `exa_world`, `exa_ai`, `exa_combat`,
  `exa_render`, `exa_assets`, `exa_formats`, `exa_rpk`.
- Lockstep: the paired gap can change which contact point a near tie evicts,
  and the parry share's double rounding can move a threat on a height
  boundary; both are last-bit changes, rare in a trace. The other changes
  only touch NaN, out-of-range or boundary inputs, or timing.
