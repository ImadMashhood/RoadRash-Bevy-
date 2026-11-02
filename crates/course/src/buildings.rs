//! Shared road-relative building footprint. Rendering and collision use the same openings.
use crate::Sample;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Profile {
    pub height: i32,
    pub depth: i32,
    /// Inclusive/exclusive source progress fractions within one 256-unit sample.
    pub start: i32,
    pub end: i32,
}
impl Profile {
    pub fn resolve(sample: &Sample, side: usize, selector: u8, height: i32) -> Option<Self> {
        if side > 1 || selector == 255 || !(17..=4096).contains(&height) {
            return None;
        }
        let shoulder = sample.shoulder[side].clamp(0, 256);
        let start = if sample.marking_flags & 16 != 0 {
            shoulder
        } else {
            0
        };
        let end = if sample.marking_flags & 8 != 0 {
            256 - shoulder
        } else {
            256
        };
        (start < end).then_some(Self {
            height,
            depth: height / 2,
            start,
            end,
        })
    }
    pub fn lateral_bounds(self, sample: &Sample, side: usize) -> [i32; 2] {
        let outside = sample.width[side] + sample.shoulder[side];
        if side == 0 {
            [-outside - self.depth, -outside]
        } else {
            [outside, outside + self.depth]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn junction_openings_and_solid_bounds_share_one_profile() {
        let mut sample = Sample::road();
        sample.shoulder = [80, 80];
        let p = Profile::resolve(&sample, 1, 0, 320).unwrap();
        assert_eq!(p.lateral_bounds(&sample, 1), [580, 740]);
        assert_eq!(p.lateral_bounds(&sample, 0), [-740, -580]);
        sample.marking_flags = 8;
        assert_eq!(Profile::resolve(&sample, 1, 0, 320).unwrap().end, 176);
        sample.marking_flags = 16;
        assert_eq!(Profile::resolve(&sample, 1, 0, 320).unwrap().start, 80);
        assert!(Profile::resolve(&sample, 1, 255, 320).is_none());
        assert!(Profile::resolve(&sample, 1, 0, 16).is_none());
    }
}
