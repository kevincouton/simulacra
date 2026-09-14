//! Simulated-town playground demo.
//!
//! Run with `cargo run -p simulacra-demo` (optional seed argument). The demo
//! synthesizes a town, stress-tests a verifier per shop, lets a cast of
//! heuristic personas run their errands, and prints the binary rewards.
//! `cargo run -p simulacra-demo -- variance [seed]` instead runs the
//! distributional-fidelity experiment: many sampled populations of agents
//! play many towns, and the empirical success-rate distribution is compared
//! against theory. Exits 0 on a successful run.

use simulacra_agents::{OraclePersona, Persona, ShopperPersona};
use simulacra_engine::{Simulation, VarianceExperiment};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|a| a == "variance") {
        let seed = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(42);
        std::process::exit(run_variance(seed));
    }

    let seed: u64 = args.first().and_then(|a| a.parse().ok()).unwrap_or(42);
    std::process::exit(run_town(seed));
}

fn run_town(seed: u64) -> i32 {
    let personas: Vec<Box<dyn Persona>> = vec![
        Box::new(ShopperPersona::new("careful-clara", 0.95, 0.05)),
        Box::new(ShopperPersona::new("distracted-dan", 0.60, 0.20)),
        Box::new(ShopperPersona::new("lazy-lou", 0.50, 0.50)),
        Box::new(OraclePersona),
    ];
    let refs: Vec<&dyn Persona> = personas.iter().map(|p| p.as_ref()).collect();

    println!("simulacra town playground (seed {seed})");
    println!("{} personas enter town\n", refs.len());

    let report = match Simulation::seeded(seed).run_town(5, &refs) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("simulation aborted: {e}");
            return 1;
        }
    };

    for r in report.results() {
        let status = if r.reward == 1.0 { "PASS" } else { "FAIL" };
        println!("[{status}] {:<15} shop {} | {}", r.agent, r.world_id, r.task);
        for v in &r.violations {
            println!("         └─ {v}");
        }
    }

    println!(
        "\n{} attempts, {} passed, success rate {:.0}%, total reward {:.0}",
        report.results().len(),
        report.results().iter().filter(|r| r.reward == 1.0).count(),
        report.success_rate() * 100.0,
        report.total_reward(),
    );
    0
}

fn run_variance(seed: u64) -> i32 {
    println!("variance experiment (seed {seed}): 30 populations x 20 agents x 4 shops");
    let report = match VarianceExperiment::new(seed, 30, 20, 4).run() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("experiment aborted: {e}");
            return 1;
        }
    };

    println!("per-run success rates:");
    for (i, rate) in report.per_run_rates.iter().enumerate() {
        println!("  run {:>2}: {:.1}% {}", i + 1, rate * 100.0, bar(*rate));
    }

    println!("\nmean:      empirical {:.3} vs theoretical {:.3}  [{}]", report.empirical_mean, report.theoretical_mean, verdict(report.mean_match));
    println!("variance:  empirical {:.5} vs theoretical {:.5}  [{}]", report.empirical_variance, report.theoretical_variance, verdict(report.variance_match));
    println!("\n{}", if report.distribution_match() { "DISTRIBUTION MATCH — the population reproduces theory" } else { "DISTRIBUTION MISMATCH — simulated population fails the limits standard" });
    if report.distribution_match() { 0 } else { 1 }
}

fn bar(rate: f64) -> String {
    let filled = (rate * 40.0).round() as usize;
    format!("{}{}", "█".repeat(filled), "░".repeat(40 - filled))
}

fn verdict(ok: bool) -> &'static str {
    if ok { "OK" } else { "MISMATCH" }
}
