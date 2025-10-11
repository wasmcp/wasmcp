# Testing Quick Start

## Commands

```bash
make              # Show help
make test         # Run fast tests (unit + integration) ~30s
make test-memory  # Run memory profiling ~1min
make watch        # Auto-run tests on file changes
make clean        # Clean build artifacts and test files
```

## What Changed

### 1. CI Now Runs Tests ✅
- **Protocol unit tests** - 40 tests, runs in ~2s
- **Integration tests** - 15 tests with real WASI streams, runs in ~30s

### 2. Automated Test Data ✅
- Script generates test files on-demand: `./scripts/setup-test-files.sh`
- No more manual `dd` commands
- Files created in `/tmp/wasmcp-test/`

### 3. Developer-Friendly Makefile ✅
- `make` shows help with all commands
- Filters out JSON-RPC noise from test output
- Consistent naming and organization

## Test Categories

| Command | What | Duration | When |
|---------|------|----------|------|
| `make test-unit` | Protocol logic tests | ~2s | Every commit |
| `make test-integration` | Base64 verification + streaming + EOF handling | ~30s | Every push |
| `make test-memory` | Bounded memory (100MB in 2MB, 10MB in 1.5MB) | ~30s | PRs + weekly |

## CI Pipeline

**Every Push:**
- ✅ Lint (format + clippy)
- ✅ Protocol unit tests (NEW)
- ✅ Integration tests (NEW)
- ✅ CLI tests
- ✅ Component builds
- ✅ Template validation (4 languages)

**On PRs:** (Coming soon)
- Memory profiling with results posted as comment

## Quick Test

Verify everything works:

```bash
# Fast tests
make test

# Memory bounded verification (2MB limit)
make test-memory

# Watch mode (requires cargo-watch)
make watch
```

Expected output:
```
Running unit tests...
test result: ok. 40 passed; 0 failed; 0 ignored; 0 measured

Running integration tests...
Test 1: Simple text response
  ✓ Simple text response completed
[... 14 more tests ...]
✓ All 15 protocol streaming integration tests passed!

✅ All tests passed
```

Memory test output:
```
Running memory-bounded tests (2MB limit)...
Testing 10MB file streaming with 2MB memory limit...
[... test output ...]
✓ All 15 protocol streaming integration tests passed!

✅ Memory bounded streaming verified (10MB content in 2MB limit)
```

## Files Added

```
.github/workflows/ci.yml     # Added protocol + integration test jobs
Makefile                      # Redesigned with help system
scripts/
  └── setup-test-files.sh     # Automated test data generation
```

## Next Steps (Deferred)

- [ ] Memory profiling CI workflow (weekly + PR comments)
- [ ] Regression detection baseline
- [ ] Benchmark infrastructure
- [ ] TESTING.md comprehensive guide

Focus: Get the basics solid first, then iterate.
