## 1. Changes

- [x] 1.1 `ModelLoader::pending_morphed_job` and `PendingJob` (the morph from the cache, the read and parse on any thread)
- [x] 1.2 `StreamWorker::begin_characters`, `pending_streamed`; test `streamed_characters_follow_the_requests_nearest_first`
- [x] 1.3 `ContentRequests::stream_characters`, `end_streamed_characters`, `take_streamed`, the pump keeping streamed bodies; the builders' `streamed_character`
- [x] 1.4 Viewer: `streamed_bodies`, posting in `finish_collision`, bodies added to the cache as they arrive, the list ended at the spawn
- [x] 1.5 `NativeLayout`, `format_channels`, `ChainOptions::format_channels`; tests `the_words_own_channels_filter_to_the_rgba_paths_bytes`, `a_base_off_its_words_layout_is_filtered_whole`, `a_grey_chain_has_the_originals_exact_levels` (checked by an independent reimplementation)
- [x] 1.6 `gaussian_kernel` through `normalise_kernel`; the `--frames` texture-wait log

## 2. Checks

- [x] 2.1 fmt, clippy `-D warnings` and tests of exa_render, exa_world, exa_assets, exa_game, exarust
- [x] 2.2 Lockstep traces: arena duel identical to main; exanima01 rows identical to main and to `--sync-content` over the shared ticks
- [x] 2.3 GPU grid before/after: pictures and FPS unchanged within noise
