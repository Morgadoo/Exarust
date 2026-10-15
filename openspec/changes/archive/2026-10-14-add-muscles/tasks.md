## 1. Muscles

- [x] 1.1 `Drive` on `Ragdoll`: animated targets advanced per tick, PD pull per particle, tone and energy
- [x] 1.2 Carry the body with the root's motion; per-tick target steps taken the short way round
- [x] 1.3 Build living bodies (`Physical`) when posed; going limp removes the drive
- [x] 1.4 Set the targets after the clips are sampled and before the bodies step

## 2. Blows

- [x] 2.1 `Shove` message: velocity change on the struck part, daze on the body
- [x] 2.2 Blade hits shove with a share of the tip velocity; knock-downs keep the momentum

## 3. Playback

- [x] 3.1 Remove a clip's horizontal root travel

## 4. Checks

- [x] 4.1 Unit tests: following a moving and turning pose; a shove staggers and the body recovers; in-place roots
- [x] 4.2 Viewer: `--shove`; screenshots of the driven walk and of the sag after a blow
