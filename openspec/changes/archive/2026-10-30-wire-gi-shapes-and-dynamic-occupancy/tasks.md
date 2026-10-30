# Tasks

## 1. Shapes

- [x] 1.1 Port the six `shading.flb` shape bodies and the `GiShape` registry tracking
- [x] 1.2 Put the draw box's half extent and the texture windows in the original's order
- [x] 1.3 Fix the cuboid constructor's frame

## 2. Dynamic occupancy

- [x] 2.1 Wire `update_dynamic` with the sub-upload
- [x] 2.2 Drain the patch list every frame and write patches to the bound and warming blocks
- [x] 2.3 Gate on a build in flight and catch up blocks built after a move
- [x] 2.4 End the upload region one voxel short of the size

## 3. Shading quality

- [x] 3.1 Feed `ShadingQuality` into `GiSettings::high_quality`
- [x] 3.2 Take the high-quality share in double precision

## 4. Checks

- [x] 4.1 Tests and clippy for exa_render, exa_world and exa_game
- [x] 4.2 Docs and READMEs
