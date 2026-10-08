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
    match args.first().map(|s| s.as_str()) {
        Some("variance") => {
            let seed = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(42);
            std::process::exit(run_variance(seed));
        }
        Some("coevolution") => {
            let seed = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(42);
            std::process::exit(run_coevolution(seed));
        }
        Some("gossip") => {
            let seed = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(42);
            std::process::exit(run_gossip(seed));
        }
        _ => {
            let seed: u64 = args.first().and_then(|a| a.parse().ok()).unwrap_or(42);
            std::process::exit(run_town(seed));
        }
    }
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

fn run_coevolution(seed: u64) -> i32 {
    let personas: Vec<ShopperPersona> = vec![
        ShopperPersona::new("careful-clara", 0.95, 0.05),
        ShopperPersona::new("distracted-dan", 0.60, 0.20),
        ShopperPersona::new("lazy-lou", 0.50, 0.50),
        ShopperPersona::new("steady-sue", 0.80, 0.10),
        ShopperPersona::new("hasty-hank", 0.70, 0.30),
        ShopperPersona::new("dreamy-dora", 0.55, 0.35),
    ];
    let refs: Vec<&dyn Persona> = personas.iter().map(|p| p as &dyn Persona).collect();

    println!("coevolution (seed {seed}): 8 rounds of the Synthesizer/Solver game");
    let report = match simulacra_engine::CoEvolution::seeded(seed).run(&refs, 8) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("coevolution aborted: {e}");
            return 1;
        }
    };

    for round in &report.rounds {
        println!(
            "  round {}: {:>2} shops x {} items -> success {:.0}% {}",
            round.round,
            round.n_shops,
            round.items_per_shop,
            round.success_rate * 100.0,
            bar(round.success_rate),
        );
    }
    println!("\nthe synthesizer ratchets difficulty up while success > 75%, down below 40%");
    0
}

fn run_gossip(seed: u64) -> i32 {
    use simulacra_agents::{ExplorerPersona, GossipBook, GossipingPersona};
    use std::cell::RefCell;
    use std::rc::Rc;

    let mk = |trust: f64, book: Rc<RefCell<GossipBook>>| -> Vec<Box<dyn Persona>> {
        (0..6)
            .map(|i| {
                let base = ExplorerPersona::new(format!("explorer-{i}"), 0.1);
                if trust > 0.0 {
                    Box::new(GossipingPersona::new(base, book.clone(), trust)) as Box<dyn Persona>
                } else {
                    Box::new(base) as Box<dyn Persona>
                }
            })
            .collect()
    };

    println!("gossip experiment (seed {seed}): hidden locations, 6 explorers, 4 shops");
    let control_book = Rc::new(RefCell::new(GossipBook::default()));
    let plain = mk(0.0, control_book.clone());
    let plain_refs: Vec<&dyn Persona> = plain.iter().map(|p| p.as_ref()).collect();
    let without = match Simulation::seeded(seed).run_town_hidden(4, &plain_refs, &control_book) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("gossip control aborted: {e}");
            return 1;
        }
    };

    let book = Rc::new(RefCell::new(GossipBook::default()));
    let wired = mk(0.9, book.clone());
    let wired_refs: Vec<&dyn Persona> = wired.iter().map(|p| p.as_ref()).collect();
    let with = match Simulation::seeded(seed).run_town_hidden(4, &wired_refs, &book) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("gossip run aborted: {e}");
            return 1;
        }
    };

    println!("  without gossip: success {:.0}%  {}", without.success_rate() * 100.0, bar(without.success_rate()));
    println!("  with gossip:    success {:.0}%  {}", with.success_rate() * 100.0, bar(with.success_rate()));
    println!(
        "\n{} sightings published; gossip lifted success by {:.1} points",
        book.borrow().len(),
        (with.success_rate() - without.success_rate()) * 100.0,
    );
    0
}
