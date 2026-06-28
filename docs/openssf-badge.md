# OpenSSF Best Practices — justification updates

Project: https://www.bestpractices.dev/en/projects/13385  
Edit form: https://www.bestpractices.dev/en/projects/13385/passing/edit

After merging repo changes (CONTRIBUTING § Automated tests, `just test`, CI `cargo test`), update these criteria on the live form and **Submit**.

## `tests_documented_added` → **Met**

Policy is documented in CONTRIBUTING:

```
Documented in CONTRIBUTING.md § Automated tests: <https://github.com/org-event/OpenPolySphere/blob/main/CONTRIBUTING.md#automated-tests>. Contributors should add unit tests for major new features where practical; run with `just test` or `cargo test -p audio-core@0.1.0 --lib`.
```

## `test_most` → **Unmet** (honest; SUGGESTED)

```
Unit tests cover VAD/downsample logic in crates/audio-core/src/vad/mod.rs (4 tests). CI runs `cargo test -p audio-core@0.1.0 --lib` on macOS, Windows, and Linux. Most of the audio pipeline, web UI, and call integration are validated via clippy, multi-platform builds, manual call testing, and ClusterFuzzLite fuzzing — not branch coverage. Broader automated coverage is planned incrementally.
```

## `dynamic_analysis_enable_assertions` → **Unmet** (SUGGESTED)

```
ClusterFuzzLite address-sanitizer fuzzing runs on every pull request (.github/workflows/cflite_pr.yml, 300s, code-change mode). Rust `debug_assert!` is enabled in debug builds by default. There is no separate written release checklist beyond PR merge gates (CI + fuzzing); formal release assertion policy may be added later.
```

## Optional: strengthen existing Met criteria

**`test`** — if you edit justification:

```
Rust unit tests in crates/audio-core (VAD/downsample). Run: `just test` or `cargo test -p audio-core@0.1.0 --lib`. Documented in CONTRIBUTING.md § Automated tests. CI runs tests on macOS, Windows, and Linux (.github/workflows/ci.yml).
```

**`test_continuous_integration`**:

```
GitHub Actions CI on push/PR: rustfmt, clippy (-D warnings), ESLint, `cargo test -p audio-core@0.1.0 --lib`, and macOS/Windows/Linux release builds (.github/workflows/ci.yml).
```

**`tests_are_added`**:

```
VAD unit tests maintained in crates/audio-core/src/vad/mod.rs; v0.4.x Linux port validated by new CI job plus existing tests. Policy in CONTRIBUTING.md § Automated tests.
```
