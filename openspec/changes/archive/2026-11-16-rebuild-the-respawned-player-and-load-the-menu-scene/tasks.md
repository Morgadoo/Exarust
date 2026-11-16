## 1. Respawn rebuild

- [x] 1.1 Read the CONTINUE's request, the identity reset and the respawn in the assembly
- [x] 1.2 `Step::PlayerData` applied; `Step::RebuildPlayer` and `save_live::RebuildPlayer`
- [x] 1.3 Test through the flow and save plugins over a scratch folder

## 2. Menu scene

- [x] 2.1 Read the menu scene load and its two callers; the camera constants
- [x] 2.2 `Step::MenuScene`, `MenuSceneView`, the viewer's no-player load and the fixed camera
- [x] 2.3 Tests: the flow and lifecycle path, the held camera; a worker GPU shot

## 3. Transition save

- [x] 3.1 Read the exit, the transition save, the copy and the level-available test
- [x] 3.2 `SaveStep::TransitionSave` and `level_available` at the exits
- [x] 3.3 Tests over a scratch folder
