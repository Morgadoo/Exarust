# morph-controller Specification

## Purpose
The morph controller: its target arrays and the nearest morph target it finds.

## Requirements

### Requirement: Morph controller setup
Setup SHALL store the target's count, allocate both arrays and copy the target's points.

#### Scenario: The setup
- **WHEN** the target has 3 points
- **THEN** the count is 3 and both arrays are allocated
- **AND** the three points are copied

### Requirement: Morph nearest update
The nearest update SHALL replace the best point, extra, distance and marker only when the new distance is strictly below the stored one. A NaN distance SHALL not replace.

#### Scenario: Closer, equal and NaN
- **WHEN** the distance is 0.5 and the stored distance 1
- **THEN** the best point and values are replaced
- **WHEN** the distance is 1 and the stored distance 1
- **THEN** nothing changes
- **WHEN** the distance is NaN
- **THEN** nothing changes

### Requirement: Nearest morph index
Finding SHALL scan the points and keep the smallest distance to the given point, returning its index and distance. A NaN distance SHALL not replace. An empty list SHALL return −1 and the maximum float.

#### Scenario: Scan, ties and empty
- **WHEN** the point is 0, 0, 0 and the points are 1, 0, 0; 0, 2, 0 and 0, 0, 0.5
- **THEN** the index is 2 and the distance 0.25
- **WHEN** a distance is NaN
- **THEN** it does not replace the best
- **WHEN** the list is empty
- **THEN** the index is −1 and the distance is the maximum float

### Requirement: Morph target switch
Switching SHALL do nothing unless the target's tag is `0xEDC0C589`. Otherwise it SHALL copy the point and points, store the point at the target and refresh it.

#### Scenario: Tagged and untagged
- **WHEN** the target's tag is `0xEDC0C589` and the point is 1, 2, 3
- **THEN** the switch and refresh run and the point is stored
- **WHEN** the tag differs
- **THEN** nothing is copied, stored or refreshed

### Requirement: Morph point blend
Blending SHALL add each source point times the weight to the matching destination point. The result SHALL have the shorter of the two counts.

#### Scenario: Weighted add and mismatch
- **WHEN** the destination is 1, 2, 3, the source 2, 4, 6 and the weight 0.5
- **THEN** the result is 2, 4, 6
- **WHEN** the source has fewer points
- **THEN** only the matching points are produced

### Requirement: Morph part walk
The walk SHALL count a node whose kind masked with `0xFFF` is `0x89` and whose `0xC00000` bits are `0x400000`, then visit every child recursively. The number of eligible nodes SHALL be returned.

#### Scenario: Eligible, ineligible and child
- **WHEN** a node has kind `0x89`, flags with `0xC00000` equal to `0x400000` and a child that is also eligible
- **THEN** the count is 2
- **WHEN** the node's kind is `0x01`
- **THEN** the node is not counted
- **WHEN** the flag band differs
- **THEN** the node is not counted

### Requirement: Morph weight merge
Merging SHALL do nothing when the scale is below the epsilon. Up to four source entries SHALL be read, stopping at the first id below one. An id already present SHALL add `weight × scale`; a new id SHALL append to the highest used slot plus one when that leaves room.

#### Scenario: Add, append and stop
- **WHEN** the current slots are 1 and 2 with weights 0.5 and 0.5, the source is 2 and 3 with weights 1 and 0.5, and the scale 2
- **THEN** slot 2 weighs 2.5 and slot 3 is appended at 1
- **WHEN** the scale is 0
- **THEN** nothing changes
- **WHEN** the first source id is 0
- **THEN** the merge stops

### Requirement: Morph point query
The query SHALL start at the base point and add each block point times its weight when the weight's magnitude is at least the epsilon.

#### Scenario: Weighted sum and skip
- **WHEN** the base is 1, 2, 3, one point is 2, 4, 6 and its weight 0.5
- **THEN** the result is 2, 4, 6
- **WHEN** a weight is 0
- **THEN** that point is skipped

### Requirement: Morph block parse
Parsing SHALL require the tag `0xEDC0C589`, then read the unknown word, the first count and its points, the owner's skipped points, the second count and, for each entry, the first points and the owner's points. A truncated or wrongly tagged block SHALL return none.

#### Scenario: One entry and failures
- **WHEN** the block has one first point, two skipped points and one entry
- **THEN** the first points and the entry's two runs are read in order
- **WHEN** the last four bytes are missing
- **THEN** parsing returns none
- **WHEN** the tag differs
- **THEN** parsing returns none

### Requirement: Morph vertex apply
For each vertex the system SHALL take the three points its joint names through the remap as the rows of a 3 × 3 basis, multiply the vertex's offset by them, subtract the controller's base, apply the owner's rows, and add the vertex's base. The result SHALL have one point per vertex.

#### Scenario: A basis and a base
- **WHEN** the points are (1, 0, 0), (0, 2, 0) and (0, 0, 3), the offset is (1, 1, 1), the controller's base is (1, 1, 1), the owner's rows are the identity and the vertex's base is (0, 0, 10)
- **THEN** the point is (0, 1, 12)

#### Scenario: Remap and orientation
- **WHEN** the remap is (2, 0, 1) and the offset is (1, 0, 0)
- **THEN** the point is (0, 0, 3)
- **WHEN** the owner's rows are (0, 1, 0), (−1, 0, 0), (0, 0, 1) and the offset is (1, 2, 0)
- **THEN** the point is (−4, 1, 0)

### Requirement: Morph basis
Solving the morph basis SHALL project the query point onto the control triangle's plane through its first point along the stored normal, then return the triangle's barycentric weights: the branch SHALL be chosen by the absolute x sum against the absolute y sum, the first two weights SHALL come from that branch, and the third SHALL be one minus them.

#### Scenario: First and second branches
- **WHEN** the triangle is (0, 0, 0), (1, 0, 0), (0, 1, 0), its normal is (0, 0, 1) and the query is (0.25, 0.25, 5)
- **THEN** the weights are 0.5, 0.25 and 0.25
- **WHEN** the triangle is (0, 0, 0), (2, 0, 0), (1, 0, 1), its normal is (0, 1, 0) and the query is (0.5, 5, 0.25)
- **THEN** the weights are 0.625, 0.125 and 0.25

### Requirement: Morph start setup
The morph start SHALL create the owner's helper when it has none and SHALL ensure the owner's controls only when both the helper and the controls are missing.

#### Scenario: Missing helper and controls
- **WHEN** the owner has no helper and no controls
- **THEN** the start creates the helper and ensures the controls
- **WHEN** the owner already has a helper
- **THEN** the start creates nothing and ensures nothing

### Requirement: Morph vertex binding
For each owner vertex the start SHALL transform it by the frame, bind it to the nearest candidate by the smallest squared distance from the maximum float, store that candidate's barycentric weights as the offset and the vertex as the base. With no candidate it SHALL use joint 0, a zero offset and list the vertex as unmatched.

#### Scenario: Bound and unmatched
- **WHEN** two vertices sit at 0 and 10 and two candidates sit with them, each with its own triangle
- **THEN** the records take joints 0 and 1, each with weights (1, 0, 0) and the vertex as its base, and nothing is unmatched
- **WHEN** there are no candidates
- **THEN** the record takes joint 0, a zero offset, the vertex as its base, and the vertex is unmatched

### Requirement: Morph remap rebuild
The morph remap rebuild SHALL replace the map with `input[map[i]]`. For every part whose flags are clear it SHALL gather each element by `input[j]` over the part's stride and count and replace the part's bytes; a part with flags set SHALL be left alone.

#### Scenario: Composed map and gathered parts
- **WHEN** the map is (1, 0), the input is (1, 0) and a part has one-byte components in pairs with bytes 10, 11, 20, 21
- **THEN** the map is (0, 1) and the part's bytes are 20, 21, 10, 11
- **WHEN** another part has its flags set
- **THEN** its bytes are unchanged

### Requirement: Morph remap refresh
The rebuild SHALL always rebuild the arrays, indices and vertices. With bit 0 set it SHALL also purge, mark the owner dirty, stamp the version, and re-bind the bound object when one exists.

#### Scenario: Refresh bit set and clear
- **WHEN** bit 0 is set and a bound object exists
- **THEN** the rebuild purges, marks the owner dirty, stamps the version and re-binds
- **WHEN** bit 0 is clear
- **THEN** only the arrays, indices and vertices are rebuilt

### Requirement: Morph data blob
Cooking the morph data SHALL write the magic 0xEDC0C589, a size of 12·(P+1)·(V+1)+12, the base point, the rest points, and each part's pivot and one delta per vertex.

#### Scenario: One part and two vertices
- **WHEN** one part and two rest points are cooked
- **THEN** the magic is 0xEDC0C589 and the size is 84

### Requirement: Morph delta
A vertex's delta SHALL be the barycentric triangle point minus the part's pivot, rotated by the owner's three rows.

#### Scenario: Identity and a quarter turn
- **WHEN** the barycentric weights are 0.5, 0.25 and 0.25 over the corners (0, 0, 0), (1, 0, 0) and (0, 1, 0), the pivot is the origin and the rotation is the identity
- **THEN** the delta is (0.25, 0.25, 0)
- **WHEN** the rotation turns a quarter about z
- **THEN** the delta is (−0.25, 0.25, 0)

### Requirement: Skin record merge and normalize
A merge SHALL add a scaled source through the four-slot joint merge. Normalization SHALL stop at the first zero joint, zero a negative weight, then divide the weights through the last used slot by the sum.

#### Scenario: Three joints and a negative weight
- **WHEN** joints 1, 2 and 3 carry weights 1, 1 and 2
- **THEN** normalization gives 0.25, 0.25 and 0.5
- **WHEN** joints 1 and 2 carry weights 2 and −1
- **THEN** normalization gives 1 and 0

### Requirement: Skin fill
A fill SHALL merge each neighbour at 1/√(distance squared) and divide by the distance sum. With no neighbours it SHALL leave the record.

#### Scenario: Two neighbours and none
- **WHEN** neighbours at squared distances 4 and 9 are merged
- **THEN** their 0.5 and 1/3 weights are divided by their sum
- **WHEN** there are no neighbours
- **THEN** the fill reports false and the record is unchanged

### Requirement: Skin relaxation
Three relaxation passes SHALL make each record half itself plus half of 1/√(distance squared) times each neighbour, then normalize.

#### Scenario: Two vertices
- **WHEN** two vertices at squared distance 4 relax toward each other
- **THEN** each record holds both joints, the self weight is larger, and the weights sum to 1

### Requirement: Morph target diffusion
Target diffusion SHALL give every unknown vertex with known neighbours the weighted mean of those neighbours at 1/√(distance squared), mark it known for the next pass, and repeat until a pass changes nothing.

#### Scenario: A chain and a stranded vertex
- **WHEN** vertex 0 is known and vertices 1 and 2 form a chain at squared distance 4
- **THEN** two passes fill vertices 1 and 2, both taking vertex 0's delta
- **WHEN** no vertex is known
- **THEN** no pass changes anything and the deltas stay

### Requirement: Morph stream registration
Stream registration SHALL run only when the controller has weights and the owner carries bit 0x200000. It SHALL reuse the stream whose low flag bits are 0x1000, or grow the owner's list by one with flags 0x04000001 and type 0x02a41000, and SHALL zero the stream's V·32 bytes.

#### Scenario: Guard, reuse and creation
- **WHEN** there are no weights or the owner bit is clear
- **THEN** nothing is registered
- **WHEN** an existing stream is found
- **THEN** it is reused and V·32 bytes are zeroed
- **WHEN** none is found
- **THEN** the list grows with flags 0x04000001, type 0x02a41000 and the vertex count

### Requirement: Morph target list
Target collection SHALL add the remapped corners of every triangle whose flags carry 0xC00000, in the order 1, 2, 0, without duplicates.

#### Scenario: Flagged and repeated triangles
- **WHEN** the first and third triangles carry the flag, are identical, and the second does not carry it
- **THEN** the list holds the first triangle's remapped corners once, in the order 1, 2, 0
