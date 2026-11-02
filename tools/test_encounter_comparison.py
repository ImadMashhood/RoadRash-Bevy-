"""The native comparison must retain failures and reject incomplete traces."""
import importlib.util
from pathlib import Path
import unittest

SPEC = importlib.util.spec_from_file_location(
    'encounter_comparison', Path(__file__).parent / 'reference/check_encounters.py')
COMPARISON = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(COMPARISON)


class EncounterComparisonTests(unittest.TestCase):
    def test_equal_cases_pass(self):
        result = COMPARISON.compare_cases('contact', [[1, 2], [3, 4]], [[1, 2], [3, 4]])
        self.assertEqual(result['outcome'], 'passed')
        self.assertEqual(result['mismatches'], 0)
        self.assertNotIn('first_divergence', result)

    def test_all_mismatches_count_and_first_counterexample_is_retained(self):
        result = COMPARISON.compare_cases('motion', [[1, 2], [3, 4]], [[1, 9], [3, 8]])
        self.assertEqual(result['outcome'], 'failed')
        self.assertEqual(result['mismatches'], 2)
        self.assertEqual(result['first_divergence'], dict(index=0, native=[1, 2], rust=[1, 9]))

    def test_equal_prefix_cannot_hide_missing_or_extra_ticks(self):
        for native, rust in [([[1], [2]], [[1]]), ([[1]], [[1], [2]])]:
            result = COMPARISON.compare_cases('truncated', native, rust)
            self.assertEqual(result['outcome'], 'failed')
            self.assertEqual(result['mismatches'], 1)
            self.assertEqual(result['first_divergence']['index'], 1)


if __name__ == '__main__':
    unittest.main()
