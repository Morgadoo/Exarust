## 1. Shared lighting

- [x] 1.1 Lighting moved to the `exa_render::shading` library; models unchanged
- [x] 1.2 Light record: multiplier for the GI only, softness, spot values, range flag
- [x] 1.3 Vertical light shaping on surfaces and in the GI injection

## 2. Terrain

- [x] 2.1 Terrain material on the shared lighting with the original's terrain gloss
- [x] 2.2 GI volume and environment cube bound to terrain

## 3. Checks

- [x] 3.1 exanima01 start room rendered at the game's camera: the light pool falls on the terrain floor
