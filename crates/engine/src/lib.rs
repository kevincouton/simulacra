//! Simulation loop: drive personas through synthesized task worlds and score
//! every attempt with a stress-tested synthesized verifier.

#![warn(missing_docs)]

use simulacra_agents::{Persona, Rng};
use simulacra_env::{EnvironmentSynthesizer, TaskWorld, Verifier};

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
        if personas.is_empty() {
            return Err("simulation needs at least one persona".into());
        }
        let worlds = EnvironmentSynthesizer::new(self.seed).synthesize_town(n_shops, 3);
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
            let prompt = task_prompt(world);
            let verifier = &verifiers[world.id() as usize];
            for persona in personas {
                let attempt = persona.act(&prompt, &views, &mut rng);
                let violations = verifier.check(&attempt, world);
                results.push(TaskResult {
                    agent: persona.name().to_string(),
                    world_id: world.id(),
                    task: prompt.clone(),
                    reward: verifier.reward(&attempt, world),
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
}
