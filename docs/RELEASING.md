# Release process

GDBLE releases are two-phase so source, manifests, hashes, and binaries share one commit.

## 1. Prepare release PR

1. Update `Cargo.toml`, `Cargo.lock`, documentation, and `release-notes/v<version>.md` for the target version. Commit the notes before starting the workflow; the tag must include them.
2. Run the `Prepare release` workflow with the version without a `v` prefix.
3. The workflow builds all desktop libraries and both Android AARs, replaces the canonical `addons/gdble` artifacts, writes `SHA256SUMS`, rejects stale legacy artifacts, and opens a release PR.
4. Review CI, artifact contents, and device-test evidence, then merge the PR.

The prepare workflow does not create a tag or GitHub release.

## 2. Tag the artifact commit

Create `v<version>` on the merged commit that already contains every release artifact:

```bash
git tag v0.6.0 <merged-commit>
git push origin v0.6.0
```

The tag workflow only validates hashes/layout, packages the tag contents, and publishes the release. It never writes back to a branch.

## Release notes

Keep one UTF-8 Markdown file per version in `release-notes/`, for example `release-notes/v0.6.0.md` for tag `v0.6.0`. Copy `release-notes/TEMPLATE.md` when preparing a new version. Prerelease tags such as `v0.7.0-rc.1` use the same exact filename convention.

GitHub Actions validates the version and rejects missing or empty notes before publishing. The release body is read from this file at the tagged commit through `body_path`; it is not generated automatically or embedded in the workflow. Editing the file without creating a new tag does not update an already published release.

Validate locally with Python 3.11+ using `python3 scripts/release-notes.py v0.6.0`. Changing the text for an upcoming release only requires editing its Markdown file, not either workflow.

## Required evidence

- fmt, strict Clippy, all automated tests, and platform builds pass.
- Godot 4.2 and current stable load smoke tests pass.
- Android AAR manifest, plugin class, both ABIs, and btleplug classes are present.
- API 23–30 and API 31+ ARM64 real-device BLE loops pass.
- x86_64 emulator load test passes.
- The tag archive has no raw Android `.so`, Demo addon copy, or legacy unreferenced binaries.
