# Spec Delta

## ADDED Requirements

### Requirement: Stagger walk
A stagger target SHALL pull the stance origin back to within 1.1 step lengths of the frame, and one footwork pass SHALL anchor one foot on its shifted spot. A target inside that length SHALL stay where it is. A target on the frame SHALL move no foot.

#### Scenario: A far target, a near target, and the frame
- **WHEN** a target 2 m ahead is walked, a target half a metre ahead is walked, a target on the frame is walked, and the stagger from reach 80, balance 1 and velocity 3 and 4 is walked
- **THEN** the far target pulls the origin to z bits 0x3f6147ae and anchors the first foot there with state 4, the near target stays at z bits 0x3f000000, the frame moves no foot, and the stagger pulls the origin to bits 0x3f072b02 and 0x3f343958 and anchors the first foot at bits 0x3f53f7cf, 0xbe4ccccc and 0x3f343958
