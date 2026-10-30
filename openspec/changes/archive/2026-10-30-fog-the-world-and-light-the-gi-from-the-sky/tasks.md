# Tasks

- [x] Trace the fog uniform, the fog snippet and the record fog colour's reader with REA.
- [x] Port the fog into the world material (lit programs, hair, unlit opacity-only alpha).
- [x] Route the scratch colour and stored span to the fog; publish the record fog colour as the GI sky light.
- [x] Replace the GI sun with the sky light shaded from above; remove the sun shadow publisher.
- [x] Tests: the fog uniform's colour and span, the weather leaving it, the strike tinting the sky light with and without fog, the column floors and the sky light's threshold; the surface shader composes with the fog.
- [x] GPU captures against the pre-change build (interior, clear noon, storm with a forced strike); no corpus format changes.
- [x] docs/RENDERING.md (docs/PLAN.md left to the integrator).
- [x] fmt, clippy -D warnings and tests for `exa_render` and `exa_world` on the worker.
