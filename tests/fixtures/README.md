# Fixtures: Scanner-Output JPEGs for the Metadata-Preservation Gate

These JPEGs are the gating test corpus for Phase 1's success criterion #3 (see
`.planning/ROADMAP.md`). The metadata-preservation fixture suite proves that
`pfp-exif::write_gps` does not disturb non-GPS metadata (ICC profile, XMP,
IPTC, Adobe APP14, Photoshop tags) or the compressed image data when a GPS
write is performed against scanner-output JPEGs.

> **Note:** Pre-D-10 (see
> `.planning/phases/01-bootstrap-exif-spike-gate/01-CONTEXT.md`) the gate was
> about DSLR vendor MakerNote preservation across Sony / Canon / Nikon. That
> requirement is retired — this app's target user is a film photographer,
> source files are scanner output, and scanner JPEGs carry no vendor MakerNote
> tags by definition.

## Required files

Place at least one representative scanner-output JPEG at:

- `tests/fixtures/scanner/*.jpg`

The test enumerates every `*.jpg` file in that directory. One file is enough
to satisfy the gate; multiple files give more coverage if you have varied
sources (different scanner models, different Photoshop versions, etc.).

Each file should:
- Come from your actual film-scan + Photoshop pipeline (NORITSU EZ Controller,
  Frontier, Plustek, Nikon Coolscan, Epson V-series, etc.) — i.e.,
  representative of what the app will see in production.
- Be at least 1 MB in size (small files trip the LFS-pointer guard).
- Carry the metadata categories the test checks: ICC profile, XMP, IPTC,
  Adobe APP14, Photoshop tags. Most Photoshop-saved files have all of these.

## Storage: Git LFS

These files are stored via Git LFS per `.gitattributes`. Clone with `git lfs
install` enabled, or run `git lfs pull` after cloning if the JPEGs come down
as text pointers.

## Provenance + sha256 inventory

When a fixture is added, append a line below: `file.jpg | sha256 | source`.
The sha256 detects corruption-on-clone (LFS smudges can fail silently).

| File | sha256 | Source |
|------|--------|--------|
| _none yet — see `docs/FIXTURES.md` for the user-action checklist_ | — | — |

## License / privacy note

The fixtures may contain identifying GPS coordinates and other personal
metadata. Before making this repo public, run:

    exiftool -gps:all= tests/fixtures/scanner/*.jpg

to strip GPS, and consider replacing fixtures with publicly-shareable photos.
For a personal repo, this is moot.
