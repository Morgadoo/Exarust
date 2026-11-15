## Why

Roadmap task B1 (wire the uncalled), the Mind powers (`wiremind` lane). The Sense Minds casts answered with an overlay request that nothing consumed, so casting Sense Minds showed no sight: the overlay's start, frame, tracker, colours, sphere and draw were ported and tested but had no live caller, and the mind awakening's glow was never added. The Loyalty marker's strength drift and the Loyalty cast's strength read relation 0 instead of the bound body's real relation. The bar's toggle in a body the player was handed toggled the bar instead of ending the control.

## What the original does

REA on Exanima.exe 0.9.5, checked in the assembly:

- The instant Sense Minds and the persistent one's activation build the one sight object once and start it (for the cast's duration, or with no end). The start registers the sight's frame hook when the overlay is off, and two global hooks: a level-change hook that drops the tracked minds and a story-quit hook that resets the overlay. The persistent power's end sets the overlay to its fade.
- The frame hook fades the overlay in and out over 2 s, unregisters itself once faded out, runs the tracker (every character but the player and the body it left; the brain's worst hostility, concern sum, readiness, stress, alarm and courage; the fall stage; the ward bit) and draws each tracked mind with a head part on the sight sphere, placed by the race record's sight place.
- The mind awakening game events add a glow on the player that fades over 3 s, drawn with the global sight noise.
- The Loyalty cast and its marker's tick past 800 read the marked body's last attitude toward its caster.
- The bar's toggle with the control bit `0x20_0000` on the player's body ends the Spirit Form or Control Mind of the body left behind, and does not toggle the bar.

## What Changes

- `exa_formats::races::RaceStats::mind_sight` reads the race record's sight place (lift, forward offset, three scales).
- `exa_game::mind::draw` runs the overlay (`MindSense`), the glows and the draw list on the frame; the bar's answer starts the overlay, `tick_minds` sends the persistent power's end to the frame, a new level drops the tracked minds and the story quit's step resets it.
- `game_event_live` adds the awakening's glow on the player.
- `thaum::live::MindRelations` gives the Loyalty cast and its marker the real relation.
- The bar toggle in a handed body ends the control.

## Impact

- `exa_formats::races`, `exa_formats::character_stats`, `exa_game::mind::{draw, sight, runtime}`, `exa_game::thaum::{cast, live}`, `exa_game::game_event_live`, `exa_game::flow`.
- Lockstep: the Loyalty marker's strength past 800 ticks now follows the relation, so a Loyalty scene longer than 9.6 s changes its traces; the sight runs on the frame only and changes no simulation value.
