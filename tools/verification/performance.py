#!/usr/bin/env python3
"""Validate actual native soak/frame receipts against the declared budgets."""
import argparse
import json
import math
from pathlib import Path
import re

COURSES = ['CITY', 'HIWAY', 'MEDLY', 'NAPA', 'CANYN']


def distribution(value):
    ordered = [value[k] for k in ['p50', 'p95', 'p99', 'max']]
    if value['count'] < 1 or any(not math.isfinite(v) or v < 0 for v in ordered) or ordered != sorted(ordered):
        raise ValueError('Invalid timing distribution')


def residency(report):
    if report['native_only'] is not True or report['vsync'] is not False:
        raise ValueError('Expected native-only, vsync-off measurement')
    samples = report['memory_samples']
    rss = max(int(re.search(r'(\d+) kB', s['rss'])[1]) * 1024 for s in samples if s['rss'])
    pixels = max(sum(v.values()) for v in report['decoded_texture_bytes_by_role'].values())
    gpu = max(s['gpu_image_bytes'] for s in samples)
    if rss > 1536 * 1024**2 or pixels > 192 * 1024**2 or gpu > 16 * 1024**2:
        raise ValueError('Native memory/residency budget exceeded')
    if max(s['cpu_images'] for s in samples) > 16 or max(s['gpu_images'] for s in samples) > 16:
        raise ValueError('Application image ownership budget exceeded')
    return dict(max_rss_bytes=rss, max_decoded_pixel_bytes=pixels, max_gpu_image_bytes=gpu,
                max_cpu_images=max(s['cpu_images'] for s in samples),
                max_gpu_images=max(s['gpu_images'] for s in samples),
                entity_index_slots=sorted({s['entities'] for s in samples}))


def common(report):
    for course in COURSES:
        for lap in [1, 2]:
            key = f'{course}/lap{lap}'
            if key not in report['course_segments'] or report['course_segments'][key]['frames'] <= 60:
                raise ValueError('Missing settled course coverage: ' + key)
    return residency(report)


def soak(report):
    memory = common(report)
    if report['duration_s'] < 3600 or report['paused_frames'] != 0:
        raise ValueError('A real unpaused hour is required')
    if report['simulation_seconds'] < report['duration_s'] * .95:
        raise ValueError('Insufficient simulation liveness; elapsed timer alone is not a soak')
    return dict(outcome='passed', duration_s=report['duration_s'], simulation_seconds=report['simulation_seconds'],
                paused_frames=report['paused_frames'], restarts=report['race_and_course_restarts'],
                binary_sha256=report['binary_sha256'], memory=memory)


def segment_statistics(course, segment):
    gpu = {k: v for k, v in segment['render_diagnostics'].items() if k.endswith('/elapsed_gpu')}
    if not gpu:
        raise ValueError('Missing hardware GPU timestamps')
    wall, cpu = segment['wall_frame_ms'], segment['cpu_raster_ms']
    for timing in [wall, cpu, *gpu.values()]:
        distribution(timing)
    if wall['p95'] > 16.7 or wall['p99'] > 50 or cpu['p95'] > 16.67 or any(v['p95'] > 5 for v in gpu.values()):
        raise ValueError('Native 60 Hz frame budget exceeded: ' + course)
    return dict(course=course, wall_frame_ms=wall, cpu_raster_ms=cpu,
                gpu_pass_ms=gpu, meets_60_hz_p95_target=True)


def per_course_profiles(reports):
    rows, courses, identities = [], set(), set()
    for report in reports:
        memory = residency(report)
        if report['duration_s'] < 310:
            raise ValueError('Fresh course run must exceed five minutes plus startup/countdown margin')
        if report['paused_frames'] or report['simulation_seconds'] < report['duration_s'] * .95:
            raise ValueError('Insufficient live course coverage')
        segments = report['course_segments']
        if len(segments) != 1:
            raise ValueError('Expected one independent course per report')
        course, segment = next(iter(segments.items()))
        if course not in COURSES or course in courses or segment['frames'] <= 60:
            raise ValueError('Invalid or duplicate independent course coverage')
        courses.add(course); identities.add(report['binary_sha256'])
        row = segment_statistics(course, segment)
        row.update(memory=memory, duration_s=report['duration_s'], simulation_seconds=report['simulation_seconds'],
                   actual_window=report['actual_window'], first_frame_ms=report['first_frame_ms'],
                   restarts=report['race_and_course_restarts'])
        rows.append(row)
    if courses != set(COURSES) or len(identities) != 1:
        raise ValueError('Five courses of the same binary are required')
    return dict(outcome='passed', binary_sha256=next(iter(identities)), rows=rows)


def frame_profile(report, supplemental=()):
    memory = common(report)
    if report['duration_s'] < 3000:
        raise ValueError('Two-lap tour requires five minutes per course per lap')
    extras = {}
    for extra in supplemental:
        extra_memory = residency(extra)
        if extra['binary_sha256'] != report['binary_sha256'] or extra['duration_s'] < 310:
            raise ValueError('Supplement must use the same binary for over five minutes')
        if any(extra[key] != report[key] for key in ['actual_window', 'level', 'bike']):
            raise ValueError('Supplement must preserve window, level and bike')
        if extra['paused_frames'] or extra['simulation_seconds'] < extra['duration_s'] * .95:
            raise ValueError('Supplement has insufficient live simulation')
        if len(extra['course_segments']) != 1:
            raise ValueError('Supplement must cover one independent course')
        course, segment = next(iter(extra['course_segments'].items()))
        if course not in COURSES or course in extras:
            raise ValueError('Invalid or duplicate supplemental course')
        if segment['wall_frame_ms']['count'] != segment['frames'] - 60:
            raise ValueError('Supplement itself has truncated timing samples')
        extras[course] = (extra, extra_memory)
    rows, used = [], set()
    for course in COURSES:
        segment = report['course_segments'][course + '/lap2']
        if segment['wall_frame_ms']['count'] != segment['frames'] - 60:
            if course not in extras:
                raise ValueError('Truncated timing sample coverage: ' + course)
            extra, extra_memory = extras[course]; used.add(course)
            row = segment_statistics(course, extra['course_segments'][course])
            row.update(sample_scope='Independent full-duration supplement; hour sample reached telemetry cap',
                       duration_s=extra['duration_s'], simulation_seconds=extra['simulation_seconds'],
                       memory=extra_memory, hour_recorded_samples=segment['wall_frame_ms']['count'],
                       hour_total_segment_frames=segment['frames'])
        else:
            row = segment_statistics(course, segment)
            row['sample_scope'] = 'Full warm second-lap course segment'
        rows.append(row)
    if used != set(extras):
        raise ValueError('Cannot replace a complete hour segment with a preferred result')
    return dict(outcome='passed', binary_sha256=report['binary_sha256'], actual_window=report['actual_window'],
                duration_s=report['duration_s'], first_frame_ms=report['first_frame_ms'],
                course_loads=report['course_loads'], memory=memory, rows=rows)


if __name__ == '__main__':
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--soak', type=Path, required=True)
    p.add_argument('--frames', type=Path, required=True)
    p.add_argument('--supplemental-course-report', type=Path, action='append', default=[])
    p.add_argument('--course-report', type=Path, action='append', default=[])
    p.add_argument('--output', type=Path, required=True)
    a = p.parse_args()
    result = dict(outcome='passed', soak=soak(json.loads(a.soak.read_text())),
                  frames=frame_profile(json.loads(a.frames.read_text()), [json.loads(path.read_text()) for path in a.supplemental_course_report]))
    if a.course_report:
        result['delivered_courses'] = per_course_profiles([json.loads(path.read_text()) for path in a.course_report])
    with a.output.open('x') as output:
        json.dump(result, output, indent=2)
        output.write('\n')
