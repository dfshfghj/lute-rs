#!/usr/bin/env python3
"""Record original Go Test coverage without treating exported tables as full AST tests."""
from collections import Counter
import json
from pathlib import Path
import re

root = Path(__file__).resolve().parents[2]
fixture = json.loads((root / 'rust/crates/lute-core/tests/fixtures/go_original.json').read_text())
tests_dir = root / 'rust/crates/lute-core/tests'
tabs = (tests_dir / 'go_tabs_tests.rs').read_text()
helper_tests = {c['go_test'] for c in json.loads((tests_dir / 'fixtures/go_helpers.json').read_text())}
helper_tests.update(['TestLexerNormalizeLines', 'TestEscapeAttrVal', 'TestEscapeHTMLStr', 'TestEscapeString', 'TestHTML2TreeEscapedText', 'TestHTML2MarkdownEscapedText', 'TestSanitize'])
parse_ports = {
    'TestMergeIALPreservingOrder': 'src/parse/ial.rs',
    'TestFileAnnotationRefRejectsNestedOpenersWithoutConsumingInput': 'src/parse/inline/misc.rs',
    'TestTableCandidateCompatibility': 'tests/go_parse_tests.rs + src/parse/table.rs',
    'TestAutoLinkCandidateCompatibility': 'tests/go_parse_tests.rs',
    'TestTaskListProbeCompatibility': 'tests/go_parse_tests.rs',
    'TestReferenceIndexCompatibility': 'src/parse/inline/index.rs',
    'TestLinkLabelBoundaries': 'src/parse/link.rs',
}
def snake(name):
    return re.sub(r'([a-z0-9])([A-Z])', r'\1_\2', re.sub(r'([A-Z]+)([A-Z][a-z])', r'\1_\2', name)).lower().replace('_i_ds', '_ids')
# Explicit source annotations preserve original function identity even when Rust
# splits a Go test into an integration assertion and private metadata assertions.
manual_ports = {}
paths = list(tests_dir.glob('go_*.rs')) + list((root / 'rust/crates/lute-core/src').rglob('*.rs'))
for path in paths:
    text = path.read_text()
    for match in re.finditer(r'^\s*//+\s*Go:\s*([^:\s]+)::(Test\w+)', text, re.M):
        following = re.search(r'\bfn\s+(\w+)\s*\(', text[match.end():])
        if following:
            manual_ports.setdefault((match[1], match[2]), []).append(str(path.relative_to(root / 'rust/crates/lute-core')) + '::' + following[1])
gaps = json.loads((root / 'conformance/go-test-migration-gaps.json').read_text())
gap_map = {}
for gap in gaps:
    gap_map.setdefault((gap['source'], gap['test']), []).append(gap)
review_path = root / 'conformance/reports/go-table-assertion-review.json'
review = {(t['source'], t['test']):t for t in json.loads(review_path.read_text())['tests']} if review_path.exists() else {}
entries = []
for original in fixture['tests']:
    entry = {'source': original['source'], 'test': original['test'], 'tables': original.get('tables', [])}
    name = original['test']
    entry['status'] = 'unported_non_table_assertions'
    if original['tables']:
        entry['status'] = 'table_contract_ported_additional_assertions_pending'
        entry['rust'] = 'tests/go_original_tables.rs'
    if name == 'TestProtyleExportMd':
        entry.update(status='ported_structural_assertions', rust='tests/go_export_md.rs', reason='Original parse + non-hybrid flatten + export renderer chain; all 20 rows asserted')
    if name == 'TestInline':
        entry.update(status='ported_ast_assertions', rust='tests/go_inline_ast.rs')
    if name == 'TestSpec':
        entry.update(status='ported_html_spec_652_cases', rust='tests/go_commonmark_html.rs')
    if name in parse_ports and original['source'].startswith('parse/'):
        entry.update(status='ported_function_assertions', rust=parse_ports[name])
    if name in helper_tests and not original['source'].startswith('parse/'):
        entry.update(status='ported_helper_assertions', rust='tests/go_helpers_tests.rs')
    if original['source'].startswith('test/tabs'):
        if 'fn go_' + snake(name) + '(' in tabs:
            entry.update(status='ported_tabs_assertions', rust='tests/go_tabs_tests.rs')
        if name == 'TestTabsOriginalTitleBlock':
            entry.update(status='partially_ported_tabs_assertions', rust='tests/go_tabs_tests.rs', reason='PDF and standalone ProtyleExportRenderer assertions unavailable')
        if name == 'TestTabsExportNestedTitleBlocks':
            entry.update(status='unavailable_pdf_renderer', reason='ProtyleExportRenderer is not implemented in Rust')
    if original['source'].startswith('parse/html_text_style_test.go'):
        entry.update(status='unavailable_style_functions', reason='HTMLTextStyle / ResolveHTMLTextStyle / HTMLStyleDeclarations have no Rust equivalents')
    if original['source'].startswith('util/'):
        entry.update(status='go_runtime_specific', reason='Go unsafe byte/string aliasing and GC behavior; Rust ownership tests cover Rust lifetime guarantees, not this Go API')
    if original['source'].startswith('conformance/'):
        entry.update(status='go_harness_specific', reason='Original Go harness tests continue under go test ./...')
    key = (original['source'], name)
    if key in review and review[key]['status'] == 'table_only_assertions' and entry['status'] == 'table_contract_ported_additional_assertions_pending':
        entry['status'] = 'ported_table_assertions_verified'
        entry['assertion_review'] = 'go-table-assertion-review.json'
    if key in manual_ports:
        entry.update(status='ported_structural_assertions', rust=sorted(set(manual_ports[key])))
    if key in gap_map:
        entry['gaps'] = gap_map[key]
        unavailable = next((gap for gap in gap_map[key] if gap['status'] != 'partial'), None)
        if unavailable:
            entry['status'] = {'go_runtime':'go_runtime_specific','native_go_chroma':'native_go_chroma_specific'}.get(unavailable['status'],'unavailable_implementation')
        else:
            entry['status'] = 'partially_ported_assertions'
    entries.append(entry)
result = {
    'description': 'Migration inventory. Table contracts do not imply all original function assertions were ported. Missing areas are not marked ignored or passing.',
    'build_profiles': ['native-no-chroma', 'javascript'],
    'omitted_capabilities': ['chroma'],
    'profile_expectations': 'rust/crates/lute-core/tests/fixtures/build_profiles.json',
    'original_test_functions': len(entries),
    'status_counts': dict(sorted(Counter(e['status'] for e in entries).items())),
    'tests': entries,
}
path = root / 'conformance/reports/go-test-coverage.json'
path.parent.mkdir(parents=True, exist_ok=True)
path.write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n')
print(json.dumps(result['status_counts'], ensure_ascii=False))
