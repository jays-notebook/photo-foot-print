# Fixtures: Real DSLR JPEGs for the MakerNote-preservation gate

These JPEGs are the gating test corpus for Phase 1's success criterion #3 (see
`.planning/ROADMAP.md`). The MakerNote-preservation fixture suite proves that
`pfp-exif::write_gps` does not strip vendor MakerNote sub-IFDs when a GPS
write is performed.

## Required files

- `sony/sample.jpg`  — one Sony α / DSLR / mirrorless camera JPEG
- `canon/sample.jpg` — one Canon EOS DSLR / mirrorless JPEG
- `nikon/sample.jpg` — one Nikon DSLR / mirrorless JPEG

Each file should:
- Be a real OOC (out-of-camera) JPEG, NOT a Lightroom export — Lightroom strips
  MakerNotes by default, so an exported JPEG would defeat the gate.
- Be at least one MB in size to ensure a populated MakerNote IFD.
- Optionally have GPS already populated (the test copies the file before
  mutating, so the fixture is never modified).

## Storage: Git LFS

These files are stored via Git LFS per `.gitattributes`. Clone with `git lfs
install` enabled, or run `git lfs pull` after cloning if the JPEGs come down
as text pointers.

## Provenance + sha256 inventory

When a fixture is added, append a line below: `vendor/file.jpg | sha256 | source`.
The sha256 detects corruption-on-clone (LFS smudges can fail silently).

| File | sha256 | Source |
|------|--------|--------|
| _none yet — see `docs/FIXTURES.md` for the user-action checklist_ | — | — |

## License / privacy note

The fixtures may contain identifying GPS coordinates and MakerNote camera
serial numbers. Before making this repo public, run:

    exiftool -gps:all= tests/fixtures/<vendor>/sample.jpg

to strip GPS, and consider replacing fixtures with publicly-shareable trip
photos. For a personal repo, this is moot.
