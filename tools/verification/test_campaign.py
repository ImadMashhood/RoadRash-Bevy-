#!/usr/bin/env python3
"""Independent corruption checks; no commercial data or native engine required."""
import copy
import hashlib
import json
from pathlib import Path
import tempfile
import threading
import time
import unittest
from unittest.mock import patch

import campaign


class FinancialTests(unittest.TestCase):
    def setUp(self):
        self.prices = [1000, 3000]
        self.old = {'cash': 3000, 'bike': 0, 'condition': 50, 'maximum_condition': 100}
        self.row = {'bike': 2, 'money_before': 500, 'condition_initial_percent': 100,
                    'shop_transactions': [
                        {'kind': 'purchase', 'from_bike': 0, 'to_bike': 1,
                         'price': 3000, 'trade_in': 500, 'cash_before': 3000, 'cash_after': 500},
                        {'kind': 'repair', 'quote': 0, 'paid': 0, 'cash_before': 500,
                         'cash_after': 500, 'condition_before': 100, 'condition_after': 100}]}

    def test_purchase_trade_in_and_zero_repair(self):
        self.assertEqual(campaign.shop_ledger(self.old, self.row, self.prices),
                         (500, 1, 100, 2500, 0))

    def test_each_transaction_field_is_checked(self):
        for index, transaction in enumerate(self.row['shop_transactions']):
            for key, value in transaction.items():
                with self.subTest(index=index, key=key):
                    broken = copy.deepcopy(self.row)
                    broken['shop_transactions'][index][key] = value + 1 if type(value) is int else 'invalid'
                    with self.assertRaises(AssertionError):
                        campaign.shop_ledger(self.old, broken, self.prices)

    def test_missing_or_duplicate_transaction_rejected(self):
        for tx in ([], self.row['shop_transactions'] * 2):
            with self.assertRaises(AssertionError):
                campaign.shop_ledger(self.old, dict(self.row, shop_transactions=tx), self.prices)

    def test_ceil_repair_and_insufficient_cash(self):
        old = dict(self.old, cash=100, condition=99)
        row = {'bike': 1, 'money_before': 99, 'condition_initial_percent': 100,
               'shop_transactions': [{'kind': 'repair', 'quote': 1, 'paid': 1,
                                     'cash_before': 100, 'cash_after': 99,
                                     'condition_before': 99, 'condition_after': 100}]}
        self.assertEqual(campaign.shop_ledger(old, row, self.prices)[-1], 1)
        old.update(cash=0)
        row.update(money_before=0, condition_initial_percent=99, shop_transactions=[])
        self.assertEqual(campaign.shop_ledger(old, row, self.prices)[-1], 0)

    def settlement(self, outcome='Won', place=1, prize=1000, penalty=0, game_over=False):
        return {'outcome': outcome, 'place': place, 'level': 2,
                'settlement': {'outcome': outcome, 'place': place,
                               'prize': prize, 'penalty': penalty, 'game_over': game_over}}

    def money(self, row, cash=1000):
        return campaign.settlement_money(row, cash, 0, self.prices, [500] * 15, 700)

    def test_prize_and_penalty_boundaries(self):
        self.assertEqual(self.money(self.settlement()), 2000)
        self.assertEqual(self.money(self.settlement('Arrested', 8, 0, 800)), 200)
        self.assertEqual(self.money(self.settlement('Wrecked', 8, 0, 100), 100), 0)
        self.assertEqual(self.money(self.settlement('Wrecked', 8, 0, 100, True), 99), 700)

    def test_financial_receipt_corruption_rejected(self):
        for key in ('outcome', 'place', 'prize', 'penalty', 'game_over'):
            row = self.settlement()
            row['settlement'][key] = 'Lost' if key == 'outcome' else 2 if key == 'place' else True if key == 'game_over' else 99
            with self.subTest(key=key), self.assertRaises(AssertionError):
                self.money(row)

    def test_unknown_outcome_and_impossible_place_rejected(self):
        for outcome, place in [('fake', 1), ('Won', 4), ('Lost', 3), ('Won', 0), ('Lost', 16)]:
            with self.subTest(outcome=outcome, place=place), self.assertRaises(AssertionError):
                self.money(self.settlement(outcome, place))


class ProfileAndSaveTests(unittest.TestCase):
    def profile(self):
        return {'schema_version': 6, 'ticks': 30, 'expected_hash': 'abc', 'commands': [],
                'encounter': {'rules': {'assists': {'fast_recovery': False, 'rescue_unreachable': False},
                                       'combat_version': 4, 'contact_walk_version': 2,
                                       'body': {'recovery_version': 2, 'source_angles': [0] * 257}},
                              'seeds': [{'id': i} for i in range(1, 16)]},
                'population': {'police': {'enabled': True}}}

    def test_current_profile_and_corruptions(self):
        row = {'authority_profile': campaign.AUTHORITY, 'ticks': 30, 'state_sha256': 'abc'}
        campaign.replay_profile(self.profile(), row)
        mutations = [lambda r: r.update(schema_version=5),
                     lambda r: r['encounter']['rules'].update(combat_version=3),
                     lambda r: r['encounter']['rules']['body'].update(recovery_version=1),
                     lambda r: r['encounter']['rules'].update(contact_walk_version=1),
                     lambda r: r['encounter']['rules']['body']['source_angles'].pop(),
                     lambda r: r['encounter']['rules']['assists'].update(fast_recovery=True),
                     lambda r: r['encounter']['seeds'][-1].update(id=1),
                     lambda r: r['population'].update(police=None),
                     lambda r: r.update(ticks=31), lambda r: r.update(expected_hash='bad'),
                     lambda r: r['commands'].append({'input': {'terminal': 'finished'}})]
        for mutate in mutations:
            replay = self.profile()
            mutate(replay)
            with self.assertRaises(AssertionError):
                campaign.replay_profile(replay, row)

    def test_schema_and_checksum_corruption(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'save.json'
            body = {'career': {'cash': 100, 'alias': 'é'}, 'requirements': {}}
            checksum = hashlib.sha256(json.dumps(body, separators=(',', ':'), ensure_ascii=False).encode()).hexdigest()
            saved = {'schema_version': 2, 'body': body, 'sha256': checksum}
            path.write_text(json.dumps(saved))
            self.assertEqual(campaign.saved_career(path), (body['career'], checksum))
            for key, value in [('schema_version', 1), ('sha256', '0' * 64)]:
                path.write_text(json.dumps(dict(saved, **{key: value})))
                with self.assertRaises(AssertionError):
                    campaign.saved_career(path)
            body['career']['cash'] += 1
            path.write_text(json.dumps(saved))
            with self.assertRaises(AssertionError):
                campaign.saved_career(path)


class CarriedDamageTests(unittest.TestCase):
    def setUp(self):
        self.original = {'id': 3, 'condition': 90, 'mass': 250, 'speed': [10, 20]}
        self.row = {'bike': 3, 'condition_capacity': 90, 'condition_initial_percent': 50}
        self.handling = {'selected': 16,
                         'specs': [self.original, dict(self.original, id=16, condition=45)]}

    def test_lawful_unaffordable_repair_uses_scaled_spec_and_original_capacity(self):
        old = {'cash': 0, 'bike': 2, 'condition': 50, 'maximum_condition': 100}
        row = dict(self.row, money_before=0, shop_transactions=[])
        campaign.shop_ledger(old, row, [1000, 2000, 3000])
        spec = campaign.owned_bike_spec(row, self.handling)
        self.assertEqual(spec['condition'], 90)
        self.assertEqual(44 * 100 // spec['condition'], 48)
        for initial, scaled in [(0, 1), (1, 1), (49, 44), (99, 89)]:
            row = dict(self.row, condition_initial_percent=initial)
            handling = {'selected': 16, 'specs': [self.original, dict(self.original, id=16, condition=scaled)]}
            self.assertEqual(campaign.owned_bike_spec(row, handling), self.original)

    def test_only_id_and_exact_condition_can_differ(self):
        for key, value in [('id', 17), ('condition', 46), ('mass', 251), ('speed', [10, 21])]:
            handling = copy.deepcopy(self.handling)
            handling['specs'][1][key] = value
            with self.subTest(key=key), self.assertRaises(AssertionError):
                campaign.owned_bike_spec(self.row, handling)
        for mutate in [lambda h: h.update(selected=3), lambda h: h['specs'].pop(),
                       lambda h: h['specs'].append(copy.deepcopy(h['specs'][1]))]:
            handling = copy.deepcopy(self.handling)
            mutate(handling)
            with self.assertRaises(AssertionError):
                campaign.owned_bike_spec(self.row, handling)
        with self.assertRaises(AssertionError):
            campaign.owned_bike_spec(dict(self.row, condition_capacity=45), self.handling)

    def test_undamaged_player_requires_original_selected_without_derived_spec(self):
        row = dict(self.row, condition_initial_percent=100)
        campaign.owned_bike_spec(row, {'selected': 3, 'specs': [self.original]})
        for handling in [self.handling, {'selected': 16, 'specs': [self.original]},
                         {'selected': 2, 'specs': [self.original]}]:
            with self.assertRaises(AssertionError):
                campaign.owned_bike_spec(row, handling)


class TerminalTests(unittest.TestCase):
    def actor(self, actor_id, completed_at=None, terminal=None):
        return {'id': actor_id, 'encounter': {'completed_at': completed_at,
                                            'recovery': {'outcome': terminal}}}

    def test_native_finish_order_and_same_tick_tie(self):
        actors = [self.actor(1, 50, 'finished'), self.actor(2, 49, 'finished'),
                  self.actor(3, 50, 'finished'), self.actor(4)]
        campaign.terminal_result({'outcome': 'Won', 'place': 2}, actors)
        for row in ({'outcome': 'Won', 'place': 1}, {'outcome': 'Lost', 'place': 2}):
            with self.assertRaises(AssertionError):
                campaign.terminal_result(row, actors)
        actors.extend([self.actor(5, 40, 'finished'), self.actor(6, 41, 'finished')])
        campaign.terminal_result({'outcome': 'Lost', 'place': 4}, actors)
        with self.assertRaises(AssertionError):
            campaign.terminal_result({'outcome': 'Won', 'place': 4}, actors)

    def test_wrong_terminal_and_missing_finish_tick(self):
        for actor in (self.actor(1, 5, 'arrested'), self.actor(1, None, 'finished')):
            with self.assertRaises(AssertionError):
                campaign.terminal_result({'outcome': 'Won', 'place': 1}, [actor])


class PipelineTests(unittest.TestCase):
    def test_bounded_global_prefetch_unique_reports_and_identical_arrays(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            lock = threading.Lock()
            active = peak = 0
            names = set()

            def run(args, **kwargs):
                nonlocal active, peak
                with lock:
                    active += 1
                    peak = max(peak, active)
                    self.assertNotIn(args[3], names)
                    names.add(args[3])
                time.sleep(.02)
                Path(args[3]).write_text(json.dumps({'rate': args[2], 'tick_hashes': [[30, 'exact']]}))
                with lock:
                    active -= 1

            records = [{'replay': f'race-{i}.json'} for i in range(10)]
            with patch.object(campaign.subprocess, 'run', side_effect=run):
                iterator = campaign.replay_reports(records, root, root / 'binary', root, 12)
                first = next(iterator)
                # At first yield only the first three races may be submitted.
                self.assertLessEqual(len(names), 12)
                results = [first, *iterator]
            self.assertEqual([i for i, _ in results], list(range(10)))
            self.assertEqual(len(names), 40)
            self.assertGreater(peak, 4)
            self.assertLessEqual(peak, 12)
            self.assertFalse(list(root.glob('current-replay-*.json')))
            for _, reports in results:
                self.assertEqual([rate for rate, _ in reports], list(campaign.SCHEDULES))
                self.assertTrue(all(report['tick_hashes'] == [[30, 'exact']] for _, report in reports))

    def test_failure_cleans_reports(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            def run(args, **kwargs):
                Path(args[3]).write_text('{}')
                raise RuntimeError('subprocess failed')
            with patch.object(campaign.subprocess, 'run', side_effect=run):
                with self.assertRaisesRegex(RuntimeError, 'subprocess failed'):
                    list(campaign.replay_reports([{'replay': 'a'}] * 10, root, root / 'binary', root, 12))
            self.assertFalse(list(root.glob('current-replay-*.json')))


if __name__ == '__main__':
    unittest.main()
