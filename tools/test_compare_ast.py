"""The comparison must not reward early rejection as faster validation."""
import json
import unittest

from compare_ast import lamlet_result, summarize


class ComparisonTests(unittest.TestCase):
    def test_speed_ratio_uses_joint_success_and_retains_failure_denominators(self):
        cases = [
            {"ast": {"accepted": True, "seconds": [0.003, 0.002, 0.004]},
             "lamlet": {"summary": {"errors": 0}, "parse_seconds": [0.0001],
                        "check_seconds": [0.0005], "serialize_seconds": [0.0001]}},
            {"ast": {"accepted": False, "seconds": [0.000001]},
             "lamlet": {"summary": {"errors": 0}}},
            {"ast": {"accepted": False, "timeout_seconds": 10},
             "lamlet": {"summary": {"errors": 1}}},
        ]
        report = summarize(cases)
        self.assertEqual(report["files"], 3)
        self.assertEqual(report["ast_accepted"], 1)
        self.assertEqual(report["ast_timeouts"], 1)
        self.assertEqual(report["lamlet_accepted"], 2)
        self.assertEqual(report["paired_accepted_files"], 1)
        self.assertAlmostEqual(report["ast_over_lamlet_check_json"], 5.0)
        self.assertIsNone(report["scientific_correctness"])
        self.assertIsNone(summarize(cases[1:])["ast_over_lamlet_check_json"])

    def test_full_json_fingerprints_detect_changed_warning_text(self):
        original = {"summary": {"errors": 0}, "diagnostics": [{"message": "one"}]}
        modified = {"summary": {"errors": 0}, "diagnostics": [{"message": "two"}]}
        before = lamlet_result({"report_json": json.dumps(original)})
        after = lamlet_result({"report_json": json.dumps(modified)})
        self.assertEqual(before["summary"], after["summary"])
        self.assertNotEqual(before["report_sha256"], after["report_sha256"])


if __name__ == "__main__":
    unittest.main()
