# Release Guide

This document defines how we create and publish release artifacts.

## 1. End-user app model

- Public end-user app name: `Le Compositeur`
- Main end-user entry point: GUI
- CLI remains a technical/admin tool for source-based workflows

## 2. GitHub release workflow behavior

Workflow file:
- `.github/workflows/release-alpha.yml`

Trigger:
- push tags: `v*` or numeric tags like `2026.03.08-5`
- manual `workflow_dispatch`

Release publish strategy:
- build in matrix jobs
- upload artifacts per platform
- publish GitHub release in one dedicated job

Reason:
- avoids duplicated "What's Changed" text caused by per-matrix release publishing

## 3. Current release artifacts

GitHub release assets currently include:
- Linux:
  - `le-compositeur-linux-x86_64.tar.gz`
  - `le-compositeur-linux-x86_64` (single GUI binary)
- Windows:
  - `le-compositeur-windows-x86_64.zip`
  - `LeCompositeur-windows-x86_64.exe` (single GUI binary)
- macOS ARM:
  - `le-compositeur-macos-arm64.dmg`
  - `le-compositeur-macos-arm64` (single GUI binary)

Note:
- `macos-x86_64` is currently excluded from CI due to unsupported runner image in this project setup.

## 4. Where to find downloadable files

Temporary (run-scoped):
- GitHub `Actions` -> selected run -> `Artifacts`

Permanent (release-scoped):
- GitHub `Releases` -> select tag -> `Assets`

## 5. Native package outputs (local build scripts)

If you build packages locally:
- RPM output: `~/rpmbuild/RPMS/x86_64/`
- DEB output: `./dist/`

These are not yet auto-published by the current GitHub release workflow.

## 6. Tagging and release type

Stable release:
- tags like `2026.03.08-5` or `v1.20260308.1`

Pre-release:
- tags containing one of:
  - `-rc`
  - `-beta`
  - `-alpha`
  - `-pre`

## 7. Quick release commands

```bash
git checkout main
git pull origin main
git tag 2026.03.08-5
git push origin 2026.03.08-5
```
