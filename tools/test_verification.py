import copy
import unittest
from verification.compare import INITIAL_KEYS, compare

class Verification(unittest.TestCase):
    def case(self):
        return {'engine_sha256': 'engine', 'initial': dict.fromkeys(INITIAL_KEYS, 'same'),
                'ticks': [[1, 'a'], [2, 'b']], 'scheduler': [[1, 30]],
                'rng': [[1, 'traffic', 7]], 'motion': [[1, [0, 100]]]}

    def test_every_incompatible_initial_condition_is_rejected(self):
        for key in INITIAL_KEYS:
            a, b = self.case(), self.case()
            b['initial'][key] = 'other'
            with self.assertRaises(ValueError): compare(a, b)
            del b['initial'][key]
            with self.assertRaises(ValueError): compare(a, b)

    def test_first_divergence_and_truncated_trace_are_explicit(self):
        a, b = self.case(), self.case()
        self.assertEqual(compare(a, b)['outcome'], 'equal')
        b['ticks'][1][1] = 'changed'
        b['rng'][0][2] = 8
        b['scheduler'] = []
        report = compare(a, b)
        self.assertEqual(report['first_differences']['ticks']['index'], 1)
        self.assertEqual(report['first_differences']['rng']['index'], 0)
        self.assertEqual(report['first_differences']['scheduler']['index'], 0)
        self.assertIsNone(report['first_differences']['motion'])
        self.assertEqual(report['tolerance']['state'], 0)
