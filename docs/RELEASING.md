# Release process

GDBLE releases are two-phase so source, manifests, hashes, and binaries share one commit.

## 1. Prepare release PR

1. Update `Cargo.toml`, `Cargo.lock`, documentation, and changelog content for the target version.
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

## Required evidence

- fmt, strict Clippy, all automated tests, and platform builds pass.
- Godot 4.2 and current stable load smoke tests pass.
- Android AAR manifest, plugin class, both ABIs, and btleplug classes are present.
- API 23–30 and API 31+ ARM64 real-device BLE loops pass.
- x86_64 emulator load test passes.
- The tag archive has no raw Android `.so`, Demo addon copy, or legacy unreferenced binaries.
