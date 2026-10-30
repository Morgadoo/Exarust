# Spec Delta

## ADDED Requirements

### Requirement: Lever light search
A kind 6 lever SHALL take as its light the first light node found from the parent of its arm's node, testing that node and then each child in order.

#### Scenario: Light beside the arm
- **WHEN** a lamp hangs from the lever object's node beside the arm
- **THEN** the lever finds it and fades it instead of signalling 0

### Requirement: Fire job spawn
A signalled fire job SHALL spawn a fire at its target's node of the signalled size and SHALL leave the node's lights as they are; a light job SHALL NOT scale lights.

#### Scenario: Torch lit by a lever
- **WHEN** a lever signals 1 to a fire job of stored size 150
- **THEN** a fire of size 1.5 appears at the node and the node's lights are unchanged
