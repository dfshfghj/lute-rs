"""Protect strict migration report counts and rejection of incomplete runs."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).with_name('summarize_go_tests.py')


class SummaryTests(unittest.TestCase):
    def summarize(self, log):
        with tempfile.TemporaryDirectory() as directory:
            directory = Path(directory)
            report = directory / 'report.json'
            workspace = directory / 'workspace.log'
            output = directory / 'summary.json'
            report.write_text(json.dumps({'checked': 2, 'failures': [], 'unsupported_drivers': []}))
            workspace.write_text(log)
            result = subprocess.run(
                [sys.executable, str(SCRIPT), str(report), '--workspace-log', str(workspace), '--out', str(output)],
                capture_output=True, text=True, check=False,
            )
            return result, json.loads(output.read_text()) if output.exists() else None

    def test_counts_passes_inside_failed_targets_and_document_tests(self):
        result, output = self.summarize('''Running tests/example.rs (target/example)
test first ... FAILED
test result: FAILED. 3 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
Running tests/other.rs (target/other)
test second ... FAILED
test third ... FAILED
test result: FAILED. 4 passed; 2 failed; 1 ignored; 0 measured; 0 filtered out
Doc-tests lute_core
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
''')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output['workspace_passed_test_functions'], 9)
        self.assertEqual(output['workspace_failed_test_functions'], 3)
        self.assertEqual(output['workspace_ignored_test_functions'], 1)
        self.assertEqual(output['additional_failed_test_count'], 3)
        self.assertEqual(output['additional_failed_tests'][0]['target'], 'tests/example.rs')

    def test_compile_failure_does_not_emit_a_partial_parity_report(self):
        result, output = self.summarize('error[E0308]: mismatched types\nerror: could not compile lute-core\n')
        self.assertNotEqual(result.returncode, 0)
        self.assertIsNone(output)
        self.assertIn('failed to compile', result.stderr)


if __name__ == '__main__':
    unittest.main()
