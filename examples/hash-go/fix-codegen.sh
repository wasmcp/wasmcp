#!/bin/bash
# Workaround for wit-bindgen-go bug with option<borrow<resource>> in cross-interface records
# See BUG_REPORT.md for details
# This script fixes the generated abi.go file after running go generate

set -e

ABI_FILE="gen/wasmcp/mcp/tools-capability/abi.go"

if [ ! -f "$ABI_FILE" ]; then
    echo "Error: $ABI_FILE not found. Run 'go generate' first."
    exit 1
fi

echo "Applying codegen fix to $ABI_FILE..."

# Fix the lift_OptionBorrowOutputStream function return type and implementation
# Before: func lift_OptionBorrowOutputStream(f0 uint32, f1 uint32) (v cm.Option[cm.Rep])
# After:  func lift_OptionBorrowOutputStream(f0 uint32, f1 uint32) (v cm.Option[protocol.OutputStream])
sed -i \
    -e 's/func lift_OptionBorrowOutputStream(f0 uint32, f1 uint32) (v cm\.Option\[cm\.Rep\])/func lift_OptionBorrowOutputStream(f0 uint32, f1 uint32) (v cm.Option[protocol.OutputStream])/' \
    -e 's/return (cm\.Option\[cm\.Rep\])(cm\.Some\[cm\.Rep\](cm\.Reinterpret\[cm\.Rep\]((uint32)(f1))))/return (cm.Option[protocol.OutputStream])(cm.Some[protocol.OutputStream](cm.Reinterpret[protocol.OutputStream]((uint32)(f1))))/' \
    "$ABI_FILE"

echo "✓ Codegen fix applied successfully"
