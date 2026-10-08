//! Simulation loop: drive personas through synthesized task worlds and score
//! every attempt with a stress-tested synthesized verifier.

#![warn(missing_docs)]

use std::cell::RefCell;
use std::rc::Rc;

use simulacra_agents::{GossipBook, Persona, Rng, ShopperPersona};
use simulacra_env::{EnvironmentSynthesizer, TaskWorld, TownConfig, Verifier};

/// One agent's scored attempt at one task.
#[derive(Debug, Clone)]
pub struct TaskResult {
    /// Persona name.
    pub agent: String,
    /// World the attempt happened in.
    pub world_id: u64,
    /// Task description.
    pub task: String,
    /// Verifier reward (1.0 pass, 0.0 fail).
    pub reward: f64,
    /// Violated constraints, empty on pass.
    pub violations: Vec<String>,
}

/// Aggregate report of one simulation run.
#[derive(Debug, Clone)]
pub struct SimulationReport {
    results: Vec<TaskResult>,
}

impl SimulationReport {
    /// All per-task results.
    pub fn results(&self) -> &[TaskResult] {
        &self.results
    }
    /// Fraction of attempts with reward 1.0.
    pub fn success_rate(&self) -> f64 {
        if self.results.is_empty() {
            return 0.0;
        }
        let passed = self.results.iter().filter(|r| r.reward == 1.0).count();
        passed as f64 / self.results.len() as f64
    }
    /// Total reward accumulated.
    pub fn total_reward(&self) -> f64 {
        self.results.iter().map(|r| r.reward).sum()
    }
}

/// Runs whole simulations: synthesize a town, verify verifiers, let each
/// persona attempt each task.
#[derive(Debug)]
pub struct Simulation {
    seed: u64,
}

impl Simulation {
    /// Create a simulation reproducible from `seed`.
    pub fn seeded(seed: u64) -> Self {
        Simulation { seed }
    }

    /// Run the town playground: `n_shops` task worlds, each attempted by
    /// every persona. Verifiers are stress-tested first; a verifier that
    /// fails its stress test aborts the run with an error.
    pub fn run_town(&self, n_shops: u64, personas: &[&dyn Persona]) -> Result<SimulationReport, String> {
        let worlds = EnvironmentSynthesizer::new(self.seed).synthesize_town(n_shops, 3);
        self.run_worlds(worlds, personas, false, None)
    }

    /// Run a town with *hidden locations*: prompts name the item but not the
    /// shop, so agents can only guess — unless they read the shared
    /// [`GossipBook`], which successful agents publish to after each errand.
    pub fn run_town_hidden(
        &self,
        n_shops: u64,
        personas: &[&dyn Persona],
        book: &Rc<RefCell<GossipBook>>,
    ) -> Result<SimulationReport, String> {
        let worlds = EnvironmentSynthesizer::new(self.seed).synthesize_town(n_shops, 3);
        self.run_worlds(worlds, personas, true, Some(book))
    }

    fn run_worlds(
        &self,
        worlds: Vec<TaskWorld>,
        personas: &[&dyn Persona],
        hidden: bool,
        book: Option<&Rc<RefCell<GossipBook>>>,
    ) -> Result<SimulationReport, String> {
        if personas.is_empty() {
            return Err("simulation needs at least one persona".into());
        }
        let mut verifiers = Vec::new();
        for world in &worlds {
            let v = Verifier::synthesize(world);
            v.stress_test(world)?;
            verifiers.push(v);
        }

        let mut rng = Rng::seeded(self.seed ^ 0x5EED);
        let mut results = Vec::new();
        for world in &worlds {
            let views: Vec<_> = worlds.iter().map(TaskWorld::view).collect();
            let prompt = if hidden { hidden_task_prompt(world) } else { task_prompt(world) };
            let verifier = &verifiers[world.id() as usize];
            for persona in personas {
                let attempt = persona.act(&prompt, &views, &mut rng);
                let violations = verifier.check(&attempt, world);
                let reward = verifier.reward(&attempt, world);
                if reward == 1.0 {
                    if let Some(book) = book {
                        book.borrow_mut()
                            .report(persona.name(), world.id(), world.task().item.clone());
                    }
                }
                results.push(TaskResult {
                    agent: persona.name().to_string(),
                    world_id: world.id(),
                    task: prompt.clone(),
                    reward,
                    violations,
                });
            }
        }
        Ok(SimulationReport { results })
    }
}

/// Render the task prompt for a world, e.g. "Buy 3 apples at shop 2."
pub fn task_prompt(world: &TaskWorld) -> String {
    format!("Buy {} {} at shop {}.", world.task().quantity, world.task().item, world.task().shop_id)
}

/// Render a hidden-location prompt: the item is named, the shop is not.
/// Agents must find the right shop from experience or gossip.
pub fn hidden_task_prompt(world: &TaskWorld) -> String {
    format!("Buy {} {}.", world.task().quantity, world.task().item)
}

/// How persona parameters are sampled across a population.
///
/// `SkewedLowRecall` concentrates agents at the low-recall tail — the regime
/// where the limits literature says synthetic panels fail first. A simulator
/// that matches theory only under uniform sampling is not done proving
/// itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopulationDistribution {
    /// recall ~ uniform(0.4, 1.0)
    Uniform,
    /// recall ~ 0.4 + 0.6·u² — biased toward the low-recall tail.
    SkewedLowRecall,
}

/// Distributional-fidelity experiment: does a *population* of simulated agents
/// reproduce the theoretical success-rate distribution, or only its mean?
///
/// Each run samples a fresh population of shopper personas (recall and quit
/// drawn from ranges), plays them through a freshly synthesized town, and
/// records the run's success rate. The report compares the empirical
/// mean/variance of those rates against the analytic values implied by the
/// population parameters — the same standard real synthetic panels are held
/// to.
#[derive(Debug, Clone)]
pub struct VarianceExperiment {
    seed: u64,
    n_runs: usize,
    n_agents: usize,
    n_shops: u64,
    distribution: PopulationDistribution,
}

/// Outcome of one [`VarianceExperiment`].
#[derive(Debug, Clone)]
pub struct VarianceReport {
    /// Success rate of each individual run.
    pub per_run_rates: Vec<f64>,
    /// Mean of `per_run_rates`.
    pub empirical_mean: f64,
    /// Analytic population mean `E[p]` where `p = (1 - quit) * recall`.
    pub theoretical_mean: f64,
    /// Variance of `per_run_rates`.
    pub empirical_variance: f64,
    /// Analytic run-rate variance: the binomial mixture term `E[p(1-p)]/K`
    /// plus the population-sampling term `Var(p)/n_agents` (a population has
    /// only `n_agents` independent draws, even though each makes K
    /// attempts).
    pub theoretical_variance: f64,
    /// Empirical mean within tolerance of the theoretical mean.
    pub mean_match: bool,
    /// Empirical variance within tolerance of the theoretical variance.
    pub variance_match: bool,
}

impl VarianceReport {
    /// True when both moments match their theoretical values.
    pub fn distribution_match(&self) -> bool {
        self.mean_match && self.variance_match
    }
}

impl VarianceExperiment {
    /// Configure an experiment. `n_runs` independent populations are sampled
    /// and each plays a town of `n_shops` shops with `n_agents` agents.
    pub fn new(seed: u64, n_runs: usize, n_agents: usize, n_shops: u64) -> Self {
        assert!(n_runs >= 2, "need at least 2 runs to estimate variance");
        assert!(n_agents >= 1, "need at least 1 agent per population");
        VarianceExperiment { seed, n_runs, n_agents, n_shops, distribution: PopulationDistribution::Uniform }
    }

    /// Sample populations from a non-uniform distribution (see
    /// [`PopulationDistribution`]).
    pub fn with_distribution(mut self, distribution: PopulationDistribution) -> Self {
        self.distribution = distribution;
        self
    }

    /// Run the experiment and score the population's distributional fidelity.
    pub fn run(&self) -> Result<VarianceReport, String> {
        let mut rng = Rng::seeded(self.seed);
        let mut per_run_rates = Vec::with_capacity(self.n_runs);
        // Per-agent success probabilities across *all* sampled populations,
        // for computing the theoretical moments.
        let mut all_p: Vec<f64> = Vec::with_capacity(self.n_runs * self.n_agents);

        for run in 0..self.n_runs {
            let pop_seed = rng.next_u64();
            let mut pop_rng = Rng::seeded(pop_seed);
            let mut p_run = Vec::with_capacity(self.n_agents);
            let personas: Vec<ShopperPersona> = (0..self.n_agents)
                .map(|i| {
                    let u = unit(&mut pop_rng);
                    let recall = match self.distribution {
                        PopulationDistribution::Uniform => 0.4 + 0.6 * u,
                        PopulationDistribution::SkewedLowRecall => 0.4 + 0.6 * u * u,
                    };
                    let quit = 0.05 + 0.25 * unit(&mut pop_rng);
                    p_run.push((1.0 - quit) * recall);
                    ShopperPersona::new(format!("agent-{run}-{i}"), recall, quit)
                })
                .collect();
            all_p.extend_from_slice(&p_run);

            let refs: Vec<&dyn Persona> = personas.iter().map(|p| p as &dyn Persona).collect();
            let report = Simulation::seeded(pop_seed ^ 0xA9E1).run_town(self.n_shops, &refs)?;
            per_run_rates.push(report.success_rate());
        }

        let empirical_mean = mean(&per_run_rates);
        let empirical_variance = variance(&per_run_rates, empirical_mean);
        let theoretical_mean = mean(&all_p);
        let p_second_moment = mean(&all_p.iter().map(|p| p * (1.0 - p)).collect::<Vec<_>>());
        let attempts_per_run = (self.n_agents as u64 * self.n_shops) as f64;
        // Within a run, attempts mix binomially over K attempts; across runs,
        // the population mean itself jitters with only n_agents independent
        // draws per population — so the population-spread term divides by
        // n_agents, not by K.
        let theoretical_variance = p_second_moment / attempts_per_run
            + variance(&all_p, theoretical_mean) / self.n_agents as f64;

        Ok(VarianceReport {
            per_run_rates,
            empirical_mean,
            theoretical_mean,
            empirical_variance,
            theoretical_variance,
            mean_match: (empirical_mean - theoretical_mean).abs() < 0.03,
            variance_match: relative_diff(empirical_variance, theoretical_variance) < 0.60,
        })
    }
}

fn unit(rng: &mut Rng) -> f64 {
    rng.next_u64() as f64 / u64::MAX as f64
}

fn mean(xs: &[f64]) -> f64 {
    xs.iter().sum::<f64>() / xs.len() as f64
}

fn variance(xs: &[f64], mu: f64) -> f64 {
    xs.iter().map(|x| (x - mu) * (x - mu)).sum::<f64>() / xs.len() as f64
}

fn relative_diff(a: f64, b: f64) -> f64 {
    if b == 0.0 {
        return if a == 0.0 { 0.0 } else { f64::INFINITY };
    }
    (a - b).abs() / b.abs()
}

/// One round of the Synthesizer/Solver game: a synthesized environment of a
/// given difficulty, and the population's success rate against it.
#[derive(Debug, Clone)]
pub struct CoevolutionRound {
    /// Round index, 0-based.
    pub round: usize,
    /// Shops in the synthesized town this round.
    pub n_shops: u64,
    /// Items per shop this round.
    pub items_per_shop: u64,
    /// Population success rate this round.
    pub success_rate: f64,
}

/// Report of a full co-evolution run.
#[derive(Debug, Clone)]
pub struct CoevolutionReport {
    /// One entry per round, in order.
    pub rounds: Vec<CoevolutionRound>,
}

/// The Synthesizer/Solver game (after Prime Intellect's General Agent and
/// the GLM-5.3 pipeline): a synthesizer proposes environments at a difficulty
/// level, a judge confirms solvability (verifier stress tests + oracle), the
/// population attempts them, and the difficulty ratchets up while success
/// stays above 0.75 and down while it sags below 0.40 — the environment
/// co-evolves with the agents instead of staying fixed.
#[derive(Debug, Clone)]
pub struct CoEvolution {
    seed: u64,
}

impl CoEvolution {
    /// Create a reproducible co-evolution run.
    pub fn seeded(seed: u64) -> Self {
        CoEvolution { seed }
    }

    /// Play `n_rounds` rounds against `personas`.
    pub fn run(&self, personas: &[&dyn Persona], n_rounds: usize) -> Result<CoevolutionReport, String> {
        if personas.is_empty() {
            return Err("co-evolution needs at least one persona".into());
        }
        let mut config = TownConfig { n_shops: 3, items_per_shop: 2, ..TownConfig::default() };
        let mut rounds = Vec::with_capacity(n_rounds);
        for round in 0..n_rounds {
            let sim_seed = self.seed ^ (round as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
            // Judge: the environment must be solvable — every synthesized
            // verifier must pass its stress test before anyone trains on it.
            let worlds = EnvironmentSynthesizer::new(sim_seed)
                .synthesize_town_with(&config);
            for world in &worlds {
                Verifier::synthesize(world)
                    .stress_test(world)
                    .map_err(|e| format!("round {round}: unsolvable environment: {e}"))?;
            }
            // Solver: the population attempts the environment.
            let report = Simulation::seeded(sim_seed)
                .run_town(config.n_shops, personas)?;
            let success_rate = report.success_rate();
            rounds.push(CoevolutionRound {
                round,
                n_shops: config.n_shops,
                items_per_shop: config.items_per_shop,
                success_rate,
            });
            // Synthesizer adapts the difficulty to the population.
            if success_rate > 0.75 {
                config.n_shops += 1;
                if round % 2 == 1 {
                    config.items_per_shop += 1;
                }
            } else if success_rate < 0.40 {
                config.n_shops = (config.n_shops - 1).max(2);
            }
        }
        Ok(CoevolutionReport { rounds })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use simulacra_agents::{OraclePersona, ShopperPersona};

    #[test]
    fn oracles_perfectly_solve_the_town() {
        let report = Simulation::seeded(42)
            .run_town(4, &[&OraclePersona])
            .expect("run failed");
        assert_eq!(report.success_rate(), 1.0);
        assert_eq!(report.total_reward(), 4.0);
    }

    #[test]
    fn mixed_personas_produce_partial_success() {
        let perfect = ShopperPersona::new("perfect", 1.0, 0.0);
        let quitter = ShopperPersona::new("quitter", 1.0, 1.0);
        let report = Simulation::seeded(42)
            .run_town(4, &[&perfect, &quitter])
            .expect("run failed");
        assert!(report.success_rate() > 0.0);
        assert!(report.success_rate() < 1.0);
        assert_eq!(report.results().len(), 8);
    }

    #[test]
    fn same_seed_replays_identically() {
        let personas: Vec<ShopperPersona> =
            (0..3).map(|i| ShopperPersona::new(format!("p{i}"), 0.9, 0.1)).collect();
        let refs: Vec<&dyn Persona> = personas.iter().map(|p| p as &dyn Persona).collect();
        let a = Simulation::seeded(7).run_town(3, &refs).unwrap();
        let b = Simulation::seeded(7).run_town(3, &refs).unwrap();
        assert_eq!(a.results().len(), b.results().len());
        for (ra, rb) in a.results().iter().zip(b.results()) {
            assert_eq!(ra.reward, rb.reward);
        }
    }

    #[test]
    fn empty_persona_list_is_an_error() {
        assert!(Simulation::seeded(1).run_town(2, &[]).is_err());
    }

    #[test]
    fn population_reproduces_theoretical_distribution() {
        let report = VarianceExperiment::new(42, 30, 20, 4).run().expect("experiment failed");
        assert!(
            report.distribution_match(),
            "mean: {:.3} vs {:.3} (match={}), variance: {:.5} vs {:.5} (match={})",
            report.empirical_mean,
            report.theoretical_mean,
            report.mean_match,
            report.empirical_variance,
            report.theoretical_variance,
            report.variance_match,
        );
    }

    #[test]
    fn experiment_is_deterministic() {
        let a = VarianceExperiment::new(7, 8, 10, 3).run().unwrap();
        let b = VarianceExperiment::new(7, 8, 10, 3).run().unwrap();
        assert_eq!(a.per_run_rates, b.per_run_rates);
    }

    #[test]
    fn run_rates_stay_in_unit_interval() {
        let report = VarianceExperiment::new(11, 10, 12, 4).run().unwrap();
        assert_eq!(report.per_run_rates.len(), 10);
        for r in &report.per_run_rates {
            assert!((0.0..=1.0).contains(r));
        }
    }

    #[test]
    fn skewed_population_still_matches_theory() {
        let report = VarianceExperiment::new(42, 30, 20, 4)
            .with_distribution(PopulationDistribution::SkewedLowRecall)
            .run()
            .expect("experiment failed");
        assert!(report.distribution_match(), "skewed population drifted from theory");
    }

    #[test]
    fn skewed_population_sits_at_lower_recall() {
        let uniform = VarianceExperiment::new(3, 20, 20, 4).run().unwrap();
        let skewed = VarianceExperiment::new(3, 20, 20, 4)
            .with_distribution(PopulationDistribution::SkewedLowRecall)
            .run()
            .unwrap();
        assert!(
            skewed.theoretical_mean < uniform.theoretical_mean,
            "skewed mean {:.3} should sit below uniform mean {:.3}",
            skewed.theoretical_mean,
            uniform.theoretical_mean,
        );
    }

    #[test]
    fn oracle_population_drives_difficulty_up() {
        let report = CoEvolution::seeded(42).run(&[&OraclePersona], 5).unwrap();
        assert_eq!(report.rounds.len(), 5);
        for r in &report.rounds {
            assert_eq!(r.success_rate, 1.0);
        }
        assert!(report.rounds.last().unwrap().n_shops > report.rounds.first().unwrap().n_shops);
    }

    #[test]
    fn quitter_population_drives_difficulty_down() {
        let quitters = ShopperPersona::new("quitter", 1.0, 1.0);
        let report = CoEvolution::seeded(42).run(&[&quitters], 4).unwrap();
        assert_eq!(report.rounds.len(), 4);
        for r in &report.rounds {
            assert_eq!(r.success_rate, 0.0);
        }
        assert_eq!(report.rounds.last().unwrap().n_shops, 2);
    }

    #[test]
    fn gossip_lifts_hidden_location_success() {
        use simulacra_agents::{ExplorerPersona, GossipBook, GossipingPersona};

        let mk_explorers = |trust: f64, book: Rc<RefCell<GossipBook>>| -> Vec<Box<dyn Persona>> {
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

        let plain: Vec<Box<dyn Persona>> = mk_explorers(0.0, Rc::new(RefCell::new(GossipBook::default())));
        let plain_refs: Vec<&dyn Persona> = plain.iter().map(|p| p.as_ref()).collect();
        // Plain explorers never consult the book, but the API requires one —
        // a fresh book keeps the control honest.
        let control_book = Rc::new(RefCell::new(GossipBook::default()));
        let without = Simulation::seeded(13).run_town_hidden(4, &plain_refs, &control_book).unwrap();

        let book = Rc::new(RefCell::new(GossipBook::default()));
        let wired: Vec<Box<dyn Persona>> = mk_explorers(0.9, book.clone());
        let wired_refs: Vec<&dyn Persona> = wired.iter().map(|p| p.as_ref()).collect();
        let with = Simulation::seeded(13).run_town_hidden(4, &wired_refs, &book).unwrap();

        assert!(
            with.success_rate() > without.success_rate(),
            "gossip should help: {:.2} vs {:.2}",
            with.success_rate(),
            without.success_rate(),
        );
        assert!(!book.borrow().is_empty(), "successful agents should have published sightings");
    }
}
