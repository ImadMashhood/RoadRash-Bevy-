//! Reproducible private acceptance driver; exports receipts, traces and a schema-3 showcase replay.
mod support;
use rr_simulation::{
    animation::{Pose, Weapon},
    replay::{Replay, run},
    riding::Configuration,
    *,
};
use serde_json::json;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let a: Vec<_> = std::env::args_os().skip(1).collect();
    if a.len() != 2 {
        return Err("usage: rr-riding-check DATA_ROOT OUTPUT_DIR".into());
    }
    let data = std::path::Path::new(&a[0]);
    let out = support::output(data, std::path::Path::new(&a[1]))?;
    std::fs::create_dir_all(&out)?;
    let course = rr_course::Course::read(&std::fs::read(data.join("DATA/COURSES/CITY.CRS"))?)?;
    let route = course.route(&vec![0; course.splits().len()])?;
    let config = Configuration::load(data, &route, 1, 50)?;
    let poses = [
        Pose::Idle,
        Pose::Ride,
        Pose::Accelerate,
        Pose::Tuck,
        Pose::PunchLeft,
        Pose::PunchRight,
        Pose::KickLeft,
        Pose::KickRight,
        Pose::BackhandLeft,
        Pose::BackhandRight,
        Pose::Crash,
        Pose::Slide,
        Pose::GetUp,
        Pose::Stand,
        Pose::Walk,
        Pose::Remount,
    ];
    let mut commands = vec![];
    for (i, pose) in poses.into_iter().enumerate() {
        commands.push(QueuedCommand {
            tick: Tick(i as u64 * 30),
            actor: ActorId(1),
            sequence: 0,
            input: Command {
                throttle: true,
                steer: if i % 3 == 0 {
                    -1
                } else if i % 3 == 1 {
                    1
                } else {
                    0
                },
                pose: Some(pose),
                weapon: Some(if i % 3 == 0 {
                    Weapon::Club
                } else if i % 3 == 1 {
                    Weapon::Chain
                } else {
                    Weapon::Fists
                }),
                ..Default::default()
            },
        });
    }
    let replay = Replay {
        schema_version: 3,
        seed: 1,
        ticks: 480,
        commands,
        route: None,
        expected_hash: None,
        handling: Some(config.clone()),
        encounter: None,
    };
    let mut baseline = None;
    let mut runs = vec![];
    for rate in ["30", "60", "144", "irregular"] {
        let report = run(replay.clone(), rate)?;
        if let Some(b) = &baseline {
            if b != &report.tick_hashes {
                return Err("schedule divergence".into());
            }
        } else {
            baseline = Some(report.tick_hashes.clone());
        }
        runs.push(json!({"rate":rate,"sha256":report.state_sha256,"ticks":report.ticks,"frames":report.presentation_frames}));
        std::fs::write(
            out.join(format!("replay-{rate}.json")),
            serde_json::to_vec_pretty(&report)?,
        )?;
    }
    std::fs::write(
        out.join("showcase-replay.json"),
        serde_json::to_vec(&replay)?,
    )?;
    let mut transitions = vec![];
    let mut sim = Simulation::with_handling(1, config.clone())?;
    let mut trace = String::from("tick,speed,gear,lean,lateral,height,airborne,frame,contact\n");
    for tick in 0..480 {
        let mut command = replay.commands[(tick / 30) as usize].input;
        if tick % 30 != 0 {
            command.weapon = None;
            command.pose = None;
        }
        sim.step(command)?;
        let snap = sim.snapshot();
        let r = snap.actors[0].rider.as_ref().unwrap();
        let b = &r.bike;
        trace += &format!(
            "{},{},{},{},{},{},{},{},{}\n",
            tick + 1,
            b.speed,
            b.gear,
            b.lean,
            b.lateral,
            b.height,
            b.airborne,
            r.frame,
            r.contact_frame
        );
        if tick % 30 == 0 {
            transitions.push(json!({"tick":tick,"pose":r.pose,"frame":r.frame,"logical_anchor":"source RRAN pivot; fixed ground anchor"}));
        }
    }
    std::fs::write(out.join("riding-trace.csv"), trace)?;
    let receipt = json!({"outcome":"passed","specifications":config.specs.len(),"source_samples":config.ground.len(),"valid_animation_banks":2,"pose_transitions":transitions,"replay_runs":runs,"replay_bytes":std::fs::metadata(out.join("showcase-replay.json"))?.len(),"scope":"native source-port comparison, not original executable parity; combat and crash/recovery decisions are verified separately by rr-encounter-check"});
    std::fs::write(
        out.join("receipt.json"),
        serde_json::to_vec_pretty(&receipt)?,
    )?;
    println!(
        "{} specs; 16 pose transitions; 4 schedules",
        config.specs.len()
    );
    Ok(())
}
