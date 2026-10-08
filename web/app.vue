<template>
  <div class="app">
    <header class="topnav">
      <div class="brand">
        <span class="logo" aria-hidden="true">⬡</span>
        <h1 class="brand-name">Simulacra</h1>
      </div>
      <nav class="nav-links">
        <a href="#town">Town Run</a>
        <a href="#variance">Variance Experiment</a>
        <a href="#coevolution">Co-Evolution</a>
        <a href="#gossip">Gossip Effect</a>
      </nav>
    </header>

    <main class="content">
      <p class="intro">
        A playground for synthetic human simulation — agent personas run
        shopping errands in a simulated town and are scored by synthesized
        verifiers. Pick a seed and run an experiment below.
      </p>

      <!-- ============ TOWN RUN ============ -->
      <section id="town" class="panel">
        <div class="panel-head">
          <h2>Town Run</h2>
          <div class="controls">
            <label class="seed-field">
              Seed
              <input v-model="town.seed" type="number" min="0" step="1" />
            </label>
            <button class="run-btn" :disabled="town.loading" @click="loadTown">
              {{ town.loading ? 'Running…' : 'Run' }}
            </button>
          </div>
        </div>

        <p v-if="town.error" class="placeholder">Start the simulacra server to see live data</p>

        <template v-else-if="town.data">
          <p class="summary" aria-live="polite">
            <strong>{{ town.data.results.length }}</strong> attempts ·
            <strong>{{ pct(town.data.success_rate) }}</strong> success rate ·
            <strong>{{ town.data.total_reward }}</strong> total reward
          </p>

          <table class="data-table">
            <thead>
              <tr>
                <th>Agent</th>
                <th>Shop</th>
                <th>Task</th>
                <th>Result</th>
              </tr>
            </thead>
            <tbody>
              <template v-for="(r, i) in town.data.results" :key="i">
                <tr>
                  <td>{{ r.agent }}</td>
                  <td>{{ r.world_id }}</td>
                  <td>{{ r.task }}</td>
                  <td>
                    <span class="badge" :class="isPass(r) ? 'pass' : 'fail'">
                      {{ isPass(r) ? 'PASS' : 'FAIL' }}
                    </span>
                  </td>
                </tr>
                <tr v-if="!isPass(r)" class="violation-row">
                  <td colspan="4">
                    <ul class="violations">
                      <li v-for="(v, j) in r.violations" :key="j">{{ v }}</li>
                    </ul>
                  </td>
                </tr>
              </template>
            </tbody>
          </table>
        </template>

        <p v-else class="placeholder">Enter a seed and press Run.</p>
      </section>

      <!-- ============ VARIANCE EXPERIMENT ============ -->
      <section id="variance" class="panel">
        <div class="panel-head">
          <h2>Variance Experiment</h2>
          <div class="controls">
            <label class="seed-field">
              Seed
              <input v-model="variance.seed" type="number" min="0" step="1" />
            </label>
            <button class="run-btn" :disabled="variance.loading" @click="loadVariance">
              {{ variance.loading ? 'Running…' : 'Run' }}
            </button>
          </div>
        </div>

        <p v-if="variance.error" class="placeholder">Start the simulacra server to see live data</p>

        <template v-else-if="variance.data">
          <div class="hbar-chart">
            <div v-for="(rate, i) in variance.data.per_run_rates" :key="i" class="hbar-row">
              <span class="hbar-label">Run {{ i }}</span>
              <div class="hbar-track">
                <div class="hbar-fill" :style="{ width: pct(rate) }"></div>
              </div>
              <span class="hbar-value">{{ pct(rate) }}</span>
            </div>
          </div>

          <div class="verdict" aria-live="polite">
            <div class="verdict-row">
              <span class="verdict-name">Mean</span>
              <span>empirical {{ variance.data.empirical_mean.toFixed(3) }}</span>
              <span>theoretical {{ variance.data.theoretical_mean.toFixed(3) }}</span>
              <span class="badge" :class="variance.data.mean_match ? 'pass' : 'fail'">
                {{ variance.data.mean_match ? 'OK' : 'MISMATCH' }}
              </span>
            </div>
            <div class="verdict-row">
              <span class="verdict-name">Variance</span>
              <span>empirical {{ variance.data.empirical_variance.toFixed(5) }}</span>
              <span>theoretical {{ variance.data.theoretical_variance.toFixed(5) }}</span>
              <span class="badge" :class="variance.data.variance_match ? 'pass' : 'fail'">
                {{ variance.data.variance_match ? 'OK' : 'MISMATCH' }}
              </span>
            </div>
          </div>

          <div
            class="banner"
            :class="variance.data.mean_match && variance.data.variance_match ? 'pass' : 'fail'"
          >
            DISTRIBUTION
            {{ variance.data.mean_match && variance.data.variance_match ? 'MATCH' : 'MISMATCH' }}
          </div>
        </template>

        <p v-else class="placeholder">Enter a seed and press Run.</p>
      </section>

      <!-- ============ CO-EVOLUTION ============ -->
      <section id="coevolution" class="panel">
        <div class="panel-head">
          <h2>Co-Evolution</h2>
          <div class="controls">
            <label class="seed-field">
              Seed
              <input v-model="coevolution.seed" type="number" min="0" step="1" />
            </label>
            <button class="run-btn" :disabled="coevolution.loading" @click="loadCoevolution">
              {{ coevolution.loading ? 'Running…' : 'Run' }}
            </button>
          </div>
        </div>

        <p v-if="coevolution.error" class="placeholder">Start the simulacra server to see live data</p>

        <template v-else-if="coevolution.data">
          <table class="data-table rounds-table">
            <thead>
              <tr>
                <th>Round</th>
                <th>Shops</th>
                <th>Items / shop</th>
                <th>Success rate</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="round in coevolution.data.rounds" :key="round.round">
                <td>{{ round.round }}</td>
                <td>{{ round.n_shops }}</td>
                <td>{{ round.items_per_shop }}</td>
                <td>
                  <div class="inline-bar">
                    <div class="hbar-track">
                      <div class="hbar-fill" :style="{ width: pct(round.success_rate) }"></div>
                    </div>
                    <span>{{ pct(round.success_rate) }}</span>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </template>

        <p v-else class="placeholder">Enter a seed and press Run.</p>
      </section>

      <!-- ============ GOSSIP EFFECT ============ -->
      <section id="gossip" class="panel">
        <div class="panel-head">
          <h2>Gossip Effect</h2>
          <div class="controls">
            <label class="seed-field">
              Seed
              <input v-model="gossip.seed" type="number" min="0" step="1" />
            </label>
            <button class="run-btn" :disabled="gossip.loading" @click="loadGossip">
              {{ gossip.loading ? 'Running…' : 'Run' }}
            </button>
          </div>
        </div>

        <p v-if="gossip.error" class="placeholder">Start the simulacra server to see live data</p>

        <template v-else-if="gossip.data">
          <div class="cards" aria-live="polite">
            <div class="card">
              <div class="card-value">{{ pct(gossip.data.no_gossip_rate) }}</div>
              <div class="card-label">Success rate without gossip</div>
            </div>
            <div class="card card-accent">
              <div class="card-value">{{ pct(gossip.data.gossip_rate) }}</div>
              <div class="card-label">Success rate with gossip</div>
            </div>
          </div>
          <p class="conclusion">
            Gossip lifts success by
            <strong>{{ deltaPoints(gossip.data) }} points</strong>.
          </p>
        </template>

        <p v-else class="placeholder">Enter a seed and press Run.</p>
      </section>
    </main>
  </div>
</template>

<script setup>
const DEFAULT_SEED = '42'

function useSection() {
  return reactive({
    seed: DEFAULT_SEED,
    loading: false,
    error: false,
    data: null,
  })
}

const town = useSection()
const variance = useSection()
const coevolution = useSection()
const gossip = useSection()

async function fetchJson(section, path, key) {
  section.loading = true
  section.error = false
  try {
    const res = await fetch(`${path}?seed=${encodeURIComponent(section.seed)}`)
    if (!res.ok) throw new Error(`HTTP ${res.status}`)
    section.data = await res.json()
    // Make the run shareable: /app/?section=gossip&seed=7 reproduces it.
    const url = new URL(window.location.href)
    url.searchParams.set('section', key)
    url.searchParams.set('seed', section.seed)
    window.history.replaceState(null, '', url)
  } catch (e) {
    section.error = true
    section.data = null
  } finally {
    section.loading = false
  }
}

const loadTown = () => fetchJson(town, '/api/town', 'town')
const loadVariance = () => fetchJson(variance, '/api/variance', 'variance')
const loadCoevolution = () => fetchJson(coevolution, '/api/coevolution', 'coevolution')
const loadGossip = () => fetchJson(gossip, '/api/gossip', 'gossip')

const RUNNERS = { town: loadTown, variance: loadVariance, coevolution: loadCoevolution, gossip: loadGossip }

// Deep links: ?section=gossip&seed=7&run=1 applies the seed to every
// section and auto-runs the named one (shared reproducible runs).
onMounted(() => {
  const params = new URLSearchParams(window.location.search)
  const seed = params.get('seed')
  if (seed !== null && /^\d+$/.test(seed)) {
    for (const s of [town, variance, coevolution, gossip]) s.seed = seed
  }
  const section = params.get('section')
  if (params.get('run') === '1' && section && RUNNERS[section]) {
    RUNNERS[section]()
  }
})

const isPass = (r) => r.reward >= 1 && (!r.violations || r.violations.length === 0)
const pct = (x) => `${(x * 100).toFixed(1)}%`
const deltaPoints = (d) => ((d.gossip_rate - d.no_gossip_rate) * 100).toFixed(1)
</script>

<style>
:root {
  --bg: #f4f4f6;
  --panel: #ffffff;
  --text: #1f2430;
  --muted: #8a8f9c;
  --border: #e2e4ea;
  --accent: #4f46e5;
  --pass: #15803d;
  --pass-bg: #dcfce7;
  --fail: #b91c1c;
  --fail-bg: #fee2e2;
}

* {
  box-sizing: border-box;
}

html {
  scroll-behavior: smooth;
}

body {
  margin: 0;
  font-family: ui-sans-serif, system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif;
  background: var(--bg);
  color: var(--text);
}

.topnav {
  position: sticky;
  top: 0;
  z-index: 10;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 0.75rem 1.5rem;
  background: var(--panel);
  border-bottom: 1px solid var(--border);
}

.brand {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-weight: 700;
}

.brand-name {
  margin: 0;
  font-size: 1rem;
}

.logo {
  color: var(--accent);
}

.nav-links {
  display: flex;
  gap: 1.25rem;
  flex-wrap: wrap;
}

.nav-links a {
  color: var(--text);
  text-decoration: none;
  font-size: 0.95rem;
}

.nav-links a:hover {
  color: var(--accent);
}

.content {
  max-width: 960px;
  margin: 0 auto;
  padding: 1.5rem;
}

.intro {
  color: var(--muted);
  margin-bottom: 2rem;
}

.panel {
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 1.25rem 1.5rem;
  margin-bottom: 2rem;
  scroll-margin-top: 70px;
}

.panel-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  flex-wrap: wrap;
  margin-bottom: 1rem;
}

.panel-head h2 {
  margin: 0;
  font-size: 1.25rem;
}

.controls {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}

.seed-field {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.9rem;
  color: var(--muted);
}

.seed-field input {
  width: 6rem;
  padding: 0.4rem 0.5rem;
  border: 1px solid var(--border);
  border-radius: 6px;
  font-size: 0.9rem;
}

.run-btn {
  padding: 0.45rem 1.1rem;
  border: none;
  border-radius: 6px;
  background: var(--accent);
  color: #fff;
  font-size: 0.9rem;
  cursor: pointer;
}

.run-btn:disabled {
  opacity: 0.6;
  cursor: default;
}

.placeholder {
  color: var(--muted);
  font-style: italic;
  padding: 1rem 0;
}

.summary {
  margin: 0 0 1rem;
}

.data-table {
  width: 100%;
  border-collapse: collapse;
}

.data-table th,
.data-table td {
  text-align: left;
  padding: 0.55rem 0.75rem;
  border-bottom: 1px solid var(--border);
  font-size: 0.92rem;
}

.data-table th {
  color: var(--muted);
  font-weight: 600;
  font-size: 0.8rem;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.violation-row td {
  border-bottom: none;
  padding-top: 0;
  padding-bottom: 0.6rem;
}

.violations {
  margin: 0;
  padding-left: 2.2rem;
  color: var(--fail);
  font-size: 0.85rem;
}

.badge {
  display: inline-block;
  padding: 0.15rem 0.6rem;
  border-radius: 999px;
  font-size: 0.78rem;
  font-weight: 600;
}

.badge.pass {
  color: var(--pass);
  background: var(--pass-bg);
}

.badge.fail {
  color: var(--fail);
  background: var(--fail-bg);
}

.hbar-chart {
  margin-bottom: 1.25rem;
}

.hbar-row {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  margin-bottom: 0.4rem;
}

.hbar-label {
  width: 3.5rem;
  font-size: 0.82rem;
  color: var(--muted);
  text-align: right;
}

.hbar-track {
  flex: 1;
  height: 1.1rem;
  background: #eef0f4;
  border-radius: 4px;
  overflow: hidden;
}

.hbar-fill {
  height: 100%;
  background: var(--accent);
  border-radius: 4px;
}

.hbar-value {
  width: 3.5rem;
  font-size: 0.82rem;
  font-variant-numeric: tabular-nums;
}

.verdict {
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 0.75rem 1rem;
  margin-bottom: 1rem;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  font-size: 0.9rem;
}

.verdict-row {
  display: flex;
  align-items: center;
  gap: 1rem;
  flex-wrap: wrap;
}

.verdict-name {
  font-weight: 600;
  width: 5rem;
}

.banner {
  text-align: center;
  font-weight: 700;
  letter-spacing: 0.08em;
  padding: 0.8rem;
  border-radius: 8px;
}

.banner.pass {
  color: var(--pass);
  background: var(--pass-bg);
}

.banner.fail {
  color: var(--fail);
  background: var(--fail-bg);
}

.inline-bar {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  min-width: 180px;
}

.inline-bar .hbar-track {
  flex: 1;
  height: 0.9rem;
}

.inline-bar span {
  font-size: 0.85rem;
  font-variant-numeric: tabular-nums;
}

.cards {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 1rem;
  margin-bottom: 1rem;
}

.card {
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 1.5rem;
  text-align: center;
}

.card-accent {
  border-color: var(--accent);
}

.card-value {
  font-size: 2.5rem;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
}

.card-accent .card-value {
  color: var(--accent);
}

.card-label {
  color: var(--muted);
  font-size: 0.9rem;
  margin-top: 0.25rem;
}

.conclusion {
  text-align: center;
  color: var(--muted);
}

.conclusion strong {
  color: var(--text);
}
</style>
