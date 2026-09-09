# Scanner fixture verification

Supply representative film/slide-scanner JPEGs locally in
`tests/fixtures/scanner/` (`.jpg` and `.jpeg`, case-insensitive).
Fixtures are private, ignored by Git, and must not be committed.
Use scanner output from your normal workflow, including Photoshop-processed
files with ICC, XMP, and IPTC metadata. Files must exceed 1 KB to exclude
old Git LFS pointer stubs; production examples are normally several MB.

Install ExifTool (`brew install exiftool`) and run:

```sh
make test-gate
make test-fault
```

`test-gate` runs metadata preservation, GPS hemisphere round trips, and
GPS validity tests against scanner fixtures. It also tests capture-time
set/remove operations and copies with EXIF removed, checking that unrelated
metadata and compressed image bytes survive. Test copies live in temporary
directories; source fixtures are never modified.

`test-fault` exercises the atomic writer, including GPS-only, capture-time
set, and capture-time removal interrupted before rename. The original must
remain byte-identical.

EXIF fixture-dependent tests are explicitly marked ignored in normal
`cargo test --workspace` output. The commands above include them and fail
with an actionable error if no usable scanner fixture exists. They never
silently return success when fixtures are missing. Synthetic JPEG tests
and fixture-discovery failure tests run without private photos.

The shared EXIF fixture helper uses only `tests/fixtures/scanner/`; old
Sony/Canon/Nikon fixture directories are not part of the product scope.
