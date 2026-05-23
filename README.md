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

## Tests

```bash
cargo test
./smoke.sh
./coverage.sh
```

`smoke.sh` exercises the CLI, starts a temporary server, and hits the HTTP API without loading the full model for English checks.

## Nightly binaries

Every push to `main` runs tests and publishes a [nightly](https://github.com/3p3r/trypanophobe/releases/tag/nightly) pre-release (see [`.github/workflows/ci.yml`](.github/workflows/ci.yml)).

| Download | Platform |
|----------|----------|
| [trypanophobe-linux-x64](https://github.com/3p3r/trypanophobe/releases/download/nightly/trypanophobe-linux-x64) | Linux x86_64 |
| [trypanophobe-linux-arm64](https://github.com/3p3r/trypanophobe/releases/download/nightly/trypanophobe-linux-arm64) | Linux ARM64 |
| [trypanophobe-darwin-arm64](https://github.com/3p3r/trypanophobe/releases/download/nightly/trypanophobe-darwin-arm64) | macOS Apple Silicon |
| [trypanophobe-darwin-x64](https://github.com/3p3r/trypanophobe/releases/download/nightly/trypanophobe-darwin-x64) | macOS Intel ([`onnxruntime`](https://formulae.brew.sh/formula/onnxruntime) via Homebrew) |
| [trypanophobe-win32-x64.exe](https://github.com/3p3r/trypanophobe/releases/download/nightly/trypanophobe-win32-x64.exe) | Windows x86_64 |

## License

MIT. The bundled classifier model is Apache-2.0 (Protect AI).
