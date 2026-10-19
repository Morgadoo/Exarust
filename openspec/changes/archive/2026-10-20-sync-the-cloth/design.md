# Design

## Behaviour

The method reads the object's meta flags at 0x128 plus 8. When bit 0x10 is set it returns with no work. Otherwise it computes the transform from the object's placement and the meta's 0x40 block, applies it once, and calls the pose with 1.0 in both floating arguments.

## Open questions

The transform and pose bodies are not in.

## Docs

This method is 108 bytes. `TVCCloth` is 74%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
