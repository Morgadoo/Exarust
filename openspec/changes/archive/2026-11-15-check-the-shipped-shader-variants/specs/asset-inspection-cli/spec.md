## ADDED Requirements

### Requirement: Shipped shader variant listing
`exa-tool shader --variants` SHALL group every material record a draw reaches (the libraries, then each chunk tree's own records, resolved as the model loader does) by program set, material word and shader word, and print per group its record and draw counts, the template flags its keys set, its distinct expanded programs over the shipped level settings, and the WESL branch the loader takes.

#### Scenario: Translucent foliage
- **WHEN** the listing runs on 0.9.5's data
- **THEN** the `shadetrl_f` groups show the translucent WESL branch
