# Scanner fixture verification

Supply representative film/slide-scanner JPEGs locally in
`tests/fixtures/scanner/` (`.jpg` and `.jpeg`, case-insensitive).
Fixtures are private, ignored by Git, and must not be committed.
Use scanner output from your normal workflow, including Photoshop-processed
files with ICC, XMP, and IPTC metadata. Files must exceed 1 KB to exclude
old Git LFS pointer stubs; production examples are normally several MB.

Install [ExifTool](https://exiftool.org/) (`brew install exiftool` on macOS)
and ensure `exiftool` is on PATH. On macOS, run:

```sh
make test-gate
make test-fault
```

On Windows PowerShell, use the equivalent commands without GNU make:

```powershell
cargo test -p pfp-exif --test metadata_preservation --test round_trip --test strict_4_predicate -- --include-ignored
cargo test -p pfp-exif --test fault_injection --features fault-injection -- --include-ignored
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

Portable list/thumbnail/host smoke tests generate real JPEGs from a pixel
pattern in temporary directories using `tests/support/synthetic_jpeg.rs`.
They never fill the private scanner directory with synthetic substitutes.
The separate scanner-only photo-list assertion is explicitly ignored unless
requested and fails when no local scanner fixture exists.

The shared EXIF fixture helper uses only `tests/fixtures/scanner/`; old
Sony/Canon/Nikon fixture directories are not part of the product scope.
