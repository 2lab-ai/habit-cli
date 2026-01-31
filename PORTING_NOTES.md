# PORTING_NOTES

This repo has been ported from a JavaScript MVP implementation to a Rust implementation.

The **CLI contract** is defined by:

- `docs/CLI_REFERENCE.md`
- `docs/MVP_SPEC.md`

## What stayed the same

- Command name: `habit`
- Commands: `add`, `list`, `show`, `archive`, `unarchive`, `checkin`, `status`, `stats`, `export`
- Exit codes: `0` success, `2` validation/usage, `3` not found, `4` ambiguous selector, `5` IO/DB
- Storage: local JSON file (`version: 1`) with deterministic ID generation (`h0001`, `h0002`, ...)
- Determinism hooks: `--db`/`HABITCLI_DB_PATH` and `--today`/`HABITCLI_TODAY`

## Storage notes

- Writes are atomic (write temp file + rename).
- A best-effort advisory lock is implemented via a `db.json.lock` file.
- Default DB path follows the spec: `HABITCLI_DB_PATH` → `XDG_DATA_HOME` → `~/.local/share`.

