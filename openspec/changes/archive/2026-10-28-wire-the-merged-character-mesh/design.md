# Design

- `dress::assemble` builds the runtime body tree from the built body and
  garment models with `dress::file_frame` (the inverse of
  `transform_from_rows`), runs the plan, and `spawn_merged_character` turns
  each step's built material groups back into file streams
  (`model::merge_dressed`) for `merge::merge`. `model::run_mesh` slices each
  run into a compact sub-mesh and rebuilds its tangents with the splitter.
- The merged vertices are in the character root's space, so each run is an
  entity under the root with an identity rest frame on joint 0. Joint `n`
  is still bone `n`, so the existing `bind_skins` binds it unchanged.
- Composites: the dressing pass decodes the job's maps on the main thread
  (cached per name) and spawns `SkinComposeInputs::generate` on the async
  compute pool. `finish_skin_composites` uploads the result and swaps the
  material of each waiting run that still draws its placeholder. The cache
  key leaves out the look seed, because it does not change the living
  skin's maps.
