## ADDED Requirements

### Requirement: Supersampling
The system SHALL supersample the scene as the original does at its 4× setting: it is drawn at four samples per window pixel, averaged with equal weights into the window-sized image, then post-processed at window resolution.

#### Scenario: Fine texture detail
- **WHEN** a detailed floor fills the view
- **THEN** its fine detail shows cleanly, without aliasing patterns
