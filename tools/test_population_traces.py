import json
from pathlib import Path
import tempfile
import unittest
from compare_population_traces import compare

class PopulationTraces(unittest.TestCase):
    def test_each_channel_reports_its_first_difference(self):
        for channel in ['equal','decisions','motion','rng']:
            with self.subTest(channel=channel), tempfile.TemporaryDirectory() as root:
                a,b=Path(root)/'a',Path(root)/'b';a.mkdir();b.mkdir()
                base=[{'tick':300,'decisions':[[2,75,300]],'motion':[[2,1000,0]]}]
                changed=json.loads(json.dumps(base))
                if channel in ['decisions','motion']:changed[0][channel][0][-1]+=1
                for d,data in [(a,base),(b,changed)]:
                    (d/'behavior-trace.json').write_text(json.dumps(data))
                    (d/'rng-trace.json').write_text(json.dumps([[100,2,'attack_gate',8 if d==b and channel=='rng' else 7]]))
                report=compare(a,b)
                self.assertEqual(report['outcome'],'equal' if channel=='equal' else 'different')
                self.assertEqual(report['scheduler'] is None, channel!='decisions')
                self.assertEqual(report['rng'] is None, channel!='rng')
                self.assertEqual(report['motion_arithmetic_candidate'] is None, channel!='motion')
