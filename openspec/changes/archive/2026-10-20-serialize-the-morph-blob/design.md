# Design

## Behaviour

The blob is the magic (4 bytes), the size (4), the base vector (12), `V` rest vectors (12 each), the part count (4), then for each part its pivot (12) and `V` deltas (12 each). The total is `12·(P+1)·(V+1)+12`.

Parsing reads the magic and size and rejects a wrong magic or a size that is not the input length. The vertex count is the owner's, so the part count is `(size − 12) / (12·(V+1)) − 1`, which must divide exactly. The stored part count must equal it. The cursor must end exactly at the end of the bytes.

## Model

`MorphData::to_bytes()` returns the blob; `MorphData::from_bytes(bytes, verts)` returns the parsed data or `None`.

## Open questions

The three evaluator routines that read the blob are not in.

## Docs

`RF_Morphing` is **46%** (7,866 of 17,099 bytes). The headline stays 14.4% and 5.7%.
