"""Allow running wasmcp.build as a module: python -m wasmcp.build"""

from .build import main

if __name__ == "__main__":
    main()