"""Keep completed proofs reachable and enhancement production behind native release."""
import json
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[1]


class RoadmapOrder(unittest.TestCase):
    def test_completed_lists_fold_without_hiding_headings_or_unchecking_proofs(self):
        readme = (ROOT / 'README.md').read_text()
        blocks = list(re.finditer(
            r'^### Phase (\d+) — ([^\n]+)\n([\s\S]*?)(?=\n### Phase |\n## 7\.|\Z)',
            readme, re.M))
        self.assertEqual([int(b[1]) for b in blocks], list(range(1, 26)))
        manifest = json.loads((ROOT / 'docs/functional-manifest.json').read_text())
        tasks = {t['id']: t for t in manifest['tasks']}
        self.assertEqual(len(tasks), 48)
        for block in blocks:
            number, body = int(block[1]), block[3]
            completed = number <= 18 or (19 <= number <= 22 and all(
                tasks[f'{number}.{i:02}']['status'] == 'passed' for i in range(1, 13)))
            if completed:
                self.assertEqual(body.count('<details>'), 1)
                self.assertEqual(body.count('</details>'), 1)
                self.assertNotIn('<details open', body)
                self.assertIn('<summary>Completed', body)
                self.assertNotIn('[ ]', body)
                self.assertIn('[x]', body)
                for path in re.findall(r'docs/evidence/\d+\.\d+\.md', body):
                    self.assertTrue((ROOT / path).is_file(), path)
            else:
                self.assertNotIn('<details>', body)
                self.assertEqual(body.count('[ ]') + body.count('[x]'), 48)
                if number >= 23:
                    self.assertNotIn('[x]', body)
            if 19 <= number <= 22:
                for i in range(1, 13):
                    identity = f'{number}.{i:02}'
                    task = tasks[identity]
                    self.assertIn(task['status'], ['pending', 'passed'])
                    mark = 'x' if task['status'] == 'passed' else ' '
                    for suffix in ['', '.a', '.b', '.c']:
                        child_mark = 'x' if suffix.removeprefix('.') in task.get('completed_children', []) else mark
                        self.assertIn(f'- [{child_mark}] **{identity}{suffix} — ', body)
                    if task['status'] == 'passed':
                        self.assertTrue((ROOT / task['evidence']).is_file(), identity)

    def test_future_ticket_mapping_preserves_parents_children_and_functional_gate(self):
        migration = json.loads((ROOT / 'docs/roadmap-migration.json').read_text())
        readme = (ROOT / 'README.md').read_text()
        self.assertEqual(migration['roadmap_revision'], 2)
        self.assertEqual(len(migration['ticket_mapping']), 72)
        old, new = set(), set()
        for row in migration['ticket_mapping']:
            old.add(row['old_id'])
            new.add(row['new_id'])
            for suffix in ['', '.a', '.b', '.c']:
                self.assertIn('**' + row['new_id'] + suffix + ' — ', readme)
        self.assertEqual(len(old), 72)
        self.assertEqual(len(new), 72)
        self.assertIn('### Phase 19 — Complete the native-asset offline game', readme)
        self.assertIn('### Phase 24 — Texture-by-texture enhancement', readme)
        self.assertIn('No AI upscaling or enhanced candidate production begins until G5', readme)
        self.assertIn('19.09 — Complete a genuine native-asset Big Game career', readme)

    def test_native_release_has_no_enhancement_ancestor_and_enhancement_requires_it(self):
        nodes = {n['id']: n for n in json.loads(
            (ROOT / 'docs/baseline/milestones.json').read_text())['milestones']}
        def ancestors(node, active=frozenset()):
            self.assertNotIn(node, active, 'milestone dependency cycle')
            result = set()
            for dependency in nodes[node]['depends_on']:
                self.assertIn(dependency, nodes)
                result.add(dependency)
                result.update(ancestors(dependency, active | {node}))
            return result
        for node in nodes:
            ancestors(node)
        self.assertNotIn('G4', ancestors('G5'))
        self.assertIn('G3F', ancestors('G5'))
        self.assertIn('G5', ancestors('G4'))
        self.assertIn('G5', ancestors('G6'))


if __name__ == '__main__':
    unittest.main()
