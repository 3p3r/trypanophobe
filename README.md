# trypanophobe

Check user prompts for injection attempts before they reach your model. One binary: use it from the command line or over HTTP.

**English prompts only.** Other languages are turned away immediately so you get a clear answer instead of a misleading score.

## Quick start

```bash
cargo build --release
```

### Command line

```bash
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

## License

MIT. The bundled classifier model is Apache-2.0 (Protect AI).
