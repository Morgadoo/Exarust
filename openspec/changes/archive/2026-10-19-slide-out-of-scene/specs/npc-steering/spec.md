# Spec Delta

## ADDED Requirements

### Requirement: Out of scene
When the low two scene bits are clear, a step SHALL advance 2 cm along the segment. The standing height SHALL add the pelvis base, and the ground point SHALL keep the path height. A length that is not strictly past the fraction times the length plus 2 SHALL cross once. The new fraction SHALL be (1 - (1 - fraction) times the length) times (1 - fraction), minus 2, divided by the next length. No horizontal length SHALL count as 0.1. A set low bit SHALL not slide.

#### Scenario: Two centimetres, or one crossing
- **WHEN** a body out of the scene is at the start of a 100 cm segment with pelvis base 90, another has fraction 0.75 on an 8 cm segment with 8 cm after it, another is at the start of a vertical segment with 20 cm after it and pelvis base 4, another is 1 cm from the end of a last segment, and bodies with a low scene bit set are stepped, including one whose only set bit is higher
- **THEN** the first fraction is 0.02, the ground is 2 cm along, and the standing height is 90, the second fraction is -0.28125 and the ground is 5.75 cm along, the vertical crosses onto the next segment behind its start and stands 4 above the path, the last segment stops at its end, a low bit does not slide, and the higher bit still slides
