## 1. Find
- [x] 1.1 Trace the hand's pick (ray, replica, stand-ins, filters) and the dropped item's colliders
- [x] 1.2 REA: the cursor pick and the put-back's contact clear

## 2. Fix
- [x] 2.1 Pick prop bodies with no stand-in by their part meshes; leave held ones out
- [x] 2.2 Clear the released item's world contacts; several scripted GUI drops
- [x] 2.3 Tests

## 3. Check
- [x] 3.1 GPU: a dropped bascinet lifted and carried (main refuses); a cudgel stacked on another lifted, the sleeper woken; story level and duel traces unchanged
- [x] 3.2 fmt, clippy -D warnings, tests, play build, openspec --strict
