"""The menu audit must cover the frozen source inventory without hiding omissions."""
import json
from pathlib import Path
import unittest
ROOT=Path(__file__).resolve().parents[1]
class FrontendAudit(unittest.TestCase):
    def test_every_original_item_has_a_disposition_and_missing_views_stay_explicit(self):
        source=json.loads((ROOT/'docs/baseline/executable/menu-items.json').read_text())
        audit=json.loads((ROOT/'docs/evidence/frontend-menu-audit.json').read_text())
        self.assertEqual(len(source['screens']),audit['inventory_screens'])
        self.assertEqual(sum(len(s['items'])for s in source['screens']),audit['inventory_items'])
        self.assertEqual([s['id']for s in source['screens']],[s['source_screen_id']for s in audit['screens']])
        for original,reviewed in zip(source['screens'],audit['screens']):
            self.assertTrue(reviewed['reviewed'])
            self.assertEqual([(i['index'],i['caption'])for i in original['items']],[(i['index'],i['caption'])for i in reviewed['items']])
            self.assertTrue(all(i['disposition']and i['reason']for i in reviewed['items']))
            self.assertEqual(reviewed['original_visual_match'],'not claimed')
        self.assertTrue(audit['missing_original_screens'])
        self.assertTrue(audit['functional_gameplay_gaps'])
        self.assertFalse(audit['original_runtime_compared'])
if __name__=='__main__':unittest.main()
