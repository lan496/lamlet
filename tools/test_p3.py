"""Exercise the independent oracle's input boundary and optional real-engine admission."""

import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

import p3_oracle


ROOT = Path(__file__).resolve().parents[1]


class P3Tests(unittest.TestCase):
    def test_shell_and_python_commands_produce_unknown_without_execution(self):
        with tempfile.TemporaryDirectory() as temporary:
            folder = Path(temporary)
            marker = folder / "should-not-exist"
            for command in (f"shell touch {marker}", "python callback invoke"):
                with self.subTest(command=command):
                    source = folder / "solution.in"
                    source.write_text(command + "\n")
                    process = subprocess.run([sys.executable, str(ROOT / "tools/p3_oracle.py"),
                        "--task", "P3", "--input", str(source), "--inputs-dir", str(folder)],
                        capture_output=True, text=True, check=True)
                    report = json.loads(process.stdout)
                    self.assertEqual(report["criteria"]["gap"]["status"], "unknown")
                    self.assertIn("unsupported command", report["criteria"]["gap"]["evidence"][0])
                    self.assertFalse(marker.exists())

    def test_dynamic_variables_cannot_be_mistaken_for_static_schedules(self):
        for expression in ("step*10", "time+100", '"extract_setting(nlocal)"', '"py_callback()"'):
            with self.subTest(expression=expression):
                with self.assertRaises(p3_oracle.Unsupported):
                    p3_oracle.commands("variable steps equal " + expression)
        self.assertEqual(p3_oracle.commands("variable steps equal 100/0.001\nrun ${steps}"),
                         ["variable steps equal 100/0.001", "run ${steps}"])

    def test_comments_continuations_and_quoted_arithmetic_preserve_the_command(self):
        self.assertEqual(p3_oracle.commands('variable a equal "3.0 + 0.3079" # Nb\n'
            'fix eq all nvt &\n temp 300 300 0.1\n'),
            ["variable a equal '3.0 + 0.3079'", "fix eq all nvt temp 300 300 0.1"])

    @unittest.skipUnless((ROOT / "target/p3-runtime/runtime.json").is_file(),
                         "Build the optional pinned P3 runtime first")
    def test_reference_and_deliberate_physical_errors_through_real_engine_and_adapter(self):
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / "admission"
            process = subprocess.run([sys.executable, str(ROOT / "tools/p3_admit.py"),
                                      "--output", str(output)], capture_output=True, text=True, timeout=120)
            self.assertEqual(process.returncode, 0, process.stdout + process.stderr)
            report = json.loads((output / "report.json").read_text())
            cases = {case["case"]: case for case in report["cases"]}
            self.assertTrue(report["native_admission_passed"])
            self.assertEqual(cases["reference"]["observed"], "correct")
            self.assertEqual(cases["wrong-gap"]["observed"], "incorrect")
            self.assertEqual(cases["overwritten-thermal-velocity"]["observed"], "incorrect")
            self.assertEqual(cases["unsupported-shell"]["observed"], "unassessed")


if __name__ == "__main__":
    unittest.main()
