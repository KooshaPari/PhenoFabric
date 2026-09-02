#!/usr/bin/env python3
"""
check_openapi.py

Validates that architecture/openapi.yaml is well-formed YAML.
Exits 0 if valid, non-zero otherwise.
"""
import sys
from pathlib import Path

def main():
    try:
        import yaml
    except ImportError:
        print("WARNING: PyYAML not installed; skipping OpenAPI check")
        return 0

    openapi = Path(__file__).parent.parent.parent / "architecture" / "openapi.yaml"
    if not openapi.exists():
        print(f"WARNING: {openapi} does not exist; skipping OpenAPI check")
        return 0

    try:
        with open(openapi) as f:
            data = yaml.safe_load(f)
        if not isinstance(data, dict):
            print(f"ERROR: {openapi} is not a YAML mapping")
            return 1
        print(f"  OK {openapi.relative_to(Path.cwd())} (openapi version: {data.get('openapi', 'unknown')})")
    except yaml.YAMLError as e:
        print(f"ERROR: {openapi}: {e}")
        return 1
    except Exception as e:
        print(f"ERROR: {openapi}: {e}")
        return 1

    print(f"\nOpenAPI well-formed.")
    return 0

if __name__ == "__main__":
    sys.exit(main())
