"""Release receipts must fail closed for missing coverage or unmet budgets."""
import copy
import unittest
from verification.performance import common, frame_profile, per_course_profiles, soak, COURSES


def receipt():
    distribution = dict(p50=8., p95=12., p99=20., max=24., count=1000)
    segment = dict(frames=1060, wall_frame_ms=distribution, cpu_raster_ms=copy.deepcopy(distribution),
                   render_diagnostics={'render/main_pass/elapsed_gpu': dict(p50=.2, p95=.4, p99=.5, max=.7, count=1000)})
    return dict(native_only=True, vsync=False, duration_s=3601., simulation_seconds=3595.,
                paused_frames=0, race_and_course_restarts=19, binary_sha256='a'*64,
                actual_window=dict(logical=[960, 720]), level=3, bike=8, first_frame_ms=900., course_loads=[],
                course_segments={f'{c}/lap{lap}': copy.deepcopy(segment) for c in COURSES for lap in [1, 2]},
                memory_samples=[dict(rss='VmRSS: 1000000 kB', gpu_image_bytes=3325980,
                                     cpu_images=8, gpu_images=8, entities=512)],
                decoded_texture_bytes_by_role={'CITY/lap1': {'riders': 123419460}})


class PerformanceReceipts(unittest.TestCase):
    def test_full_evidence_passes_without_treating_index_slots_as_live_entities(self):
        r = receipt()
        self.assertEqual(soak(r)['outcome'], 'passed')
        self.assertEqual(len(frame_profile(r)['rows']), 5)
        self.assertEqual(common(r)['entity_index_slots'], [512])

    def test_elapsed_hour_without_live_simulation_is_rejected(self):
        r = receipt(); r['simulation_seconds'] = 4.
        with self.assertRaisesRegex(ValueError, 'liveness'): soak(r)
        r = receipt(); r['paused_frames'] = 1
        with self.assertRaisesRegex(ValueError, 'unpaused'): soak(r)

    def test_partial_course_coverage_is_rejected(self):
        r = receipt(); del r['course_segments']['NAPA/lap2']
        with self.assertRaisesRegex(ValueError, 'coverage'): frame_profile(r)

    def test_slow_frames_are_not_hidden_by_fast_gpu_passes(self):
        for key, value in [('p95', 18.), ('p99', 23.)]:
            r = receipt(); r['course_segments']['CITY/lap2']['wall_frame_ms'][key] = value
            if key == 'p99':
                r['course_segments']['CITY/lap2']['wall_frame_ms'].update(p99=51., max=60.)
            with self.assertRaisesRegex(ValueError, '60 Hz'): frame_profile(r)

    def test_missing_gpu_or_excessive_residency_is_rejected(self):
        r = receipt(); r['course_segments']['CITY/lap2']['render_diagnostics'] = {}
        with self.assertRaisesRegex(ValueError, 'GPU timestamps'): frame_profile(r)
        r = receipt(); r['memory_samples'][0]['gpu_image_bytes'] = 17*1024**2
        with self.assertRaisesRegex(ValueError, 'residency'): frame_profile(r)

    def test_nonfinite_or_impossible_percentiles_are_rejected(self):
        for value in [float('nan'), -1., 100.]:
            r = receipt(); r['course_segments']['CITY/lap2']['wall_frame_ms']['p50'] = value
            with self.assertRaisesRegex(ValueError, 'Invalid timing'): frame_profile(r)

    def test_short_probe_cannot_replace_five_minutes_per_course(self):
        r = receipt(); r['duration_s'] = 600.
        with self.assertRaisesRegex(ValueError, 'five minutes'): frame_profile(r)

    def independent_courses(self):
        rows = []
        for course in COURSES:
            r = receipt()
            r.update(duration_s=320., simulation_seconds=315.,
                     course_segments={course: r['course_segments'][course+'/lap2']})
            rows.append(r)
        return rows

    def test_independent_five_minute_courses_validate_the_same_binary(self):
        r = self.independent_courses()
        self.assertEqual(per_course_profiles(r)['outcome'], 'passed')
        r[-1]['binary_sha256'] = 'b'*64
        with self.assertRaisesRegex(ValueError, 'same binary'): per_course_profiles(r)

    def test_repeated_or_short_course_does_not_count_as_full_coverage(self):
        r = self.independent_courses(); r[-1] = copy.deepcopy(r[0])
        with self.assertRaisesRegex(ValueError, 'duplicate'): per_course_profiles(r)
        r = self.independent_courses(); r[0]['duration_s'] = 60.
        with self.assertRaisesRegex(ValueError, 'five minutes'): per_course_profiles(r)


    def test_sample_cap_requires_full_duration_same_binary_supplement(self):
        r = receipt(); r['course_segments']['CANYN/lap2']['wall_frame_ms']['count'] = 400
        with self.assertRaisesRegex(ValueError, 'Truncated'): frame_profile(r)
        extra = self.independent_courses()[-1]
        self.assertEqual(frame_profile(r, [extra])['rows'][-1]['duration_s'], 320.)
        extra['binary_sha256'] = 'b'*64
        with self.assertRaisesRegex(ValueError, 'same binary'): frame_profile(r, [extra])
        extra['binary_sha256'] = 'a'*64; extra['actual_window'] = dict(logical=[640, 480])
        with self.assertRaisesRegex(ValueError, 'window'): frame_profile(r, [extra])

    def test_independent_capped_wall_or_cpu_series_is_rejected(self):
        for key in ['wall_frame_ms', 'cpu_raster_ms']:
            with self.subTest(key=key):
                rows = self.independent_courses()
                rows[-1]['course_segments']['CANYN'][key]['count'] = 400
                with self.assertRaisesRegex(ValueError, 'Truncated'):
                    per_course_profiles(rows)

    def test_tour_missing_cpu_coverage_requires_complete_supplement(self):
        r = receipt()
        r['course_segments']['CANYN/lap2']['cpu_raster_ms']['count'] = 400
        with self.assertRaisesRegex(ValueError, 'Truncated'):
            frame_profile(r)
        extra = self.independent_courses()[-1]
        self.assertEqual(frame_profile(r, [extra])['outcome'], 'passed')
        extra['course_segments']['CANYN']['cpu_raster_ms']['count'] = 400
        with self.assertRaisesRegex(ValueError, 'Supplement itself'):
            frame_profile(r, [extra])

    def test_segment_markers_and_aggregate_prefix_have_distinct_scopes(self):
        r = receipt()
        r['aggregate_timing_samples_truncated'] = True
        self.assertEqual(frame_profile(r)['outcome'], 'passed')
        r['course_segments']['CANYN/lap2']['timing_samples_truncated'] = True
        with self.assertRaisesRegex(ValueError, 'truncation marker'):
            frame_profile(r)
        r['course_segments']['CANYN/lap2']['wall_frame_ms']['count'] = 400
        r['course_segments']['CANYN/lap2']['cpu_raster_ms']['count'] = 400
        with self.assertRaisesRegex(ValueError, 'Truncated'):
            frame_profile(r)

    def test_complete_slow_segment_cannot_be_replaced_after_observation(self):
        r = receipt(); extra = self.independent_courses()[-1]
        with self.assertRaisesRegex(ValueError, 'Cannot replace'): frame_profile(r, [extra])
        r['course_segments']['CANYN/lap2']['wall_frame_ms'].update(p95=30., p99=40., max=50.)
        with self.assertRaisesRegex(ValueError, '60 Hz'): frame_profile(r, [extra])


if __name__ == '__main__': unittest.main()
