## Why

Roadmap task B4 (lane `lighting2`, triage lanes L07 "lighting, GI & shadows"
and L10 "render pipeline & post"). The post processor's two fades were ported
as values only (test-only), so no screen faded in from black and no
desaturation fade ran; and a placed object's lights took its condition share
only at spawn.

## Evidence

REA (Exanima.exe 0.9.5):

- The final post pass uploads a `Blur` vector: the screen fade's weight
  (alpha to the tenth), the bloom strength times alpha, the desaturation mask
  and the kind's bloom term; it clears the fade's flag (and the desaturation's
  kind) once a fade has run out. The shader mixes the graded scene toward its
  bloom by `Blur.z`, scales it by `Blur.x`, and screens the bloom times
  `1.75·Blur.y + Blur.w`.
- The screen fade's start is called by the story start (after its camera shot)
  and the revive (last); the desaturation start by the mind awakening (kind 0)
  and the teleport bubble's hold (kind 1).
- The object light pass sets, for each child of a placed object, a mesh's
  emissive scale and a voxel light's intensity (its base times
  (1 − c/255)²) and recomputes the colour; it runs again whenever the
  object's condition changes.

The exanima01 near-flame excess (`context/artifacts/2026-11-17-lighting2/`)
lies in the GI share: no code change here.

## What changes

- `ExaPost.blur` carries the `Blur` vector; `post.wesl` applies it (idle it is
  (1, strength, 0, 0), the old picture).
- `ScreenFade` (resource) takes fade requests (`FadeRequest`, also a bridged
  message for the simulation step), stamps them with the frame clock, writes
  every post camera's `Blur` and stops a fade that has run out.
- `Step::ScreenFade` in the story start and the revive; the mind awakening and
  the bubble's hold start the desaturation fade.
- `ConditionLit` on condition-scaled placed objects and
  `rescale_condition_lights`, which re-scales their voxel lights and glow
  materials when the condition byte changes.
- `EXA_GI_DEBUG=diffuse`: the direct lights without their highlights.
