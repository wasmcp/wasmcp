# Registry Management

Component aliases and composition profiles for efficient reuse.

## Component Aliases

Short names for frequently-used components to simplify composition.

**Add alias:**
```bash
wasmcp registry component add <alias> <path-or-oci>
```

**Examples:**
```bash
# Local file path
wasmcp registry component add calc ./calculator.wasm
wasmcp registry component add calc target/wasm32-wasip2/release/calculator.wasm

# OCI package
wasmcp registry component add strings wasmcp:string-tools@1.0.0
wasmcp registry component add db namespace:database@2.0

# Alias referencing another alias
wasmcp registry component add prod-calc calc
```

**List aliases:**
```bash
wasmcp registry component list
```

**Remove alias:**
```bash
wasmcp registry component remove <alias>
```

**Use in compose:**
```bash
wasmcp compose server calc strings -o server.wasm
```

## Composition Profiles

Save multi-component compositions for reuse.

**Create profile:**
```bash
wasmcp registry profile add <name> <components...> -o <output> [-b <base-profile>]
```

**Examples:**
```bash
# Basic profile
wasmcp registry profile add dev calc strings -o dev-server.wasm

# Profile with inheritance
wasmcp registry profile add prod logger monitor -o prod.wasm -b dev
# prod includes: calc + strings (from dev) + logger + monitor
```

**List profiles:**
```bash
wasmcp registry profile list
```

**Remove profile:**
```bash
wasmcp registry profile remove <name>
```

**Use profile:**
```bash
# Uses profile's output path (~/.config/wasmcp/composed/dev-server.wasm)
wasmcp compose server dev

# Override output path
wasmcp compose server dev -o ./my-server.wasm
```

## Profile Inheritance

Profiles can extend other profiles:

```bash
# Base profile
wasmcp registry profile add base calc strings -o base.wasm

# Dev extends base, adds debug tools
wasmcp registry profile add dev logger debugger -o dev.wasm -b base
# dev = calc + strings + logger + debugger

# Prod extends dev, adds monitoring
wasmcp registry profile add prod monitor alerts -o prod.wasm -b dev
# prod = calc + strings + logger + debugger + monitor + alerts
```

**Inheritance rules:**
- Components from base profile appear first in chain
- Can reference other profiles with `-b <profile-name>`
- No circular dependencies allowed

## Registry Configuration

**Location:** `~/.config/wasmcp/config.toml` (XDG Base Directory compliant)

**Format:**
```toml
# Component aliases
[components]
calc = "/absolute/path/to/calculator.wasm"
strings = "wasmcp:string-tools@1.0.0"
weather = "calc"  # Alias can reference another alias

# Profiles
[profiles.dev]
components = ["calc", "strings"]
output = "dev-server.wasm"

[profiles.prod]
base = "dev"  # Inherit from dev
components = ["monitor", "logger"]
output = "prod-server.wasm"
```

**Composed outputs:** `~/.config/wasmcp/composed/` (when using profiles without `-o` override)

**Downloaded dependencies:** `~/.config/wasmcp/deps/` (cached OCI packages and framework components)

## View Registry

**All information:**
```bash
wasmcp registry info
```

**Components only:**
```bash
wasmcp registry info --components
wasmcp registry info -c
```

**Profiles only:**
```bash
wasmcp registry info --profiles
wasmcp registry info -p
```

## Validation

Registry enforces:
- **Unique names:** Component aliases and profile names cannot conflict
- **No circular dependencies:** Detected in alias chains and profile inheritance
- **Valid identifiers:** Names must be alphanumeric with hyphens/underscores
- **Reserved names:** Cannot use CLI command names (compose, registry, etc.)

## Output Path Behavior

**Explicit `-o` flag:** Always uses current working directory or absolute path
```bash
wasmcp compose server dev -o ./server.wasm  # Creates ./server.wasm
wasmcp compose server dev -o /abs/path/server.wasm  # Creates /abs/path/server.wasm
```

**Profile without `-o`:** Uses `~/.config/wasmcp/composed/{profile-output}`
```bash
wasmcp compose server dev  # Creates ~/.config/wasmcp/composed/dev-server.wasm
```

**No profile, no `-o`:** Uses current working directory
```bash
wasmcp compose server calc strings  # Creates ./mcp-server.wasm
```

## Complete Examples

**Development workflow:**
```bash
# Register components
wasmcp registry component add calc ./calc.wasm
wasmcp registry component add strings ./strings.wasm
wasmcp registry component add logger ./logger.wasm

# Create dev profile
wasmcp registry profile add dev calc strings logger -o dev.wasm

# Use in development
wasmcp compose server dev
wasmtime serve -Scli ~/.config/wasmcp/composed/dev.wasm
```

**Production workflow:**
```bash
# Use OCI packages for prod components
wasmcp registry component add monitor wasmcp:monitor@1.0.0
wasmcp registry component add alerts wasmcp:alerts@2.0.0

# Create prod profile extending dev
wasmcp registry profile add prod monitor alerts -o prod.wasm -b dev

# Deploy to production
wasmcp compose server prod -o /deploy/server.wasm
```

**Quick experiments:**
```bash
# Try new component without registering
wasmcp compose server calc strings ./experimental.wasm -o test.wasm
wasmtime serve -Scli test.wasm
```

## Related Resources

- **Building servers:** See `building-servers` resource for create, build, compose, run workflow
- **CLI reference:** See `reference` resource for detailed command flags
- **Configuration:** Registry config format documented in `reference` resource
