# Design

The cache, bindings and light slots are plain Rust models of the original's
CPU-side rules, kept beside the WESL renderer rather than inside it: Exarust
does not create one GPU pipeline per program key, so a per-key cache in the
render path would only add work. The models carry the original's numbers
and order so they can be compared with the warm-up table and with captured
uniforms, and they can be wired later where a renderer path needs them.

- Bucket key: `(material, shader)`; entry key: `(object, environment)`.
  Both are compared whole, as the engine compares 64-bit values.
- Light slots keep each light's last slot in a per-state map; the engine
  keeps it on the light. A cleared state releases its lights' slot indices
  and keeps the uniforms it wrote.
- The material block writes zero where the engine skips a field and keeps
  the previous draw's value (the block belongs to the drawing state).
- No new GPU uniform type is introduced, so no `ShaderType` layout changes.
