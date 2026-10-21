# Design

## Behaviour

Both powers keep a state. Event 2 sets it to 2, which means the effect is being removed; any other state leaves it on.

The mask, in state 2, clears bit `0x10000` in the caster's body flags, notifies and returns 2 (inactive); otherwise it sets the bit and returns 0 (active). The sense, in state 2, reads its value, calls the trigger when it is above zero, and returns 2; otherwise it returns 0.

## Model

`MindEffect { state }`, `step_mind_mask(effect, event, flags) -> MindMaskStep`, `step_mind_sense(effect, event, value) -> MindSenseStep`.

## Open questions

The mask's visual and the sense's target scan are separate.

## Docs

`SG_Thaum_Mind` is **3%**. The headline stays 14.4% and 5.7%.
