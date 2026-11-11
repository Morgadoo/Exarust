## Why

Phase 2 (data and loading) and the rendering phase in docs/PLAN.md. Two component gaps: FLX_Content_Stream's streamed
level characters had no driver (the thread loop's nearest-first pass, the character-chunk handoff and the pump's
insert were reproduced but nothing posted a level's characters), and RFP_Mipmaps_Kaiser filtered every image as four
channels where the original's chain driver filters the channels of the image's pixel word.

## What the original does

REA and `exe.asm`:

- **Character-chunk handoff.** When the streaming hook is installed the scene importer hands the whole characters
  chunk to the content stream. The handoff reads the count, makes a stage node per entry whose record exists (a
  conditional entry only when its locale matches the session), keeps the entry's transform, class and saved
  sub-chunks on it, and resets the streamed cursor. The level load does not wait for these characters.
- **Thread loop, last step.** When a pass ran no job, object or character request, it picks the pending streamed
  character nearest the focus point (the placed player's position), swaps it to the cursor, builds it (the saved
  chunks applied, the size words scaled) and pushes it to the streamed result ring.
- **Pump.** Each main-thread pump drains that ring, frees each stage node and inserts the built character into the
  scene and its sector, so characters appear as they are built.
- **Mip chain driver.** The driver masks the word to its target (`0x2008`/`0x4200` one float a texel, `0x6208`/`0x8400`
  two, `0xC600` three, `0xE608` four), converts the stored pixels to that many floats, runs the two axis passes whose
  span helpers accumulate `w·s` per tap in order into a zeroed destination (`mulss`/`addss`, or `mulps`/`addps` for two
  to four floats), renormalises three- and four-channel levels when the image asks, and converts back.

## What changes

- The running game hands a level's character bodies to the stream thread while the collision builds: each body model
  morphed by its record's physique, once per variant, read, parsed, morphed and meshed on the thread, nearest the
  player's start first; the pump keeps them and the play loader adds each to the model cache as it arrives. The level
  spawn then finds them built (exanima01: 35 bodies; the spawn frame's character build 136-264 ms to 1 ms). The spawn
  does not wait; what is not built yet is dropped from the list and built by the spawn as before, so spawn order and
  simulation are unchanged. [Exarust: the spawn stays one frame after the collision rather than characters appearing
  one by one, so lockstep traces keep their rows.]
- The mip chain filters the word's channels out of the RGBA8 base when the base is laid out as the word's decode makes
  it (grey, alpha, grey-alpha, RGB), and spreads them back; otherwise, or for a renormalised image, all four.
- `--frames` logs how many frames it skipped for the texture loader (they run the simulation, so a trace's length
  depends on them).
- The terrain's Gaussian kernel uses the unit's kernel normaliser, as the original's terrain init calls it.
