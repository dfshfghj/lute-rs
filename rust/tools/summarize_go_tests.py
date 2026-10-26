#!/usr/bin/env python3
"""Summarize a strict original-Go test run; this file never changes expected values."""
import argparse
from collections import Counter
import hashlib
import json
import re
from pathlib import Path

root = Path(__file__).resolve().parents[2]
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('report', type=Path)
p.add_argument('--workspace-log', type=Path, help='cargo test --workspace --no-fail-fast output')
p.add_argument('--out', type=Path, default=root / 'conformance/reports/go-original-rust.json')
args = p.parse_args()
report = json.loads(args.report.read_text())
fixture = root / 'rust/crates/lute-core/tests/fixtures/go_original.json'
source_hash = hashlib.sha256()
for path in sorted((root / 'rust/crates/lute-core/src').rglob('*.rs')):
    source_hash.update(str(path.relative_to(root)).encode())
    source_hash.update(b'\0')
    source_hash.update(path.read_bytes())
failures = []
for case in report['failures']:
    expected = case['expected']
    actual = case['actual'].get('value')
    kind = 'output_difference'
    if actual is None:
        kind = 'rust_error'
    elif case['source'] == 'test/code_syntax_highlight_test.go' or 'highlight-' in expected or 'class="chroma"' in expected or (case['source'], case['test'], case['case']) in {('test/debug_test.go', 'TestDebug', '54'), ('test/kramdown_block_ial_test.go', 'TestKramdownBlockIALIDName', '4')}:
        kind = 'native_go_chroma_contract'
    offset = None
    if actual is not None:
        offset = next((i for i, (a, b) in enumerate(zip(expected, actual)) if a != b), min(len(expected), len(actual)))
    failure = {key: case[key] for key in ['source', 'test', 'table', 'case', 'api']}
    for index_key in ['driver_index', 'case_index']:
        if index_key in case:
            failure[index_key] = case[index_key]
    failure['kind'] = kind
    failure['input_sha256'] = hashlib.sha256(case['input'].encode()).hexdigest()
    failure['first_difference_char'] = offset
    if offset is not None:
        start = max(0, offset - 50)
        failure['expected_context'] = expected[start:offset + 130]
        failure['actual_context'] = actual[start:offset + 130]
    else:
        failure['actual_error'] = case['actual']
    failures.append(failure)
output = {
    'description': 'Observed failures, not an allowlist. The strict Rust test still fails on every mismatch and unavailable driver.',
    'command': 'LUTE_GO_TEST_REPORT=/tmp/go-original.json cargo test --manifest-path rust/Cargo.toml -p lute-core --test go_original_tables' + (' --features javascript' if report.get('build_profile') == 'javascript' else ''),
    'fixture_sha256': hashlib.sha256(fixture.read_bytes()).hexdigest(),
    'rust_core_sources_sha256': source_hash.hexdigest(),
    'build_profile': report.get('build_profile', 'legacy-native-go-reference'),
    'omitted_capabilities': report.get('omitted_capabilities', []),
    'reference_differences': report.get('reference_differences', []),
    'checked': report['checked'],
    'failed': len(failures),
    'passed': report['checked'] - len(failures),
    'by_kind': dict(sorted(Counter(f['kind'] for f in failures).items())),
    'by_api': dict(sorted(Counter(f['api'] for f in failures).items())),
    'unsupported_drivers': report['unsupported_drivers'],
    'failures': failures,
}
if 'build_profile' in report:
    profile_path = root / 'rust/crates/lute-core/tests/fixtures/build_profiles.json'
    profiles = json.loads(profile_path.read_text())
    if profiles['original_fixture_sha256'] != output['fixture_sha256']:
        raise SystemExit('source build profiles do not match the original fixture; regenerate references')
    output['build_profiles_sha256'] = hashlib.sha256(profile_path.read_bytes()).hexdigest()
if args.workspace_log:
    log = args.workspace_log.read_text()
    if 'error[E' in log or 'could not compile' in log:
        raise SystemExit('workspace failed to compile; do not report a partial test run as parity evidence')
    origins = {}
    for path in list((root / 'rust/crates/lute-core/tests').glob('go_*.rs')) + list((root / 'rust/crates/lute-core/src').rglob('*.rs')):
        text = path.read_text()
        patterns = [r'^\s*//+\s*Go:\s*([^:\s]+)::(Test\w+)', r'^\s*//+\s*Go\s+([^:\s]+):\s*(Test\w+)']
        for pattern in patterns:
            for match in re.finditer(pattern, text, re.M):
                function = re.search(r'\bfn\s+(\w+)\s*\(', text[match.end():])
                if function:
                    origin = {'source': match[1], 'test': match[2], 'rust': str(path.relative_to(root))}
                    if origin not in origins.setdefault(function[1], []):
                        origins[function[1]].append(origin)
    failed = []
    target = ''
    for line in log.splitlines():
        if 'Running unittests ' in line:
            target = 'lute-core --lib' if 'lute_core-' in line else 'lute-wasm --lib'
        elif match := re.search(r'Running (tests/[^ ]+\.rs)', line):
            target = match[1]
        elif match := re.match(r'^test (\S+) \.\.\. FAILED$', line):
            name = match[1]
            if name == 'original_go_table_contracts':
                continue  # Every table case is already enumerated above.
            section = re.search(r'^---- ' + re.escape(name) + r' stdout ----\n(.*?)(?=^---- |^failures:|^test result:|\Z)', log, re.M | re.S)
            detail = section[1].strip()[:1800] if section else ''
            detail = detail.replace(str(root) + '/', '')
            detail = re.sub(r"(thread '[^']+' \()\d+(\))", r'\1thread\2', detail)
            failed.append({'target':target,'rust_test':name,'origins':origins.get(name.split('::')[-1],[]),'detail':detail})
    output['additional_failed_tests'] = failed
    output['additional_failed_test_count'] = len(failed)
    output['workspace_passed_test_functions'] = sum(map(int,re.findall(r'test result: (?:ok|FAILED)\. (\d+) passed',log)))
    output['workspace_failed_test_functions'] = sum(map(int,re.findall(r'test result: FAILED\. \d+ passed; (\d+) failed',log)))
    output['workspace_ignored_test_functions'] = sum(map(int,re.findall(r'test result: (?:ok|FAILED)\. \d+ passed; \d+ failed; (\d+) ignored',log)))
args.out.parent.mkdir(parents=True, exist_ok=True)
args.out.write_text(json.dumps(output, ensure_ascii=False, indent=2) + '\n')
print(f"{output['passed']}/{output['checked']} passed; {output['failed']} differences; {len(output['unsupported_drivers'])} unavailable drivers")
print(args.out)
