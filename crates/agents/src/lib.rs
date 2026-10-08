//! Simulated agents: persona trait, heuristic-stub personas, and an
//! [`LlmProvider`] seam where a real model backend can later be plugged in.
//!
//! The crate is deliberately dependency-free and deterministic: every persona
//! decision is driven by an explicit [`Rng`] seed so simulations are
//! reproducible and testable offline.

#![warn(missing_docs)]

mod rng;

#[cfg(feature = "llm")]
pub mod llm;

pub use rng::Rng;

/// A single attempt at a task, as observed by a verifier.
///
/// The attempt is intentionally minimal: a verifier scores what the agent
/// *did* (which world it acted in, and the action record), not what it
/// intended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attempt {
    /// Identifier of the world the agent chose to act in.
    pub world_id: u64,
    /// Serialized action payload. Interpretation is environment-specific.
    pub actions: Vec<String>,
}

impl Attempt {
    /// An attempt that does nothing — used by verifiers to reject no-ops.
    pub fn no_op(world_id: u64) -> Self {
        Attempt { world_id, actions: Vec::new() }
    }
}

/// Extract the first integer from a task prompt ("Buy 3 apples" → 3).
///
/// Defaults to 1 when no number is present, so prompts can stay informal.
pub fn parse_quantity(prompt: &str) -> u64 {
    let digits: String = prompt
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse().unwrap_or(1)
}

/// Extract the shop id from a task prompt ("... at shop 2." → Some(2)).
///
/// Returns `None` when the prompt names no shop.
pub fn parse_shop_id(prompt: &str) -> Option<u64> {
    let needle = "shop ";
    let idx = prompt.to_ascii_lowercase().find(needle)? + needle.len();
    let digits: String = prompt[idx..]
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse().ok()
}

/// Find the world a prompt refers to, falling back to the first world.
pub fn resolve_world<'a>(prompt: &str, worlds: &'a [WorldView]) -> Option<&'a WorldView> {
    let first = worlds.first()?;
    Some(match parse_shop_id(prompt) {
        Some(id) => worlds.iter().find(|w| w.id == id).unwrap_or(first),
        None => first,
    })
}

/// A persona decides how an agent turns a task prompt into an [`Attempt`].
///
/// Implementations must be deterministic given the same `rng` state — this is
/// what keeps whole simulations replayable from a seed.
pub trait Persona {
    /// Human-readable persona name, e.g. `"careful-shopper"`.
    fn name(&self) -> &str;
    /// Produce an attempt for `task_prompt` inside the candidate `worlds`.
    fn act(&self, task_prompt: &str, worlds: &[WorldView], rng: &mut Rng) -> Attempt;
}

/// What a persona can see about one candidate world.
#[derive(Debug, Clone)]
pub struct WorldView {
    /// World identifier, matching the verifier's world.
    pub id: u64,
    /// Public description shown to the agent.
    pub description: String,
}

/// Provider seam for real LLM backends.
///
/// The default build uses [`StubProvider`], which never performs network
/// calls. A future feature-gated backend can implement this trait to let
/// personas defer generation to a real model.
pub trait LlmProvider {
    /// Generate a completion for `prompt`. Returning `None` models a
    /// backend failure and callers must handle it.
    fn complete(&self, prompt: &str) -> Option<String>;
}

/// Offline provider that returns canned responses keyed by prompt content.
///
/// Used in tests and as the default so no code path ever requires network
/// access.
#[derive(Debug, Default)]
pub struct StubProvider;

impl LlmProvider for StubProvider {
    fn complete(&self, prompt: &str) -> Option<String> {
        Some(format!("stub: {prompt}"))
    }
}

/// A shopper persona for the town playground: remembers the right shop most
/// of the time, occasionally slips to a wrong one, and sometimes gives up.
#[derive(Debug, Clone)]
pub struct ShopperPersona {
    name: String,
    /// Probability (0.0–1.0) of recalling the correct shop.
    pub recall: f64,
    /// Probability (0.0–1.0) of abandoning the task (no-op attempt).
    pub quit_chance: f64,
}

impl ShopperPersona {
    /// Build a persona with the given recall and quit probabilities.
    pub fn new(name: impl Into<String>, recall: f64, quit_chance: f64) -> Self {
        assert!((0.0..=1.0).contains(&recall), "recall must be in 0.0..=1.0");
        assert!((0.0..=1.0).contains(&quit_chance), "quit_chance must be in 0.0..=1.0");
        ShopperPersona { name: name.into(), recall, quit_chance }
    }
}

impl Persona for ShopperPersona {
    fn name(&self) -> &str {
        &self.name
    }

    fn act(&self, task_prompt: &str, worlds: &[WorldView], rng: &mut Rng) -> Attempt {
        let Some(target) = resolve_world(task_prompt, worlds) else {
            return Attempt::no_op(0);
        };
        if rng.chance(self.quit_chance) {
            return Attempt::no_op(target.id);
        }
        let world = if worlds.len() > 1 && !rng.chance(self.recall) {
            // Misremember the errand: drift to any other shop.
            let others: Vec<_> = worlds.iter().filter(|w| w.id != target.id).collect();
            let idx = rng.below(others.len() as u64) as usize;
            others[idx]
        } else {
            target
        };
        Attempt { world_id: world.id, actions: buy_actions(parse_quantity(task_prompt)) }
    }
}

/// An oracle persona that always acts correctly in the first world — the
/// reference "solvable" behavior used to stress-test verifiers.
#[derive(Debug, Default)]
pub struct OraclePersona;

impl Persona for OraclePersona {
    fn name(&self) -> &str {
        "oracle"
    }

    fn act(&self, _task_prompt: &str, worlds: &[WorldView], _rng: &mut Rng) -> Attempt {
        match resolve_world(_task_prompt, worlds) {
            Some(w) => Attempt { world_id: w.id, actions: buy_actions(parse_quantity(_task_prompt)) },
            None => Attempt::no_op(0),
        }
    }
}

fn buy_actions(quantity: u64) -> Vec<String> {
    std::iter::once("enter".to_string())
        .chain(std::iter::repeat_n("buy".to_string(), quantity as usize))
        .chain(std::iter::once("pay".to_string()))
        .collect()
}

/// Extract the item name from a task prompt ("Buy 3 apples at shop 2" →
/// "apples"). Returns `None` when no alphabetic item follows the quantity.
pub fn parse_item(prompt: &str) -> Option<String> {
    let lower = prompt.to_ascii_lowercase();
    let idx = lower.find("buy ")? + 4;
    let mut chars = prompt[idx..].chars();
    // Skip the quantity digits.
    for c in chars.by_ref() {
        if !(c.is_ascii_digit() || c == ' ') {
            let item: String = std::iter::once(c)
                .chain(chars.by_ref())
                .take_while(|c| c.is_alphabetic())
                .collect();
            return if item.is_empty() { None } else { Some(item) };
        }
    }
    None
}

/// A shared memory of stock sightings, reported by agents after successful
/// errands. This is the Smallville-style information-diffusion layer: one
/// agent's success becomes another agent's knowledge.
#[derive(Debug, Default, Clone)]
pub struct GossipBook {
    reports: Vec<GossipReport>,
}

/// One sighting: `agent` saw `item` in stock at `shop_id`.
#[derive(Debug, Clone)]
pub struct GossipReport {
    /// Agent who made the report.
    pub agent: String,
    /// Shop where the item was found.
    pub shop_id: u64,
    /// Item found in stock.
    pub item: String,
}

impl GossipBook {
    /// Record a sighting.
    pub fn report(&mut self, agent: impl Into<String>, shop_id: u64, item: impl Into<String>) {
        self.reports.push(GossipReport { agent: agent.into(), shop_id, item: item.into() });
    }
    /// The most recent shop reported to stock `item`, if any.
    pub fn lookup(&self, item: &str) -> Option<u64> {
        self.reports.iter().rev().find(|r| r.item == item).map(|r| r.shop_id)
    }
    /// Number of reports recorded.
    pub fn len(&self) -> usize {
        self.reports.len()
    }
    /// True when no reports have been recorded.
    pub fn is_empty(&self) -> bool {
        self.reports.is_empty()
    }
}

/// A persona that consults a shared [`GossipBook`] before acting.
///
/// When the task prompt names no shop (hidden-location errands) and the book
/// knows where the item is, the persona trusts the report with probability
/// `trust` and goes straight there; otherwise it falls back to the inner
/// persona's own behavior.
#[derive(Debug, Clone)]
pub struct GossipingPersona<P> {
    inner: P,
    book: std::rc::Rc<std::cell::RefCell<GossipBook>>,
    trust: f64,
}

impl<P> GossipingPersona<P> {
    /// Wrap `inner` with access to `book`; `trust` is the probability of
    /// following a report when one exists.
    pub fn new(
        inner: P,
        book: std::rc::Rc<std::cell::RefCell<GossipBook>>,
        trust: f64,
    ) -> Self {
        assert!((0.0..=1.0).contains(&trust), "trust must be in 0.0..=1.0");
        GossipingPersona { inner, book, trust }
    }
}

impl<P: Persona> Persona for GossipingPersona<P> {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn act(&self, task_prompt: &str, worlds: &[WorldView], rng: &mut Rng) -> Attempt {
        if parse_shop_id(task_prompt).is_none() {
            if let Some(item) = parse_item(task_prompt) {
                if let Some(shop) = self.book.borrow().lookup(&item) {
                    if rng.chance(self.trust) {
                        if let Some(world) = worlds.iter().find(|w| w.id == shop) {
                            return Attempt {
                                world_id: world.id,
                                actions: buy_actions(parse_quantity(task_prompt)),
                            };
                        }
                    }
                }
            }
        }
        self.inner.act(task_prompt, worlds, rng)
    }
}

/// An explorer persona for hidden-location errands: it has no memory of
/// which shop stocks what, so it tries a uniformly random shop (and
/// sometimes gives up). With a [`GossipBook`] and a [`GossipingPersona`]
/// wrapper, its odds improve dramatically.
#[derive(Debug, Clone)]
pub struct ExplorerPersona {
    name: String,
    /// Probability (0.0–1.0) of abandoning the task (no-op attempt).
    pub quit_chance: f64,
}

impl ExplorerPersona {
    /// Build an explorer with the given quit probability.
    pub fn new(name: impl Into<String>, quit_chance: f64) -> Self {
        assert!((0.0..=1.0).contains(&quit_chance), "quit_chance must be in 0.0..=1.0");
        ExplorerPersona { name: name.into(), quit_chance }
    }
}

impl Persona for ExplorerPersona {
    fn name(&self) -> &str {
        &self.name
    }

    fn act(&self, task_prompt: &str, worlds: &[WorldView], rng: &mut Rng) -> Attempt {
        let Some(world) = rng.pick(worlds) else {
            return Attempt::no_op(0);
        };
        if rng.chance(self.quit_chance) {
            return Attempt::no_op(world.id);
        }
        Attempt { world_id: world.id, actions: buy_actions(parse_quantity(task_prompt)) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn views() -> Vec<WorldView> {
        (0..3)
            .map(|i| WorldView { id: i, description: format!("world {i}") })
            .collect()
    }

    #[test]
    fn oracle_always_acts_in_first_world() {
        let mut rng = Rng::seeded(42);
        let p = OraclePersona;
        for _ in 0..20 {
            let a = p.act("buy 2 apples", &views(), &mut rng);
            assert_eq!(a.world_id, 0);
            assert!(!a.actions.is_empty());
        }
    }

    #[test]
    fn shopper_recall_one_never_misremembers() {
        let mut rng = Rng::seeded(7);
        let p = ShopperPersona::new("perfect", 1.0, 0.0);
        for _ in 0..50 {
            assert_eq!(p.act("t", &views(), &mut rng).world_id, 0);
        }
    }

    #[test]
    fn shopper_quit_chance_one_always_no_ops() {
        let mut rng = Rng::seeded(7);
        let p = ShopperPersona::new("quitter", 1.0, 1.0);
        for _ in 0..20 {
            assert!(p.act("t", &views(), &mut rng).actions.is_empty());
        }
    }

    #[test]
    fn stub_provider_responds_without_network() {
        let p = StubProvider;
        assert!(p.complete("hello").unwrap().contains("hello"));
    }

    #[test]
    #[should_panic]
    fn shopper_rejects_out_of_range_recall() {
        ShopperPersona::new("bad", 1.5, 0.0);
    }

    #[test]
    fn parse_item_extracts_the_item() {
        assert_eq!(parse_item("Buy 3 apples at shop 2.").as_deref(), Some("apples"));
        assert_eq!(parse_item("Buy 1 coffee.").as_deref(), Some("coffee"));
        assert_eq!(parse_item("no items here"), None);
    }

    #[test]
    fn gossip_book_returns_most_recent_sighting() {
        let mut book = GossipBook::default();
        assert!(book.is_empty());
        book.report("clara", 2, "apples");
        book.report("dan", 3, "apples");
        book.report("clara", 1, "bread");
        assert_eq!(book.lookup("apples"), Some(3));
        assert_eq!(book.lookup("bread"), Some(1));
        assert_eq!(book.lookup("milk"), None);
        assert_eq!(book.len(), 3);
    }

    #[test]
    fn explorer_picks_a_valid_world() {
        let mut rng = Rng::seeded(5);
        let p = ExplorerPersona::new("explorer", 0.0);
        for _ in 0..50 {
            let a = p.act("Buy 2 apples.", &views(), &mut rng);
            assert!(a.world_id < 3);
            assert!(!a.actions.is_empty());
        }
    }

    #[test]
    fn gossiping_persona_follows_the_book_at_trust_one() {
        let book = std::rc::Rc::new(std::cell::RefCell::new(GossipBook::default()));
        book.borrow_mut().report("clara", 2, "apples");
        let p = GossipingPersona::new(ExplorerPersona::new("explorer", 0.0), book, 1.0);
        let mut rng = Rng::seeded(9);
        for _ in 0..20 {
            let a = p.act("Buy 2 apples.", &views(), &mut rng);
            assert_eq!(a.world_id, 2, "should follow the report");
        }
    }

    #[test]
    fn gossiping_persona_ignores_the_book_at_trust_zero() {
        let book = std::rc::Rc::new(std::cell::RefCell::new(GossipBook::default()));
        book.borrow_mut().report("clara", 2, "apples");
        let p = GossipingPersona::new(ExplorerPersona::new("explorer", 0.0), book, 0.0);
        let mut rng = Rng::seeded(9);
        // With zero trust it never lands on shop 2 via the report alone —
        // but the explorer may randomly pick it, so just check it acts.
        for _ in 0..20 {
            assert!(!p.act("Buy 2 apples.", &views(), &mut rng).actions.is_empty());
        }
    }
}
