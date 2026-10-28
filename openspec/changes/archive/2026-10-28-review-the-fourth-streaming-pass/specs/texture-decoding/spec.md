## MODIFIED Requirements

### Requirement: Single-channel diffuse
A diffuse map stored as BC4, the original's luminance format, SHALL be uploaded as one channel and shown as grey on every material path, with no diffuse bias for a record that has a secondary colour. A BC4 specular or emissive map SHALL be shown as grey too.

#### Scenario: A BC4 diffuse
- **WHEN** a run-time apparel or race-skin material takes a BC4 diffuse
- **THEN** the shader repeats the red channel as grey

#### Scenario: A BC4 specular map
- **WHEN** a material's specular map is BC4
- **THEN** the material carries the grey specular flag and the shader repeats the red channel

## ADDED Requirements

### Requirement: Image reader dispatch
A name without a dot SHALL be read as a provider's image, by its magic. A name with a dot SHALL go to the reader registered for the text after its last dot, compared without ASCII case; the only reader SHALL be "rfi". An extension longer than 16 bytes, or a trailing dot, SHALL leave the extension empty, which matches no reader. A name with no reader SHALL report "Unrecognised file extension: " and the extension's first 15 bytes in the name's case, and SHALL load the grey placeholder.

#### Scenario: An unknown extension
- **WHEN** the name a.b.TGA is loaded
- **THEN** it reports "Unrecognised file extension: TGA" and loads the placeholder

#### Scenario: A leading dot
- **WHEN** the name .rfi is dispatched
- **THEN** its extension is rfi
