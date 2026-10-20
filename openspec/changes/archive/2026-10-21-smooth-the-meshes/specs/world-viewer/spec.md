## ADDED Requirements

### Requirement: Smoothed model view
`exarust model <pack> <entry> --msm` SHALL display the model with every mesh subdivided once as the original's `msm` debug command and its character-creation hair are, its bounds refitted, warning of meshes with too many faces.

#### Scenario: Hair
- **WHEN** `exarust model CharExt "hair hula01" --msm` runs
- **THEN** the hair is shown with four faces for each stored one
