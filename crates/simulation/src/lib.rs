//! Renderer-independent, integer-only foundation scenario.
//! Constants are authored placeholders, not recovered Road Rash handling.

pub const TICKS_PER_SECOND: u32 = 60;

#[derive(Default, Clone, Copy, Debug)]
pub struct Command {
    pub throttle: bool,
    pub brake: bool,
    pub steer: i8,
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub tick: u64,
    pub speed_mm_per_tick: u32,
    pub distance_mm: u64,
    pub lateral_mm: i32,
}

#[derive(Default)]
pub struct Simulation {
    state: Snapshot,
}

impl Simulation {
    pub fn snapshot(&self) -> Snapshot {
        self.state
    }

    /// One command advances exactly one tick; rendering never supplies a delta.
    pub fn step(&mut self, command: Command) {
        let acceleration = if command.brake {
            -25
        } else if command.throttle {
            8
        } else {
            -3
        };
        self.state.speed_mm_per_tick =
            (i64::from(self.state.speed_mm_per_tick) + acceleration).clamp(0, 1000) as u32;
        self.state.distance_mm = self
            .state
            .distance_mm
            .saturating_add(u64::from(self.state.speed_mm_per_tick));
        let steering = i32::from(command.steer.clamp(-1, 1));
        self.state.lateral_mm = (self.state.lateral_mm
            + steering * (self.state.speed_mm_per_tick / 12) as i32)
            .clamp(-4000, 4000);
        self.state.tick = self.state.tick.saturating_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replay_is_independent_of_presentation_sampling() {
        let mut a = Simulation::default();
        let mut b = Simulation::default();
        for tick in 0..600 {
            let command = Command {
                throttle: tick < 400,
                brake: tick >= 400,
                steer: if tick < 200 { 1 } else { -1 },
            };
            a.step(command);
            if tick % 3 == 0 {
                let _ = a.snapshot();
            }
            b.step(command);
        }
        assert_eq!(a.snapshot(), b.snapshot());
        assert_eq!(a.snapshot().speed_mm_per_tick, 0);
    }
    #[test]
    fn braking_wins_and_bounds_hold_for_extreme_commands() {
        let mut sim = Simulation::default();
        for _ in 0..10_000 {
            sim.step(Command {
                throttle: true,
                brake: false,
                steer: i8::MAX,
            });
        }
        assert_eq!(sim.snapshot().speed_mm_per_tick, 1000);
        assert_eq!(sim.snapshot().lateral_mm, 4000);
        sim.step(Command {
            throttle: true,
            brake: true,
            steer: i8::MIN,
        });
        assert_eq!(sim.snapshot().speed_mm_per_tick, 975);
        assert!(sim.snapshot().lateral_mm < 4000);
    }
}
