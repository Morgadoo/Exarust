# Design

## Free body

`dying_tick` still calls `release_hands` once. Each loose weapon that has a solid shape is then spawned as one `Ragdoll::rigid` body. The shape's mobility is replaced by the loose weapon's own mobility before that build, so the part's inverse mass is the restored mobility. The constructor's friction is 0.75. `set_velocity` places the previous particles behind the current ones by that velocity, and `wake` raises the countdown to 256. The test then calls `Ragdoll::tick` on that spawned body. A missing or flat shape is left unembodied. A later dying tick does not spawn again.

## Open questions

- The loose weapon does not yet meet other bodies. It meets an empty level in the test, and the level when a caller passes one.
- The striking word is still cleared outright. The original flag word only loses bits 0, 16 and 17.

## Docs

The combat-rule figure stays 27.3%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
