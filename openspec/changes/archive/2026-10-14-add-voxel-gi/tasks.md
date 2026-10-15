## 1. Inputs

- [x] 1.1 GI sample points parsed from meshes; counted by `corpus meshes`
- [x] 1.2 Occupancy voxeliser (sector-aligned block, lid, rounded trilinear splat, edge drop); unit tests
- [x] 1.3 Floor layer from tile-grid floors; sector floors; unit tests

## 2. Volume

- [x] 2.1 Volume textures and the propagation compute pass with injected lights
- [x] 2.2 Warm-up steps kept until the pipeline is ready
- [x] 2.3 Propagation limited to each sector's floor and up; empty sectors left dark

## 3. Surfaces

- [x] 3.1 Surface ambient and reflections from the volume; cube path outside it
- [x] 3.2 Viewer switches `--no-gi` and `--gi-warm-up`

## 4. Checks

- [x] 4.1 Menu scene rendered with the GI and compared with the original
