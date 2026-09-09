//! Synthetic environment synthesis and verifier synthesis.
//!
//! Following the GLM-5.3-style pipeline this crate models:
//!
//! 1. **Environment synthesis** — a seed grows a town of shops with *hidden*
//!    stock state and a set of shopping tasks expressed against it.
//! 2. **Verifier synthesis** — a binary verifier is generated from the task's
//!    *constraints* (never from a reference solution), then stress-tested with
//!    oracle / no-op / unsolved attempts before it is trusted for training.

#![warn(missing_docs)]

use simulacra_agents::{Attempt, WorldView};

const ITEMS: &[&str] = &["apples", "bread", "milk", "eggs", "cheese", "coffee"];

/// Hidden state of one shop: how many units of each item are in stock.
#[derive(Debug, Clone)]
pub struct Stock {
    item: String,
    quantity: u64,
}

impl Stock {
    /// Item name.
    pub fn item(&self) -> &str {
        &self.item
    }
    /// Units in stock.
    pub fn quantity(&self) -> u64 {
        self.quantity
    }
}

/// One synthesized task world: a shop, its hidden stock, and the task prompt
/// and target parameters a verifier will check against.
#[derive(Debug, Clone)]
pub struct TaskWorld {
    id: u64,
    description: String,
    stock: Vec<Stock>,
    task: TaskSpec,
}

/// The parameters of a shopping task, expressed as constraints the verifier
/// checks. This — not any reference attempt — is the ground truth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskSpec {
    /// Which shop the errand targets.
    pub shop_id: u64,
    /// Item to buy.
    pub item: String,
    /// How many units must be bought.
    pub quantity: u64,
}

impl TaskWorld {
    /// Stable world identifier.
    pub fn id(&self) -> u64 {
        self.id
    }
    /// Public description (what an agent sees).
    pub fn description(&self) -> &str {
        &self.description
    }
    /// The task constraints.
    pub fn task(&self) -> &TaskSpec {
        &self.task
    }
    /// Units of `item` in stock, if stocked.
    pub fn stock_of(&self, item: &str) -> Option<u64> {
        self.stock.iter().find(|s| s.item == item).map(|s| s.quantity)
    }
    /// All stocked items.
    pub fn stock(&self) -> &[Stock] {
        &self.stock
    }
    /// Public view of this world for personas.
    pub fn view(&self) -> WorldView {
        WorldView { id: self.id, description: self.description.clone() }
    }
}

/// Synthesizes towns (collections of task worlds) from a seed.
#[derive(Debug)]
pub struct EnvironmentSynthesizer {
    seed: u64,
}

impl EnvironmentSynthesizer {
    /// Create a synthesizer for the given seed.
    pub fn new(seed: u64) -> Self {
        EnvironmentSynthesizer { seed }
    }

    /// Grow a town of `n_shops` shops, each stocked with `items_per_shop`
    /// distinct items, and emit one task per shop.
    pub fn synthesize_town(&self, n_shops: u64, items_per_shop: u64) -> Vec<TaskWorld> {
        let mut rng = simulacra_agents::Rng::seeded(self.seed);
        let mut worlds = Vec::new();
        for shop in 0..n_shops {
            let id = shop;
            let mut stock = Vec::new();
            let mut used = Vec::new();
            for _ in 0..items_per_shop {
                let item = loop {
                    let candidate = ITEMS[rng.below(ITEMS.len() as u64) as usize];
                    if !used.contains(&candidate) {
                        used.push(candidate);
                        break candidate;
                    }
                };
                stock.push(Stock { item: item.into(), quantity: 1 + rng.below(4) });
            }
            let task_item = stock[0].item.clone();
            let task_qty = stock[0].quantity;
            let description = format!("Shop {shop}: a small store on the town square.");
            worlds.push(TaskWorld {
                id,
                description,
                stock,
                task: TaskSpec { shop_id: id, item: task_item, quantity: task_qty },
            });
        }
        worlds
    }
}

/// A synthesized binary verifier for one task world.
///
/// The verifier is constructed purely from the task's constraints, so it is
/// independent of any reference solution. Call [`Verifier::stress_test`]
/// before trusting its reward for training.
#[derive(Debug, Clone)]
pub struct Verifier {
    spec: TaskSpec,
}

impl Verifier {
    /// Synthesize a verifier for `world`'s task from its constraints only.
    pub fn synthesize(world: &TaskWorld) -> Self {
        Verifier { spec: world.task().clone() }
    }

    /// Binary reward: 1.0 if the attempt fully satisfies the task, else 0.0.
    pub fn reward(&self, attempt: &Attempt, world: &TaskWorld) -> f64 {
        if self.check(attempt, world).is_empty() {
            1.0
        } else {
            0.0
        }
    }

    /// Human-readable list of violated constraints (empty == pass).
    pub fn check(&self, attempt: &Attempt, world: &TaskWorld) -> Vec<String> {
        let mut violations = Vec::new();
        if attempt.actions.is_empty() {
            violations.push("no-op attempt".into());
            return violations;
        }
        if attempt.world_id != self.spec.shop_id {
            violations.push(format!("acted in shop {} instead of {}", attempt.world_id, self.spec.shop_id));
        }
        if !attempt.actions.iter().any(|a| a == "buy") {
            violations.push("never bought anything".into());
        }
        if !attempt.actions.iter().any(|a| a == "pay") {
            violations.push("never paid".into());
        }
        let bought = attempt.actions.iter().filter(|a| *a == "buy").count() as u64;
        if bought != self.spec.quantity {
            violations.push(format!("bought {bought} units, task requires {}", self.spec.quantity));
        }
        if world.stock_of(&self.spec.item).unwrap_or(0) < self.spec.quantity {
            violations.push(format!("shop is out of stock of {}", self.spec.item));
        }
        violations
    }

    /// Stress-test the verifier with oracle, no-op, and unsolved attempts.
    ///
    /// Returns `Err` with a description if the verifier would mislabel any of
    /// the three canonical probes — mirroring the oracle / no-op /
    /// unsolved-state checks used before a synthesized verifier is trusted.
    pub fn stress_test(&self, world: &TaskWorld) -> Result<(), String> {
        let oracle = Attempt {
            world_id: self.spec.shop_id,
            actions: std::iter::repeat_n("buy".to_string(), self.spec.quantity as usize)
                .chain(std::iter::once("pay".to_string()))
                .collect(),
        };
        if self.reward(&oracle, world) != 1.0 {
            return Err("oracle attempt was not accepted".into());
        }
        let no_op = Attempt::no_op(self.spec.shop_id);
        if self.reward(&no_op, world) != 0.0 {
            return Err("no-op attempt was not rejected".into());
        }
        let unsolved = Attempt { world_id: (self.spec.shop_id + 1).max(1), actions: vec!["buy".into(), "pay".into()] };
        if self.reward(&unsolved, world) == 1.0 {
            return Err("unsolved attempt was not rejected".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use simulacra_agents::{OraclePersona, Persona, Rng, ShopperPersona};

    fn town() -> Vec<TaskWorld> {
        EnvironmentSynthesizer::new(42).synthesize_town(3, 2)
    }

    #[test]
    fn synthesis_is_deterministic() {
        let a = EnvironmentSynthesizer::new(42).synthesize_town(4, 3);
        let b = EnvironmentSynthesizer::new(42).synthesize_town(4, 3);
        assert_eq!(a.len(), b.len());
        for (wa, wb) in a.iter().zip(b.iter()) {
            assert_eq!(wa.task(), wb.task());
            assert_eq!(wa.stock().len(), wb.stock().len());
        }
    }

    #[test]
    fn different_seeds_diverge() {
        let a = EnvironmentSynthesizer::new(1).synthesize_town(4, 3);
        let b = EnvironmentSynthesizer::new(2).synthesize_town(4, 3);
        let ta: Vec<_> = a.iter().map(|w| w.task().clone()).collect();
        let tb: Vec<_> = b.iter().map(|w| w.task().clone()).collect();
        assert_ne!(ta, tb);
    }

    #[test]
    fn verifier_passes_stress_test_on_fresh_worlds() {
        for world in town() {
            let v = Verifier::synthesize(&world);
            v.stress_test(&world).expect("synthesized verifier failed stress test");
        }
    }

    #[test]
    fn oracle_persona_earns_reward() {
        let world = town().remove(0);
        let prompt = format!("Buy {} {} at shop {}.", world.task().quantity, world.task().item, world.task().shop_id);
        let v = Verifier::synthesize(&world);
        let mut rng = Rng::seeded(1);
        let attempt = OraclePersona.act(&prompt, &[world.view()], &mut rng);
        assert_eq!(v.reward(&attempt, &world), 1.0);
    }

    #[test]
    fn shopper_persona_earns_reward_with_recall_one() {
        let world = town().remove(0);
        let prompt = format!("Buy {} {} at shop {}.", world.task().quantity, world.task().item, world.task().shop_id);
        let v = Verifier::synthesize(&world);
        let p = ShopperPersona::new("perfect", 1.0, 0.0);
        let mut rng = Rng::seeded(1);
        let attempt = p.act(&prompt, &[world.view()], &mut rng);
        assert_eq!(v.reward(&attempt, &world), 1.0);
    }

    #[test]
    fn stock_never_exceeds_task_quantity() {
        for seed in 0..20 {
            for world in EnvironmentSynthesizer::new(seed).synthesize_town(3, 3) {
                let t = world.task();
                assert!(world.stock_of(&t.item).unwrap() >= t.quantity);
            }
        }
    }
}
