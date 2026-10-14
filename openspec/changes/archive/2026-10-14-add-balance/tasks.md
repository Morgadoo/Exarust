## 1. Body

- [x] 1.1 Drive support flag: carrying, gravity compensation and the pelvis drive only while standing
- [x] 1.2 Fallen bodies follow their pose relative to their own pelvis
- [x] 1.3 Centre of mass and its velocity; ticks run per frame

## 2. Balance

- [x] 2.1 Balance rule over the ticks the body ran; fall below 0.6 with the fall clip by direction
- [x] 2.2 `Fallen` stops player and AI steering
- [x] 2.3 Get up: choose the clip by lying orientation, place the controller, stand, raise balance

## 3. Checks

- [x] 3.1 Unit tests: steady bodies keep their balance; a dazed body pushed off its feet falls; support-line distance
- [x] 3.2 Play mode: a body blow topples the player, who gets up again (screenshots)
