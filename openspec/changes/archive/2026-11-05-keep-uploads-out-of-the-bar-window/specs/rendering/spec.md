# Spec Delta

## ADDED Requirements

### Requirement: Upload memory
The renderer's upload (staging) buffers SHALL be placed in host-visible device-local memory only when that memory's heap is at least 1 GiB (a resizable BAR), and in host-visible system memory otherwise, so that a game sharing the GPU never writes its uploads through the shared 256 MiB BAR window.

#### Scenario: Four games on a GPU without a resizable BAR
- **WHEN** four games run in one grid on an RTX 3090 whose BAR1 window is 256 MiB
- **THEN** no game maps a BAR block for its uploads, each one's staging writes run at system-memory speed, and no game's render thread stalls on its uploads

#### Scenario: Resizable BAR
- **WHEN** the device-local host-visible heap is the whole of video memory
- **THEN** uploads keep using it
