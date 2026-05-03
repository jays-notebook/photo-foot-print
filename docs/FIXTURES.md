# [needs_user_action] Fixture acquisition for Phase 1's MakerNote-preservation gate

Phase 1's success criterion #3 (the gate) requires three real DSLR JPEGs:

- `tests/fixtures/sony/sample.jpg`
- `tests/fixtures/canon/sample.jpg`
- `tests/fixtures/nikon/sample.jpg`

Until these files exist, the MakerNote-preservation tests in
`crates/pfp-exif/tests/makernote_diff.rs` are `#[ignore]`d (see Plan 02 in
`.planning/phases/01-bootstrap-exif-spike-gate/`). `cargo test` will pass on a
fresh clone, but the gate is unverified.

## Steps

1. Install Git LFS (one-time):

       brew install git-lfs
       git lfs install --local

2. Copy one OOC (out-of-camera) JPEG from each vendor's body into:

       tests/fixtures/sony/sample.jpg
       tests/fixtures/canon/sample.jpg
       tests/fixtures/nikon/sample.jpg

   Notes:
   - **Out-of-camera only.** Lightroom-exported JPEGs strip MakerNotes by
     default, which would defeat the gate.
   - File size ≥ 1 MB suggests a populated MakerNote IFD. ARW/CR2/NEF-derived
     in-camera JPEGs satisfy this trivially.
   - GPS may be present or absent — the test copies the file before mutating,
     so the fixture is never modified by the suite.

3. Record provenance + sha256 in `tests/fixtures/README.md` (append a row to
   the inventory table). sha256 commands:

       sha256sum tests/fixtures/sony/sample.jpg
       sha256sum tests/fixtures/canon/sample.jpg
       sha256sum tests/fixtures/nikon/sample.jpg

   On macOS without coreutils: `shasum -a 256 tests/fixtures/<vendor>/sample.jpg`.

4. Commit. Git LFS will smudge the JPEGs into pointer files automatically per
   `.gitattributes`.

5. Lift the `#[ignore]` gate on the MakerNote-preservation tests:

       cd crates/pfp-exif
       grep -rn '#\[ignore\]' tests/

   Edit each gated test in `tests/makernote_diff.rs` and remove the
   `#[ignore = "fixture missing — see docs/FIXTURES.md"]` line, OR — preferred
   — leave the `#[ignore]` in place and run them explicitly:

       cargo test -p pfp-exif --test makernote_diff -- --ignored

   The Makefile target `make test-gate` (added in Plan 02) wraps this.

## What if I cannot supply fixtures from all three vendors?

The gate is non-negotiable for shipping Phase 2. If you cannot supply a
fixture for one vendor at Phase 1 time, the project status is "Phase 1
green except gate criterion #3" — Phase 2 cannot start until the missing
vendor fixture lands and the test passes. See ROADMAP.md "Phase Gating".

## Privacy

Fixtures live in your local clone. If this repo is made public:

    exiftool -gps:all= tests/fixtures/<vendor>/sample.jpg

removes GPS coordinates from the fixture before pushing.
