## 1. Reverse engineering

- [x] 1.1 REA: the texture format table the upload reads, the format lookup for data-flagged words, and the `glTexImage2D` binding
- [x] 1.2 REA: the packed-storage unpacker for 3- and 4-byte pixels
- [x] 1.3 Data: the embers' material record and voxel light

## 2. Decoder

- [x] 2.1 `exa_formats::rfi`: RGB8 and RGBA8 decode in their stored order, red first, raw and packed
- [x] 2.2 Unit tests on synthetic bytes: raw and packed RGB8 and RGBA8, an RGB8 normal map, the RGB8 format words
- [x] 2.3 `exa-tool corpus images`: every image by format word and storage; uncompressed images decoded; RGB8 normal maps checked for z in blue

## 3. Verification

- [x] 3.1 Corpus (`exa-tool corpus images`, needs the installation): every image tallied by format word and storage, all 86 uncompressed 3- and 4-channel images decoded, the 42 RGB8 normal maps hold z in blue (all 42 fail with the old order); `exa-tool png` of `embersael`, `pwrcrstl01e`, `ironold02n`, `oaklfb01d`, `fireprt03`, `cerpt03d`, `thmnrgico1`, `maintitle01` and `langetl a4n` gives the stored order
- [x] 3.2 Screenshots through `gpu_run.sh`, before and after: `arenao3` at the fire pit with the roasting pig, `arenao5` at its fire pit and inside an oak
- [x] 3.3 Remove probes and dumps from the tracked tree

## 4. Documentation

- [x] 4.1 `docs/formats/RFI.md`: channel order and its evidence; format table uses
- [x] 4.2 `docs/RENDERING.md`, `docs/PLAN.md`, `docs/research/online-research.md`, `crates/exa_tool/README.md`
- [x] 4.3 `openspec validate --all --strict`
- [x] 4.4 `make check`
