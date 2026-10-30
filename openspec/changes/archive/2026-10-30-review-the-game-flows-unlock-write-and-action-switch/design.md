# Design

The flow stays a pure state machine that returns steps; the binary owns
the settings store, so the unlock write is a `Step::SettingsWrite` that
`apply_steps` turns into a `SettingsEntry` message and
`user_settings::write_flow_settings` writes and flushes. `Flow` keeps an
`expert_written` flag, seeded at boot from the stored unlock, standing in
for the original's single flag word.

The action switch reuses `exa_anim::Motions::start_requested`, which the
cast pipeline already used inline, so both paths start a clip the same
way. `apply_steps` skips the start when the animator's action already
names the requested one, as the unforced switch returns early.

The spawn constant stays `PRACTICE_POINT` (0, 0, 500); the lane's extra
origin constant is removed.
