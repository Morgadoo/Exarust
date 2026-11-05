# gpu-allocator 0.28.0, patched for Exarust

The crates.io release, unchanged except for one function in
`src/vulkan/mod.rs`: `Allocator::upload_memorytype_index`, which picks the
memory type of an upload (`MemoryLocation::CpuToGpu`) allocation. Examples and
dev-dependencies are dropped from the manifest.

Upstream prefers host-visible device-local memory for uploads. Without a
resizable BAR that memory is the 256 MiB BAR window every process on the GPU
shares; whichever game allocated first got wgpu's staging blocks there and
wrote every texture and buffer upload over PCIe at 0.6-3 GB/s instead of
8-9 GB/s, which stalled that game's render thread (`prepare_assets<GpuImage>`,
`allocate_and_free_meshes`) when several games shared the GPU. The patch keeps
uploads out of a device-local heap smaller than 1 GiB; with a resizable BAR
(the heap is the whole of video memory) the upstream choice stands.

Drop the patch (and `[patch.crates-io]` in the workspace `Cargo.toml`) when
the dependency is upgraded and upstream makes the same choice.
