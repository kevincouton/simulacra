//! The four playground experiments, each returning a JSON string.
//!
//! Shared by the `/api/*` HTTP routes, the MCP tools, and the A2A agent so
//! every entry point runs exactly the same code.

use std::cell::RefCell;
use std::rc::Rc;

use simulacra_agents::{ExplorerPersona, GossipBook, GossipingPersona, OraclePersona, Persona, ShopperPersona};
use simulacra_engine::{CoEvolution, Simulation, VarianceExperiment};

pub const DEFAULT_SEED: u64 = 42;

pub fn demo_cast() -> Vec<Box<dyn Persona>> {
    vec![
        Box::new(ShopperPersona::new("careful-clara", 0.95, 0.05)),
        Box::new(ShopperPersona::new("distracted-dan", 0.60, 0.20)),
        Box::new(ShopperPersona::new("lazy-lou", 0.50, 0.50)),
        Box::new(OraclePersona),
    ]
}

pub fn as_refs(cast: &[Box<dyn Persona>]) -> Vec<&dyn Persona> {
    cast.iter().map(|p| p.as_ref()).collect()
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

pub fn run_town(seed: u64) -> String {
    let cast = demo_cast();
    let refs = as_refs(&cast);
    let report = Simulation::seeded(seed).run_town(5, &refs).unwrap();
    let results: String = report
        .results()
        .iter()
        .map(|r| {
            let violations: String = r
                .violations
                .iter()
                .map(|v| format!("\"{}\"", esc(v)))
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "{{\"agent\":\"{}\",\"world_id\":{},\"task\":\"{}\",\"reward\":{},\"violations\":[{}]}}",
                esc(&r.agent),
                r.world_id,
                esc(&r.task),
                r.reward,
                violations
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"seed\":{seed},\"results\":[{}],\"success_rate\":{:.4},\"total_reward\":{:.1}}}",
        results,
        report.success_rate(),
        report.total_reward()
    )
}

pub fn run_variance(seed: u64) -> String {
    let report = VarianceExperiment::new(seed, 30, 20, 4).run().unwrap();
    let rates: String = report
        .per_run_rates
        .iter()
        .map(|r| format!("{r:.4}"))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"seed\":{seed},\"per_run_rates\":[{}],\"empirical_mean\":{:.4},\"theoretical_mean\":{:.4},\"empirical_variance\":{:.5},\"theoretical_variance\":{:.5},\"mean_match\":{},\"variance_match\":{}}}",
        rates,
        report.empirical_mean,
        report.theoretical_mean,
        report.empirical_variance,
        report.theoretical_variance,
        report.mean_match,
        report.variance_match
    )
}

pub fn run_coevolution(seed: u64) -> String {
    let personas = vec![
        ShopperPersona::new("careful-clara", 0.95, 0.05),
        ShopperPersona::new("distracted-dan", 0.60, 0.20),
        ShopperPersona::new("lazy-lou", 0.50, 0.50),
        ShopperPersona::new("steady-sue", 0.80, 0.10),
        ShopperPersona::new("hasty-hank", 0.70, 0.30),
        ShopperPersona::new("dreamy-dora", 0.55, 0.35),
    ];
    let refs: Vec<&dyn Persona> = personas.iter().map(|p| p as &dyn Persona).collect();
    let report = CoEvolution::seeded(seed).run(&refs, 8).unwrap();
    let rounds: String = report
        .rounds
        .iter()
        .map(|r| {
            format!(
                "{{\"round\":{},\"n_shops\":{},\"items_per_shop\":{},\"success_rate\":{:.4}}}",
                r.round, r.n_shops, r.items_per_shop, r.success_rate
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("{{\"seed\":{seed},\"rounds\":[{}]}}", rounds)
}

pub fn run_gossip(seed: u64) -> String {
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
    let control_book = Rc::new(RefCell::new(GossipBook::default()));
    let plain = mk(0.0, control_book.clone());
    let plain_refs = as_refs(&plain);
    let without = Simulation::seeded(seed).run_town_hidden(4, &plain_refs, &control_book).unwrap();

    let book = Rc::new(RefCell::new(GossipBook::default()));
    let wired = mk(0.9, book.clone());
    let wired_refs = as_refs(&wired);
    let with = Simulation::seeded(seed).run_town_hidden(4, &wired_refs, &book).unwrap();

    format!(
        "{{\"seed\":{seed},\"no_gossip_rate\":{:.4},\"gossip_rate\":{:.4}}}",
        without.success_rate(),
        with.success_rate()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runners_emit_json() {
        for body in [run_town(42), run_variance(42), run_coevolution(42), run_gossip(42)] {
            crate::json::parse(&body).expect("runner output must be valid JSON");
        }
    }

    #[test]
    fn runners_are_seeded() {
        assert_eq!(run_gossip(42), run_gossip(42));
        assert_ne!(run_gossip(42), run_gossip(7));
    }
}
