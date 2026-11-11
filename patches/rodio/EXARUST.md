# rodio 0.22.2, patched for Exarust

The crates.io release, unchanged except for `SampleRateConverter::next` in
`src/conversions/sample_rate.rs` and an `allow` for the unused-code warnings
Bevy's feature set leaves (`src/lib.rs`). Examples, benches and
dev-dependencies are dropped from the manifest.

The resampler reduces the ratio `from:to` and finds a chunk's next input
frame as `(from * position / to) % from`, in u32. Once `from * to` passes
u32 (a pitched voice, whose rate is the buffer's times its speed, against
the device's rate, the reduced ratio sharing no factor), the product wraps,
the target frame lands behind the current one, and the `while` loop that
walks to it steps through 2^32 frames: 80-125 s of the audio thread spent in
the source queue's silence (`SourcesQueueOutput::go_next`). Nothing is heard
in a grid, but Bevy drops the output stream with the frame's world, and
cpal's `Stream::drop` joins that thread: the game logged "frames played,
exiting" and stayed on the GPU for those 80-125 s while its simulation thread
kept stepping. Evidence: `context/artifacts/2026-10-30-exitstall/`
(`EXA_EXIT_TRACE=1`: the main thread in `cpal::host::alsa::Stream::drop`
joining, the `cpal_alsa_out` thread sampled three times inside the
resampler's `next_input_span` → `go_next`).

The patch computes both products in u64 and says once on stderr when a ratio
that would have wrapped is resampled.

Drop the patch (and its `[patch.crates-io]` line in the workspace
`Cargo.toml`) when rodio is upgraded past a release whose resampler does not
wrap.
