#!/usr/bin/env python3
"""校验 conformance/reports 里的源码指纹是否与当前代码一致。"""
import hashlib
import json
import pathlib
import sys

root = pathlib.Path(__file__).resolve().parents[2]


def source_hash() -> str:
    h = hashlib.sha256()
    for path in sorted((root / "rust/crates/lute-core/src").rglob("*.rs")):
        h.update(str(path.relative_to(root)).encode())
        h.update(b"\0")
        h.update(path.read_bytes())
    return h.hexdigest()


def main() -> int:
    current = source_hash()
    print(f"current rust_core_sources_sha256: {current}")
    stale = 0
    for report in sorted((root / "conformance/reports").glob("*.json")):
        data = json.loads(report.read_text())
        recorded = data.get("rust_core_sources_sha256")
        if recorded is None:
            continue
        ok = recorded == current
        stale += 0 if ok else 1
        print(f"  {'current' if ok else 'STALE  '}  {report.name}  {recorded[:12]}")
    if stale:
        print(f"\n{stale} 份报告与当前源码不一致，需要重新生成：")
        print("  LUTE_GO_TEST_REPORT=/tmp/go-original.json cargo test \\")
        print("    --manifest-path rust/Cargo.toml -p lute-core --test go_original_tables")
        print("  python3 rust/tools/summarize_go_tests.py /tmp/go-original.json --workspace-log <log>")
        return 1
    print("\n全部一致")
    return 0


if __name__ == "__main__":
    sys.exit(main())
