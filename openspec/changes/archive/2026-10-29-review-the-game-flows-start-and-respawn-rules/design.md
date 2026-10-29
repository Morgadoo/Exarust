# Design

The new data stays as named steps and fields so a host can apply them:
`StartView`, `StoryStart::spawn_point`, `StoryStart::start_record` and
`Step::StartRecord`, `Step::LevelTime`, `Step::PlayerData { flags }`.

Two steps are applied in the running game because their state already
exists there: `Step::Condition` writes the player's `exa_combat::Condition`,
and `Step::Wake`/`Step::StoryPose` write a `WakeFlags` component on the
player body (replacing, or or-ing for the revival), which the flow tick
reads back as `PlayerReading::animator`.

The unlock words reach the boot as an `UnlockSettings` resource built from
the user's settings store; a missing entry keeps the current word, as the
original's failed read does.

The exit nudge's test is written `value > 0.0` so the unordered result
falls to the negated branch, as the original's parity and below-or-equal
jumps do.
