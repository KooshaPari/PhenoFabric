#!/usr/bin/env python3
"""
check_json_schemas.py

Validates all JSON schema files under architecture/schemas/.
Exits 0 if all are valid, non-zero otherwise.
"""
import json
import sys
from pathlib import Path

def main():
    schemas_dir = Path(__file__).parent.parent.parent / "architecture" / "schemas"
    errors = []

    if not schemas_dir.exists():
        print(f"WARNING: {schemas_dir} does not exist; skipping JSON schema check")
        return 0

    for schema_file in sorted(schemas_dir.glob("*.json")):
        try:
            with open(schema_file) as f:
                json.load(f)
            print(f"  OK {schema_file.relative_to(Path.cwd())}")
        except json.JSONDecodeError as e:
            errors.append(f"  ERROR {schema_file.relative_to(Path.cwd())}: {e}")
        except Exception as e:
            errors.append(f"  ERROR {schema_file.relative_to(Path.cwd())}: {e}")

    if errors:
        print("\nJSON schema validation failed:")
        for e in errors:
            print(e)
        return 1

    print(f"\nAll JSON schemas valid ({len(list(schemas_dir.glob('*.json')))} files).")
    return 0

if __name__ == "__main__":
    sys.exit(main())
