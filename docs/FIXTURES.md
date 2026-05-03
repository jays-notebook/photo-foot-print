# [needs_user_action] Fixture acquisition for Phase 1's metadata-preservation gate

Phase 1's success criterion #3 (the gate) requires at least one representative
scanner-output JPEG at:

- `tests/fixtures/scanner/*.jpg`

Until at least one such file exists, the metadata-preservation tests in
`crates/pfp-exif/tests/metadata_preservation.rs` are `#[ignore]`d (see Plan 02
in `.planning/phases/01-bootstrap-exif-spike-gate/`). `cargo test` will pass
on a fresh clone, but the gate is unverified. The gate runs explicitly via
`make test-gate`, which fails loudly when no fixture is supplied.

> **Pre-D-10 note:** The gate originally required Sony / Canon / Nikon DSLR
> JPEGs to validate vendor MakerNote preservation. That requirement was
> retired (see `.planning/phases/01-bootstrap-exif-spike-gate/01-CONTEXT.md`
> D-10). The app's target user is a film photographer; source files come
> from negative/slide scanners and are post-processed in Photoshop, so DSLR
> MakerNotes are not relevant.

## Steps

1. Install Git LFS (one-time):

       brew install git-lfs
       git lfs install --local

2. Install exiftool (used by the gate to diff metadata before/after writes):

       brew install exiftool

3. Copy one (or more) representative JPEG(s) from your film-scan + Photoshop
   pipeline into:

       tests/fixtures/scanner/01.jpg
       tests/fixtures/scanner/02.jpg
       ...

   The test enumerates every `*.jpg` file in this directory; one fixture is
   enough to satisfy the gate, more is better.

   Notes:
   - **Representative of production input.** The fixture should look like
     what the app will see in real use: scanner-software output (NORITSU EZ
     Controller, Frontier, Plustek, Coolscan, Epson V-series), optionally
     post-processed in Photoshop.
   - File size ≥ 1 MB. Smaller files trip the LFS-pointer guard and the
     fixture will be skipped.
   - GPS may be present or absent — the test copies the file before mutating,
     so the fixture is never modified by the suite.

4. Record provenance + sha256 in `tests/fixtures/README.md` (append a row to
   the inventory table). sha256 command on macOS:

       shasum -a 256 tests/fixtures/scanner/*.jpg

5. Commit. Git LFS will smudge the JPEGs into pointer files automatically per
   `.gitattributes`.

6. Run the gate:

       make test-gate

   On a fresh clone with no fixtures, this now panics with a clear actionable
   message. With at least one fixture supplied, it runs the real
   metadata-preservation diff.

## What does the gate actually check?

For every fixture in `tests/fixtures/scanner/*.jpg`, the test:

1. Copies the fixture to a tempdir.
2. Captures `exiftool -a -G1 -s` output (call this `before`).
3. Captures the compressed image data from the SOS marker through EOI.
4. Calls `pfp_exif::write_gps(&copy, lat, lng, None, None)`.
5. Captures `exiftool -a -G1 -s` again (call this `after`).
6. Captures the post-write image data.
7. Asserts:
   - Image data byte-identical (EXIF-09).
   - `before` filtered minus the expected drift groups (`[GPS]`,
     `DateTimeOriginal`, `ModifyDate`, exiftool's filesystem-metadata group,
     and `[IFD1] ThumbnailOffset` pointer shift) equals the same filter on
     `after` (EXIF-08).

If either assertion fails, ROADMAP.md "Phase Gating" applies — Phase 2
cannot start, and Phase 1 is re-planned around an alternative writer
(`img-parts` + hand-encoded EXIF byte payload, or `exiftool` shell-out).

## Privacy

Fixtures live in your local clone. If this repo is made public:

    exiftool -gps:all= tests/fixtures/scanner/*.jpg

removes GPS coordinates from the fixture before pushing.
