# Design

- **Benchmark workload.** The fragment program is WGSL for Bevy's `Material2d`, bound at group 2 (the unused projection uniform, the texture, the sampler).
  - wgpu has no RGB8 format, so the target and the source are RGBA8 (linear, not sRGB). The program reads only `.rgb`.
  - The original sets its depth function to always pass. Bevy's 2D opaque pass tests greater-or-equal, so the quads share one depth: every quad passes and is shaded. Staggered depths would let early depth testing reject all but the nearest quad and inflate the score.
  - A small system after the scene's creation shows four quads during the warm-up and sixteen during the runs. It only runs while a benchmark exists.
- **Detection trigger.** The routine that runs the detection or the shipped-key copy has no direct caller and was already reproduced as the Defaults button. The tab handlers call only the page switch, which places the pages and tabs. The tab switch therefore runs nothing, which keeps the saved key copy when the form opens on the Controls page.
- **Controller view.** The view is the Controller tab's lit bits, which only the Steam Input path sets. Without Steam Input the page switch forces the keyboard view. `controller_view()` is `steam_input && controller`.
- **Probes.** Deck detection and video memory read the environment and sysfs once, when the form opens. The probes are cheap and total, and each falls back to "not a Deck" or "Unknown".
- **Help box.** The box is a read-only markup `TextBox` with the page's font and a one-unit inset each side (a labelled guess).
