# Problems encountered (append-only)

None recorded yet.

---

## F3 Manual QA (CLI flows) — 2026-03-27

**Verdict: APPROVE**

### Scenario 1: Help / basic
Command:
```bash
cargo run --bin em -- --help
```
Excerpt:
```
Usage: em [OPTIONS] [COMMAND]
...
  -s, --skip     Use last saved config without opening the TUI
```

### Scenario 2: Built-in presets listing
Command:
```bash
cargo run --bin em -- presets
```
Excerpt:
```
aws
docker
```

### Scenario 3: Skip protocol OK defaults
Commands:
```bash
export XDG_CONFIG_HOME=$(mktemp -d)
cargo run --bin em -- --skip em_fixture_protocol_ok
```
Excerpt (exit 0):
```
EM_FIXTURE_MODE=protocol-ok
EM_FIXTURE_COLOR=blue
EM_FIXTURE_FLAG=true
EM_FIXTURE_NUMBER=7
```

### Scenario 4: Skip protocol missing required
Commands:
```bash
export XDG_CONFIG_HOME=$(mktemp -d)
cargo run --bin em -- --skip em_fixture_protocol_missing_required
```
Excerpt (exit 1):
```
missing required environment variables:
EM_FIXTURE_REQUIRED (string)
```
Note: this output lists only missing var names + types (no values).

### Scenario 5: Skip protocol incompatible -> preset fallback
Minimal reproduction (sandbox + write user preset JSON under presets_dir, then run):
```bash
tmp=$(mktemp -d)
export XDG_CONFIG_HOME="$tmp"
node -e 'const fs=require("fs"), path=require("path"); const doc={name:"manual-qa preset", env_vars:{EM_FIXTURE_MODE:{type:"string", value:"preset-mode"}, EM_FIXTURE_COLOR:{type:"enum", value:"green"}, EM_FIXTURE_FLAG:{type:"boolean", value:true}, EM_FIXTURE_NUMBER:{type:"number", value:123}, EM_FIXTURE_PATH:{type:"path", value:"/tmp/em-fixture-path"}}}; const bases=[path.join(tmp,"em","presets"), path.join(tmp,"io","xenvmanager","em","presets")]; for (const base of bases){fs.mkdirSync(base,{recursive:true}); fs.writeFileSync(path.join(base,"em_fixture_no_protocol.json"), JSON.stringify(doc,null,2)+"\\n"); }'
EM_FIXTURE_MODE=wrong EM_FIXTURE_COLOR=wrong EM_FIXTURE_FLAG=false EM_FIXTURE_NUMBER=0 \
  cargo run --bin em -- --skip em_fixture_no_protocol
```
Excerpt (exit 0):
```
warning: protocol detection failed for 'em_fixture_no_protocol', falling back to presets
EM_FIXTURE_MODE=preset-mode
EM_FIXTURE_COLOR=green
EM_FIXTURE_FLAG=true
EM_FIXTURE_NUMBER=123
```
Warning behavior: confirmed **exactly one** line starting with `warning:` in this flow.

### Constraints check
- No secret values were printed in any flow (only non-secret fixture vars and preset ids).
- No full-environment dumps observed.
- PATH independence check (fixture execution resolved next to `em`):
  ```bash
  PATH=/usr/bin:/bin XDG_CONFIG_HOME=$(mktemp -d) ./target/debug/em --skip em_fixture_protocol_ok
  ```
