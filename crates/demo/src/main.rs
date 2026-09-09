//! Simulated-town playground demo.
//!
//! Run with `cargo run -p simulacra-demo` (optional seed argument). The demo
//! synthesizes a town, stress-tests a verifier per shop, lets a cast of
//! heuristic personas run their errands, and prints the binary rewards.
//! Exits 0 on a successful run.

use simulacra_agents::{OraclePersona, Persona, ShopperPersona};
use simulacra_engine::Simulation;

fn main() {
    let seed: u64 = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(42);

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
            std::process::exit(1);
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
}
