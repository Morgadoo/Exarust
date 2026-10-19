# Spec Delta

## ADDED Requirements

### Requirement: Plan around a footprint
A plan SHALL lift the footprint at the standing point, search from there, and stamp that footprint again. That search SHALL cost less than the same search with the footprint still down, and the centre SHALL be 63 again afterwards.

#### Scenario: The lifted search costs less, then the footprint is back
- **WHEN** a footprint is stamped and a search runs four cells away, and a plan then runs that same search
- **THEN** the plan costs less than the search with the footprint still down, the centre is 63, and a neighbour is 32

### Requirement: Dirty rectangle
A rectangle SHALL merge when its padded union is strictly smaller than the two padded rectangles, and the queue SHALL keep sixteen. A refresh SHALL grow each side by three cells. A sample from 1 to 254 SHALL raise the lower floor when it is 0 or within fifteen, and the upper floor when higher. One at or under the lower floor and its underside SHALL raise an empty upper floor. A changed floor SHALL be written and the costs SHALL run again. A zero-extent box SHALL cover its cell and a neighbour.

#### Scenario: Close rectangles join, and a sample raises the floor
- **WHEN** a zero-extent box is taken around cell 12, cell 12 is queued after cell 10, cell 17 is queued after cell 10, seventeen separated rectangles are queued, and a refresh is given voxels 20, 30 and 12, plus 255, 0 and a voxel outside
- **THEN** the box covers cell 12 and a neighbour inside the margin of three, the close pair is one rectangle, the far pair is two, seventeen keep the newest sixteen, voxel 20 raises the lower floor to 20 and its roughness is no longer 0, voxel 30 raises the upper floor to 30, voxel 12 raises an empty upper floor to 12, and voxel 255 and the outside floor stay as they were
