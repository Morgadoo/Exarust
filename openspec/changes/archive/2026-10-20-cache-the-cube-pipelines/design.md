# Design

## Behaviour

`PreparedCubes.specialized` maps `(AssetId<Mesh>, CubePipelineKey)` to `CachedRenderPipelineId`. `prepare_draws` looks the pair up; on a hit it reuses the id, and on a miss it calls the specializer once and stores the result. A failed specialization is not stored, so it retries when the mesh or shader becomes ready.

The cache persists across frames, keyed by the asset and the alpha-cut/two-sided key. A mesh removal leaves a stale entry, which is harmless because the mesh id is not reused for a different layout in practice; the cache can be cleared with the prepared draws if that changes.

## Model

A generic `cached_pipeline` helper performs the hit/miss policy and is unit-tested with a counter closure.

## Open questions

Whether the render app ever recreates `PipelineCache` while `PreparedCubes` survives is not established; if it does, the cache would need clearing on that event.

## Docs

No coverage change. The dev profile no longer trips the debug assertion; the `play` profile was already clean.
