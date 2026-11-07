//! Pure indexed dashboard composition and native-width gauge primitives.
pub(super) fn gap_value(progress: i64) -> i32 {
    let scaled = (progress.unsigned_abs() as u32).wrapping_mul(1000) as i32;
    (scaled >> 4) / 5280
}
pub(super) fn meter_level(current: i32, base: i32) -> i32 {
    if current <= 0 {
        0
    } else if base <= 1 {
        31
    } else {
        (((i64::from(current) * 2 - 2) * 15 / i64::from(base - 1)) + 1).clamp(0, 31) as i32
    }
}
#[allow(clippy::too_many_arguments)]
pub(super) fn blit(
    out: &mut [u8],
    source: &[u8],
    stride: usize,
    palette: &[[u8; 4]],
    rect: [i32; 4],
    at: [i32; 2],
    keyed: bool,
) {
    for y in rect[1]..rect[3] {
        for x in rect[0]..rect[2] {
            let (dx, dy) = (at[0] + x - rect[0], at[1] + y - rect[1]);
            if !(0..640).contains(&dx) || !(0..480).contains(&dy) {
                continue;
            }
            let index = source[y as usize * stride + x as usize] as usize;
            if keyed && index == 0 {
                continue;
            }
            let pixel = &mut out[((dy * 640 + dx) * 4) as usize..][..4];
            pixel.copy_from_slice(&if index == 0 {
                [0, 0, 0, 255]
            } else {
                [palette[index][0], palette[index][1], palette[index][2], 255]
            });
        }
    }
}
pub(super) fn source_sine(table: &[i32], angle: i32) -> i32 {
    let index = (angle as u32 & 1023) as usize;
    match (angle as u32 & 4095) >> 10 {
        0 => table[index] * 2,
        1 => table[1023 - index] * 2,
        2 => table[index] * -2,
        _ => table[1023 - index] * -2,
    }
}
pub(super) fn needle(out: &mut [u8], origin: [i32; 2], end: [i32; 2]) {
    let [mut x, mut y] = origin;
    let dx = (end[0] - x).abs();
    let dy = -(end[1] - y).abs();
    let sx = if x < end[0] { 1 } else { -1 };
    let sy = if y < end[1] { 1 } else { -1 };
    let mut error = dx + dy;
    for _ in 0..2048 {
        for yy in y - 1..=y + 1 {
            for xx in x - 1..=x + 1 {
                if (0..640).contains(&xx) && (0..480).contains(&yy) {
                    out[((yy * 640 + xx) * 4) as usize..][..4]
                        .copy_from_slice(&[0x72, 0x11, 0x08, 255]);
                }
            }
        }
        if [x, y] == end {
            break;
        }
        let twice = 2 * error;
        if twice >= dy {
            error += dy;
            x += sx;
        }
        if twice <= dx {
            error += dx;
            y += sy;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_meter_uses_base_stamina_and_preserves_empty_full_boundaries() {
        assert_eq!(meter_level(0, 2500), 0);
        assert_eq!(meter_level(1, 2500), 1);
        assert_eq!(meter_level(1250, 2500), 15);
        assert_eq!(meter_level(2500, 2500), 31);
        assert_eq!(meter_level(3000, 2500), 31);
        assert_eq!(meter_level(1, 1), 31);
        assert_eq!(gap_value(485), 5);
        assert_eq!(gap_value(-485), 5);
        assert_eq!(gap_value(84480), 1000);
    }
    #[test]
    fn dashboard_holes_are_transparent_but_empty_meter_pixels_are_opaque() {
        let mut pixels = vec![0; 640 * 480 * 4];
        let palette = vec![[0, 0, 255, 0], [12, 34, 56, 0]];
        blit(
            &mut pixels,
            &[0, 1],
            2,
            &palette,
            [0, 0, 2, 1],
            [10, 20],
            true,
        );
        let at = ((20 * 640 + 10) * 4) as usize;
        assert_eq!(&pixels[at..at + 4], &[0, 0, 0, 0]);
        assert_eq!(&pixels[at + 4..at + 8], &[12, 34, 56, 255]);
        blit(
            &mut pixels,
            &[0, 1],
            2,
            &palette,
            [0, 0, 2, 1],
            [10, 20],
            false,
        );
        assert_eq!(&pixels[at..at + 4], &[0, 0, 0, 255]);
    }
}
