# Design

## Behaviour

The hit is false when the key is 8 or the list at `0x230` is absent. Otherwise it is the hit test's result. A hit calls the selection with the list's count at `0x158` less one. The method returns true in every case.

The singleton at `0x1004f0450` is returned when present. When absent, `0x100130290` runs, the previous hook at `0x10049a9d0` is saved and the creator is installed as the new hook.

## Model

`hit_creation_option(key, has_list, hits)` returns the hit, the selection and the true result. `ensure_creation_singleton(exists)` returns whether the creator ran and whether the singleton is ready.

## Open questions

The hit, selection and creator bodies are not in.

## Docs

These methods are 80 and 76 bytes. `SG_Character_Create` moves from 25% to about 26%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
