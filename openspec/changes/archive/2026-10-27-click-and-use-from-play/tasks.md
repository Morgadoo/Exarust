## 1. Research

- [x] 1.1 Read the click and double-click handlers, the object helper, the use routines, the mark helper, the timed list, the request, the power hold, the box test and the point transform, the door and wheel drags in the assembly

## 2. Implementation

- [x] 2.1 `player_controls::{click, interact_object, use_on, mark_used, request_mode, TimedList::tick_sections}` corrected
- [x] 2.2 `hand::{box_ok_local, wheel_turn}` corrected
- [x] 2.3 `exa_ui::hud::HudShowUntil` and the own-body click in `hand::hold_hand`; the double click gated by combat mode and Ctrl

## 3. Tests

- [x] 3.1 Unit tests pinned to the corrected branches and numbers
- [x] 3.2 `cargo test` and `cargo clippy -D warnings` for exa_game, exa_combat and exa_ui
