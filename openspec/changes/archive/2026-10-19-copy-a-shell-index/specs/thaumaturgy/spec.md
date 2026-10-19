# Spec Delta

## ADDED Requirements

### Requirement: Shell indices
A shell index list SHALL copy each section that is not hidden, from its first triangle through its last, inclusive. Each triangle SHALL keep its three indices in order, after the remap. A hidden section SHALL be skipped. A range that ends before it starts SHALL copy nothing. A short index list or a missing remap SHALL be an error.

#### Scenario: A kept winding and a hidden section
- **WHEN** two visible ranges and a hidden range are copied, a reversed triangle is copied, a range that ends early is copied, a short list is copied, and a missing remap is copied
- **THEN** the kept list is 14, 15, 16, 11, 12, 10, 17, 18, 19, the reversed triangle stays reversed, the early range is empty, the short list is an error, and the missing remap is an error
