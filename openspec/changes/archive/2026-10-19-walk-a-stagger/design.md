# Design

## Walk

`walk_stagger` takes the stagger target, the character frame and the clip's ankle spots. The step length is 80 times the square root of the scale. The stance origin is the target pulled back to 1.1 of that length from the frame, horizontally, at the frame's height. Each spot keeps its offset from the frame and moves to the origin. One footwork pass then anchors at most one foot on its new spot, at the ankle's height. A target already inside the length is used as it is. A target on the frame leaves both feet.

## Open questions

- The dying tick does not call this walk. A played collapse still does not step.
- The chest pins' lead during the step is the recovery lead, already shipped, and is not called from this function.

## Docs

The combat-rule figure stays 27.3%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
