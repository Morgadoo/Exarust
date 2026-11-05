# Proposal

## Why

When two to four games share the GPU in one `exarust grid`, one of them
(always the second to launch in round 63's runs) kept a render thread of
160-1600 ms where the others took 17-25 ms, from its first loading frame
to the end of the run. Its CPU went to `memmove` inside wgpu's
`write_texture`/`write_buffer` (`prepare_assets<GpuImage>`,
`allocate_and_free_meshes`, the cluster and GI buffers): every upload was
slow, not more uploads than usual.

The cause is where wgpu puts its upload (staging) buffers. Its allocator,
gpu-allocator, prefers host-visible device-local memory for uploads. On the
RTX 3090 without a resizable BAR that is the 256 MiB BAR1 window, which
every process on the GPU shares (the desktop, Firefox, other games). The
first games to start each take a 64 MiB block of it; when the window's
virtual space runs out the kernel logs `NVRM: dmaAllocMapping_GM107: can't
alloc VA space for mapping`, and the game that asked gets memory it writes
at about 0.5 GB/s instead of 8-11 GB/s. Every stalled run's stalled game
started within 0.2 s of that kernel message (15:13:05, 15:18:18, 15:32:29,
16:44:38), and a probe in the stalled game measured 0.5 GB/s for an 8 MiB
staging write. Even when the BAR mapping succeeds, writes there run at
about 5.5 GB/s against 8-11 GB/s in system memory, so the slot's game is
slower anyway.

The grid host also opened an audio stream it never used. When a frame
stalled, the stream broke and cpal 0.17 reported `alsa::poll() returned
POLLERR` in a tight loop: 0.9-7 GB of host log.

## What Changes

- Patch gpu-allocator 0.28.0 (`patches/gpu-allocator`, through
  `[patch.crates-io]`): an upload allocation takes host-visible device-local
  memory only when that heap is at least 1 GiB (a resizable BAR), and
  plain host-visible memory otherwise. Nothing drawn changes; only where the
  CPU writes the bytes it uploads.
- The grid host no longer adds Bevy's audio plugin, and filters
  `rodio::stream` errors from its log as the games already do.
- The frame profiler's report (`--perf-log`) gains two lines: the image,
  mesh and material assets added or changed since the last report (with the
  busiest image), and a staging-write probe (the best of four 8 MiB writes
  into a freshly mapped upload buffer, in MB/s).

## Out of scope

- A per-frame upload budget (`RenderAssetBytesPerFrame`): the uploads were
  not too many, they were slow; a budget would delay what is drawn.
- cpal's POLLERR loop itself (fixed upstream in cpal 0.18, which rodio 0.22
  does not take). A game's own stream still filters the log line.
- Phase: tooling and performance under docs/PLAN.md's running play
  milestones; no original behaviour is ported, so there is no REA evidence.
