# Tasks

- [x] Rebase the lane onto main.
- [x] Verify the `TexPos` routine against the assembly: 32-bit difference, five-bit shift mask, signed level test, sign extension, 64-bit subtraction.
- [x] Take the whole-tile test as signed and pin it; correct the comment and the docs' description of the level passed.
- [x] Keep the selector invariant test.
- [x] fmt, clippy -D warnings and tests for exa_world and exa_render; `openspec validate --all --strict`.
