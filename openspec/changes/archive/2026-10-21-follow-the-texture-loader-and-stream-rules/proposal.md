# Proposal

## Why

The first pass modelled the original's texture loader queue and content
stream scheduler but ran neither: every texture decoded on the main thread
at load time, and the loader's per-request state, the cancel and priority
paths, the player focus and the object fetch were approximations. A review
against the original's loader, upload pass, stream thread and main-thread
pump found several rules misstated (the loader's status bits, when a
decode may start, which load a finished decode releases, what an object
fetch searches, which object is the player), and the textures of run-time
materials, race skins and apparel skipped the sampler's bias and the
single-channel grey of a BC4 diffuse.

## What Changes

- The texture loader keeps one mark per request id; a finished decode
  clears only its own load's mark, a decoded texture is no priority item,
  and a release while decoding drops the upload.
- The frame pumps ready uploads before and after it starts the next
  decodes, so the loader's yield rule never idles it.
- Streaming runs in the game: decodes go to Bevy's task pool (four in
  flight, collected oldest first), a replaced placeholder keeps its handle,
  and materials and shadow cuts that held the placeholder are rebound.
- A single-channel BC4 diffuse is shown as grey on every material path,
  and every material path takes the texture bias.
- The unattended `--screenshot` waits (capped) for the loader to go idle.
- The content stream names the player by the arrival variable: an actor's
  operative kind (15 on arrival, 18 on a session start or load) or a
  marker record whose first word is the current arrival variable and
  whose second is all ones.
- An object fetch always searches the result ring; its default is the
  null node while object requests are queued, else the scene's nil node.

## Capabilities

### Modified Capabilities

- `content-streaming`: loader passes and per-request state, player focus,
  fetch defaults, streaming in the running game.
- `texture-decoding`: the sampler's bias product and the BC4 grey path.

## Impact

- `exa_assets`: `stream` (per-request marks, cancel, promote), `texture_stream`
  (plugin, pump order, rebinding), `texture` (bias order), `texture_cache`,
  `content_stream` (player focus, fetch), `model`, `human_skin`.
- `exa_render` (material bias and grey flags, shadow cut rebinding),
  `exa_world` (flame and terrain textures through the synchronous path),
  `exarust` (screenshot wait), docs `RENDERING.md`, `formats/RFI.md`,
  crate README of `exa_assets`.
- Evidence: reverse engineering of `Exanima.exe` (REA, Ghidra pseudo-code
  and assembly): the loader thread, its status bits and release path, the
  upload pass, the priority load, the stream thread's pass order, the
  nearest-character distance, the fetch default, the level builder's
  arrival mapping, the marker routine, and the sampler's bias product.
  The review confirmed each rule in the assembly, not only in the
  pseudo-code (the bias multiply order, the distance sum order, the
  fetch default).
- Out of scope: a Bevy driver for the content stream (its player focus and
  fetches stay a model, tested but not run), BC4 grey for emissive and
  unpacked specular maps, asynchronous terrain, flame and particle
  textures (they load synchronously), the loader's wake and sleep timing,
  and measured load and frame times on the retail corpus.

## Phase

Phase 2, static world (texture loading and level content), with the stream
scheduler for the level loader's later move off the main thread.
