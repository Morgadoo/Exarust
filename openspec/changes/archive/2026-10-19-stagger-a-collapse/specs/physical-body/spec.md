# Spec Delta

## ADDED Requirements

### Requirement: Collapse stagger
A body that is already stepping SHALL take no stagger. Otherwise a balance that widens above 0.4 SHALL scale the reach by the balance, and any lower balance SHALL scale the widened reach by 0.4. The span SHALL square the tone plus one half and multiply by that length and the scale root. The target SHALL add the velocity's reciprocal-square-root unit times the span to the chest. A zero velocity SHALL leave the chest. Landings and the step kind SHALL be 0.

#### Scenario: A still chest, a balance of 0.4, and a step already running
- **WHEN** reach 80 at balance 1, tone 1 and scale root 1 is staggered with velocity 3 and 4, reach 0.01 is staggered at balance 0.4 and at the next lower balance, a zero velocity is staggered, a scale root of the square root of 2 is staggered at balance 0.8 and tone 0.5, and a body that is already stepping is staggered
- **THEN** the first step has length bits 0x42a00000 and target bits 0x42d7faf0, 0x42be0000 and 0x430ffca0, reach 0.01 at balance 0.4 has length bits 0x3b83126f, the next lower balance has length bits 0x3b83126e, the zero velocity leaves the chest, the rooted step has length bits 0x42800000 and target bits 0x41200000, 0x42b4f9a3 and 0xc0a00000, landings and the step kind are 0, and the body that is already stepping takes nothing
