"""Validate authored fixture contracts and milestone acyclicity."""
import json
import struct
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]

class FoundationContracts(unittest.TestCase):
    def test_authored_examples_have_known_answers(self):
        fixture = json.loads((ROOT / 'docs/fixtures/authored.json').read_text())
        image = fixture['indexed_image']
        self.assertEqual(len(image['indices']), image['width'] * image['height'])
        self.assertEqual(image['indices'], [0, 255])
        container = fixture['resource_container']
        self.assertEqual(container['magic'], 'CRSR')
        self.assertEqual(bytes.fromhex(container['entries'][0]['payload_hex']), bytes([1, 2, 3, 4]))
        encoded = bytes.fromhex(container['bytes_hex'])
        self.assertEqual(encoded[:4], b'CRSR')
        directory = struct.unpack_from('<I', encoded, 16)[0]
        self.assertEqual(directory, container['directory_offset'])
        self.assertEqual(struct.unpack_from('<I', encoded, directory+8)[0], 1)
        entry_id, offset, length = struct.unpack_from('<III', encoded, directory+20)
        self.assertEqual(entry_id, 1)
        self.assertEqual(encoded[offset:offset+length], bytes([1, 2, 3, 4]))
        graph = fixture['route_graph']
        nodes = {n['id']: n['next'] for n in graph['nodes']}
        visited = set()
        def walk(node):
            if node in visited:
                return
            visited.add(node)
            for child in nodes[node]:
                walk(child)
        walk(graph['root'])
        self.assertEqual(len(visited), graph['expected_reachable'])
        self.assertEqual([n for n in visited if not nodes[n]], graph['expected_terminals'])

    def test_milestone_dependencies_are_acyclic_and_owned(self):
        data = json.loads((ROOT / 'docs/baseline/milestones.json').read_text())
        nodes = {n['id']: n for n in data['milestones']}
        visited, active = set(), set()
        def visit(node):
            self.assertNotIn(node, active, 'circular dependency')
            if node in visited:
                return
            active.add(node)
            self.assertTrue(nodes[node]['owner'])
            for dependency in nodes[node]['depends_on']:
                self.assertIn(dependency, nodes)
                visit(dependency)
            active.remove(node)
            visited.add(node)
        for node in nodes:
            visit(node)

class EvidenceContract(unittest.TestCase):
    def test_unknown_claims_remain_unverified(self):
        ledger = json.loads((ROOT / 'docs/baseline/evidence-ledger.json').read_text())
        labels = {'verified-local', 'upstream-reported', 'inferred', 'unknown'}
        for entry in ledger['entries']:
            self.assertIn(entry['confidence'], labels)
            self.assertEqual(len(entry['executable_sha256']), 64)
            self.assertEqual(len(entry['upstream_revision']), 40)
            self.assertIn('status', entry['capture_settings'])
        unknown = next(e for e in ledger['entries'] if e['id'] == 'original-input')
        self.assertEqual(unknown['confidence'], 'unknown')

class ActorInventory(unittest.TestCase):
    def test_all_selectable_actor_ids_resolve_unique_complete_records(self):
        actors = json.loads((ROOT / 'docs/baseline/executable/selectable-actors.json').read_text())
        paths = {f['path'] for f in json.loads((ROOT / 'docs/baseline/required-files.json').read_text())['files']}
        self.assertEqual([b['bike_id'] for b in actors['bikes']], list(range(15)))
        for bike in actors['bikes']:
            self.assertEqual(bike['spec_resource_id'], bike['bike_id']+1)
            self.assertIn(bike['source_image'], paths)
            self.assertTrue(bike['display_name'])
            self.assertGreater(bike['price'], 0)
            self.assertTrue(bike['eligibility'])
        self.assertEqual([c['character_id'] for c in actors['characters']], list(range(1,9)))
        for character in actors['characters']:
            self.assertTrue(character['display_name'])
            self.assertTrue(character['eligibility'])
            self.assertEqual(len(character['bike_ids_by_level']), 5)
            self.assertTrue(all(0 <= bike < 15 for bike in character['bike_ids_by_level']))
            for image in character['source_images']:
                self.assertIn(image, paths)
