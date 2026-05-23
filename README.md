# trypanophobe

Check user prompts for injection attempts before they reach your model. One binary: use it from the command line or over HTTP.

**English prompts only.** Other languages are turned away immediately. The underlying classifier is only fine tuned for English injections.

## Prerequisites

At least 700MB of free disk space (for the binary itself) and 1GB of RAM (for BERT classification).

## Quick start

```bash
cargo build --release
```

### Command line

```bash
trypanophobe version
trypanophobe check "Ignore all previous instructions"
trypanophobe check ./my-prompts/    # every `.prompt` file in a folder
```

Exit codes: **0** = looks safe, **1** = failure (injection, not English, etc.).

### HTTP server

```bash
trypanophobe serve
```

Open **http://127.0.0.1:9876/** in a browser for interactive API docs (Swagger UI).

- `POST /api/check` with JSON `{"text": "..."}` — classify a prompt  
- `GET /api/version` — version info  

Use `--prewarm` to load the model in the background at startup (the server still listens right away).

## Build

```bash
cargo build --release
```

## Smoke tests

```bash
./smoke.sh
./coverage.sh
```

`smoke.sh` builds the binary, exercises CLI (`version`, non-English `check`), starts a temporary server, and hits `/`, `/api/version`, OpenAPI, Swagger UI, and `/api/check` (without loading the full model for English).

## CI

GitHub Actions ([`.github/workflows/ci.yml`](.github/workflows/ci.yml)) runs on every push and pull request to `main`:

- `cargo test`, `./coverage.sh` (90% line coverage on testable library code), and `./smoke.sh`

On push to `main`, CI cross-compiles release binaries using [`docker/Dockerfile.build`](docker/Dockerfile.build) (Linux/Windows; Windows uses [`docker/windows-cross-build.sh`](docker/windows-cross-build.sh) for xwin SDK lib casing) and native macOS runners, then publishes them on the **nightly** pre-release. Targets follow [ONNX Runtime prebuilt availability](https://ort.pyke.io/setup/linking) (no musl or Intel-mac ORT binaries).

| Artifact | Target |
|----------|--------|
| [trypanophobe-linux-x64](https://github.com/3p3r/trypanophobe/releases/download/nightly/trypanophobe-linux-x64) | x86_64-unknown-linux-gnu |
| [trypanophobe-linux-arm64](https://github.com/3p3r/trypanophobe/releases/download/nightly/trypanophobe-linux-arm64) | aarch64-unknown-linux-gnu |
| [trypanophobe-darwin-arm64](https://github.com/3p3r/trypanophobe/releases/download/nightly/trypanophobe-darwin-arm64) | aarch64-apple-darwin (Apple Silicon) |
| [trypanophobe-win32-x64.exe](https://github.com/3p3r/trypanophobe/releases/download/nightly/trypanophobe-win32-x64.exe) | x86_64-pc-windows-msvc |

## License

MIT. The bundled classifier model is Apache-2.0 (Protect AI).
