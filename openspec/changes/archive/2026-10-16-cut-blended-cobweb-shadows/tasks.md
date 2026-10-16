## 1. Shadow cut

- [x] 1.1 `ExaMaterial::cut_shadow` and `cuts_shadow()`. The loader sets `cut_shadow`: an alpha-tested blend word, or the opacity flag (record flag `0x400000` or a loaded opacity map) and an opacity above 0.75
- [x] 1.2 Cube pass: the cut texture is the opacity map, else the diffuse map. Texels with alpha below ½ are dropped, without the material's opacity or squaring. Both sides are drawn, with or without a cut texture
- [x] 1.3 Unit tests: a blended cobweb record cuts, as do the flag alone, a loaded opacity map alone and an alpha-tested blend word; opacity 0.75 and neither flag nor map do not; the choice of cut texture

## 2. Undrawn faces

- [x] 2.1 `PreparedCubes` exists from the renderer's start. Faces that could not be drawn are handed back as failed and stay dirty

## 3. Verification

- [x] 3.1 Corpus (`shipped_cut_shadow_records`, ignored, returns early without an installation): the menu's `cobweb01` record is blend word `0x0203`, flags `0x400017`, opacity 1, opacity layer `CobWb01O`; `harhps1` has the opacity flag and no opacity layer
- [x] 3.2 Menu at the original's camera, with and without `--no-shadows`, against the original's capture: torch-lit walls and floor within about 4 of 255 sRGB levels
- [x] 3.3 exanima01: a torch placed in a wall holder's slot links its `TorchSdw` disc and the holder, not its own handle. The wall shows no shadow of the handle
- [x] 3.4 Cube cache: no face rendered in a still frame (menu and exanima01), and frame time well below a cache-less run in exanima01
- [x] 3.5 Remove probes and dumps from the tracked tree

## 4. Documentation

- [x] 4.1 `docs/RENDERING.md`: Lights (casters), Cached cube shadows (both sides, undrawn faces), Voxel GI (menu check)
- [x] 4.2 `openspec validate --all --strict`
- [x] 4.3 fmt, clippy `-D warnings`, tests of the touched crates
