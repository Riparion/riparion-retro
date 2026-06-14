//! `sim` — the kaintuck shared-economy CLI harness.
//!
//! Runs a world of bot traders against the shared market and writes a metrics
//! report (JSON or CSV), plus a wall-clock throughput line for the perf goal.
//!
//! ```text
//! cargo run -p kaintuck-engine --features cli --bin sim -- \
//!     --bots 32 --ticks 5000 --policy mixed --format json --out report.json
//! ```

use std::process::ExitCode;
use std::time::Instant;

use kaintuck_engine::market::MarketParams;
use kaintuck_engine::sim::{run_sim, PolicyKind, SimConfig};

/// Built from the actual `SimConfig`/`MarketParams` defaults so the advertised
/// values can't drift from what the harness really uses.
fn usage() -> String {
    let d = SimConfig::default();
    let p = MarketParams::default();
    format!(
        "Usage: sim [options]
  --bots N         number of bot traders          (default {bots})
  --seed N         base RNG seed                   (default {seed})
  --ticks N        ticks to run                    (default {ticks})
  --dt F           time advanced per tick          (default {dt})
  --policy KIND    greedy | cautious | random | mixed   (default {policy})
  --relax F        market relax rate (mean-reversion)   (default {relax})
  --impact F       price move per depth-unit traded     (default {impact})
  --depth F        trade depth in units                 (default {depth:.0})
  --burst          single-journey burst (no bot respawn; default: sustained)
  --format FMT     json | csv                      (default json)
  --out PATH       write report to PATH            (default: stdout)
  -h, --help       show this help
",
        bots = d.bots,
        seed = d.base_seed,
        ticks = d.ticks,
        dt = d.dt,
        policy = format!("{:?}", d.policy).to_lowercase(),
        relax = p.relax_rate,
        impact = p.impact,
        depth = p.depth_units,
    )
}

fn main() -> ExitCode {
    let mut cfg = SimConfig::default();
    let mut format = String::from("json");
    let mut out: Option<String> = None;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut next = |flag: &str| -> Result<String, ()> {
            args.next().ok_or(()).map_err(|_| {
                eprintln!("missing value for {flag}");
            })
        };
        match arg.as_str() {
            "-h" | "--help" => {
                print!("{}", usage());
                return ExitCode::SUCCESS;
            }
            "--bots" => match next("--bots").ok().and_then(|v| v.parse().ok()) {
                Some(n) => cfg.bots = n,
                None => return bad("--bots expects a non-negative integer"),
            },
            "--seed" => match next("--seed").ok().and_then(|v| v.parse().ok()) {
                Some(n) => cfg.base_seed = n,
                None => return bad("--seed expects an integer"),
            },
            "--ticks" => match next("--ticks").ok().and_then(|v| v.parse().ok()) {
                Some(n) => cfg.ticks = n,
                None => return bad("--ticks expects a non-negative integer"),
            },
            "--dt" => match next("--dt").ok().and_then(|v| v.parse().ok()) {
                Some(f) => cfg.dt = f,
                None => return bad("--dt expects a number"),
            },
            "--policy" => match next("--policy").ok().and_then(|v| PolicyKind::parse(&v)) {
                Some(p) => cfg.policy = p,
                None => return bad("--policy expects greedy|cautious|random|mixed"),
            },
            "--relax" => match next("--relax").ok().and_then(|v| v.parse().ok()) {
                Some(f) => cfg.params.relax_rate = f,
                None => return bad("--relax expects a number"),
            },
            "--impact" => match next("--impact").ok().and_then(|v| v.parse().ok()) {
                Some(f) => cfg.params.impact = f,
                None => return bad("--impact expects a number"),
            },
            "--depth" => match next("--depth").ok().and_then(|v| v.parse().ok()) {
                Some(f) => cfg.params.depth_units = f,
                None => return bad("--depth expects a number"),
            },
            "--burst" => cfg.sustained = false,
            "--format" => match next("--format").ok() {
                Some(f) if f == "json" || f == "csv" => format = f,
                _ => return bad("--format expects json|csv"),
            },
            "--out" => match next("--out").ok() {
                Some(p) => out = Some(p),
                None => return bad("--out expects a path"),
            },
            other => return bad(&format!("unknown argument {other:?}\n\n{}", usage())),
        }
    }

    let started = Instant::now();
    let report = run_sim(&cfg);
    let elapsed = started.elapsed();

    let body = match format.as_str() {
        "csv" => report.to_csv(),
        _ => report.to_json(),
    };

    match &out {
        Some(path) => {
            if let Err(e) = std::fs::write(path, &body) {
                return bad(&format!("could not write {path}: {e}"));
            }
            eprintln!("wrote {} report to {path}", format);
        }
        None => println!("{body}"),
    }

    // Summary line (always to stderr so it never pollutes a piped report).
    let steps = report.ticks.saturating_mul(report.bots as u64);
    let secs = elapsed.as_secs_f64().max(1e-9);
    eprintln!(
        "relax={:.3} impact={:.3} depth={:.0} | dispersion {:.1}% (peak-max {:.0}%)  inflation {:.3}  near-clamp {} | vol {:.0} | wins {} losses {} | {:.0} bot-steps/s",
        report.relax_rate,
        report.impact,
        report.depth_units,
        report.steady_dispersion * 100.0,
        report.peak_max_dispersion * 100.0,
        report.steady_inflation,
        report.near_clamp,
        report.total_volume,
        report.wins,
        report.losses,
        steps as f64 / secs,
    );

    ExitCode::SUCCESS
}

fn bad(msg: &str) -> ExitCode {
    eprintln!("sim: {msg}");
    ExitCode::FAILURE
}
