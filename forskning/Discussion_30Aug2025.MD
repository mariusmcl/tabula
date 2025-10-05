## Discussion: Entity-Token LLM + Graph Ops (30 Aug 2025)

### Overview
- **Goal**: Move beyond text-only tokens by treating tokens as real-world entities and abstract concepts, operating over a knowledge graph with custom symbolic and numeric operations, while remaining open-ended with LLM planning.
- **Deliverable today**: A minimal Node + TypeScript prototype with entity tokens, graph ops, a measurement store, and a tiny planner to answer “scale-by-factor” queries (e.g., Taj Mahal height × 3 → 219 m). Plus an end-to-end conceptual architecture and training/deployment story.

### Core idea
- **Entity tokens**: Each token maps to a grounded entity (e.g., `taj_mahal`, concept of `mass`), not just a subword.
- **Graph-first computation**: Run operations like `follow`, `path`, set ops, measurement retrieval, and unit-aware math over entities and relations.
- **Hybrid reasoning**: Use an LLM (or smaller policy) to propose typed plans in a constrained JSON schema; use a deterministic executor with verifiers to guarantee units, types, and reproducibility.

### Why this differs from basic LLM tool-use
- **Typed, inspectable plans**: Plans are explicit JSON programs with variables, units, constraints, and ops, rather than ad-hoc tool strings.
- **Entity grounding**: IDs unify references across data, tools, and steps; avoids string ambiguity.
- **Constraint-guided search**: Verifiers (units/dimensions/feasibility) prune impossible paths pre-execution.
- **Deterministic execution + provenance**: Easy to re-run, audit, and compare; suitable for safety and reproducibility.

### Related lines of work (non-exhaustive)
- Knowledge-enhanced LMs: KGLM, ERNIE, KEPLER, K-Adapter, KnowBERT, REALM/RETRO, GraphRAG.
- Neuro-symbolic reasoning: Neural Theorem Provers, Differentiable Reasoning over KBs, NSM, Text-to-SQL/GraphQL.
- Object-centric tokens: Slot Attention, Perceiver IO (analogy: token≈entity).
- Planning with tools: ReAct-style agents, program induction; mixture-of-experts/modules and constrained decoding.

### System architecture (conceptual)
- **Planner (LLM/policy)**: Emits a JSON plan using a fixed op vocabulary and typed variables (units, dimensions, entity refs).
- **Executor**: Resolves entities, queries the graph/measurements, calls tools/simulators, enforces constraints, and returns structured results with provenance.
- **Verifiers/critics**: Unit/dimension checks, numeric stability, constraint satisfaction, statistical fit, and uncertainty; provide signals for training and runtime gating.
- **Knowledge base**: Versioned knowledge graph of entities/relations; measurement store and optional embedding space; external tools (CAS, solvers, simulators) as callable modules.

### Prototype implemented today
- Minimal TypeScript modules:
  - `src/types.ts`: entity tokens, relations, graph model, `MeasurementStore`, `UnitValue`, `OpContext`.
  - `src/graph.ts`: build graph, add entities/edges, `neighborsByPredicate`, `runPathQuery`.
  - `src/ops.ts`: graph ops (`follow`, `path`, `intersect`, `dedupe`); measurement ops (`getMeasurement`, `multiplyUnit`).
  - `src/measure.ts`: in-memory measurement API.
  - `src/planner.ts`: tiny “scale-by-factor” planner that resolves entity → gets measurement → multiplies by factor.
  - `src/nl.ts`: toy NL→entity/predicate heuristic (for earlier demo queries).
  - `src/demo.ts`: seeds entities and Taj Mahal height; exercises ops and the scale-by-factor plan (returns `219 m`).
- Run locally:
  - `npm run dev`

### Minimal JSON plan schema (v0.1)
```json
{
  "plan_id": "string",
  "version": "string",
  "kb": { "mode": "live|snapshot", "snapshot_id": "string|null" },
  "question": "string",
  "entities": [ { "ref": "e1", "id": "optional-string", "label": "optional-string" } ],
  "vars": [ { "name": "string", "type": "number|unit_value|entity|list", "unit": "optional-string", "value": "optional-any" } ],
  "steps": [
    { "id": "s1", "op": "resolve_entity|getMeasurement|multiplyUnit|follow|path|intersect|dedupe",
      "args": { "any": "JSON" }, "in": { "any": "$var" }, "out": { "varName": "$targetVar" } }
  ],
  "constraints": [ { "type": "unit", "check": "$height.unit == 'm'" } ],
  "verify": ["units", "non_nan", "finite"],
  "return": { "variables": ["$scaled_height"], "format": "json|text" },
  "provenance": { "kb_version": "string|null", "tools": [ { "name": "string", "version": "string" } ] }
}
```

#### Example: “How tall would the Taj Mahal be if scaled by 3?”
```json
{
  "plan_id": "scale-height-001",
  "version": "0.1",
  "kb": { "mode": "live", "snapshot_id": null },
  "question": "how tall would the taj mahal be if we scaled it by a factor of 3",
  "entities": [{ "ref": "e1", "label": "Taj Mahal" }],
  "vars": [
    { "name": "height", "type": "unit_value", "unit": "m" },
    { "name": "factor", "type": "number", "value": 3 },
    { "name": "scaled_height", "type": "unit_value" }
  ],
  "steps": [
    { "id": "s1", "op": "resolve_entity", "args": { "label": "Taj Mahal" }, "out": { "entity": "$e1" } },
    { "id": "s2", "op": "getMeasurement", "args": { "entity": "$e1", "property": "height" }, "out": { "value": "$height" } },
    { "id": "s3", "op": "multiplyUnit", "args": { "x": "$height", "factor": "$factor" }, "out": { "value": "$scaled_height" } }
  ],
  "constraints": [ { "type": "unit", "check": "$height.unit == 'm'" } ],
  "verify": ["units", "non_nan", "finite"],
  "return": { "variables": ["$scaled_height"], "format": "json" },
  "provenance": { "kb_version": "2025-08-30T00:00:00Z", "tools": [{ "name": "measure-engine", "version": "0.1.0" }] }
}
```

### Training vs deployment
- **Training (offline/iterative)**: Use versioned KB snapshots; generate/curate tasks; execute plans in sandbox; collect verifier signals (units, constraints, numeric stability, fit); update planner/value via SFT → RFT/DPO → search with value → optional offline RL for long horizons.
- **Deployment (serving)**: Constrained decoding to plan; execute on live KB; strict guardrails; cache sub-results; return answer + provenance (KB/tool versions). Log traces for later retraining.
- **Same KB, different modes**: Training uses immutable snapshots for reproducibility; deployment uses live KB. All artifacts record KB version/timestamp.

### World model integration (hypothesis generation, counterfactual rollouts)
- **Purpose**: Add discovery power via a learned world model (WM) for counterfactual simulation and hypothesis generation, while keeping math/symbolic/numeric modules for exact computation and verification.
- **Roles**:
  - **World model**: learns predictive latent dynamics and constraints; proposes hypotheses, variables, and experiment designs; supports counterfactual rollouts with uncertainty.
  - **Planner**: queries WM for exploratory rollouts and KB/executor for validated steps; compiles a typed JSON plan over entities, units, and ops.
  - **Executor**: performs deterministic graph ops, unit-safe math, CAS/solver calls; attaches provenance.
  - **Verifier**: enforces units/dimensions, feasibility, residuals/likelihood, and uncertainty bounds; gates execution and provides learning signals.
- **Interfaces (minimal)**:
  - WM: `predict(next | state, action)`, `rollout(policy, horizon)`, `uncertainty(state/plan)`, `propose_hypotheses(question)`
  - Executor: `run(op, args)` with unit/type checks; CAS/solver hooks
  - Verifier: `units()`, `feasibility()`, `residuals()`, `evidence()`
  - Planner: produces the JSON plan; uses WM uncertainty to branch/ask for data
- **Training (not RL-only)**:
  - Self-supervised predictive learning (JEPA/predictive coding) on multimodal traces (text, tables, sims, sensors).
  - Information-theoretic exploration (e.g., information gain/uncertainty reduction) to select simulations/data—no reward hacking required.
  - Physics/constraints in the loss: unit/dimension penalties, invariances, conservation; optional differentiable physics.
  - Symbolic supervision: fit WM to equations discovered by symbolic regression; validate on held-out predictive tests.
  - Optional RL for long-horizon decision quality, with verifiers as hard constraints and shaped rewards.
- **Deployment loop**: Propose (WM) → Plan (typed program) → Execute (deterministic) → Verify (gate + score) → Update (posterior/evidence + provenance).
- **Why this over RL-only**: Precision from math modules, better generalization via structure + checks, data efficiency from information gain, and auditability via explicit plans and traces.
- **Field parallels**: FAIR’s world models/predictive learning; DeepMind hybrids (e.g., geometry/algorithm discovery combining learned proposal + symbolic/optimization engines); OpenAI’s RL/process supervision contrasts with our typed-IR + verifier emphasis.

### Outer learning loop (do we need RL?)
- **Start simple**: Verifier-driven selection and supervised fine-tuning on accepted plans; rejection-based fine-tuning (RFT/DPO) for robustness.
- **Search + value**: Beam/MCTS with a learned value model over partial plans; verifiers prune; train value on rollout returns.
- **When RL helps**: Long-horizon, open-ended problems with multi-objective tradeoffs (accuracy/cost/safety/novelty). Keep verifiers as hard constraints and reward shaping.

### Pros and cons
- **Pros**: Deterministic and testable execution; unit/type safety; transparent, reproducible plans; strong provenance; modular and composable ops; better control over safety/cost.
- **Cons**: Ontology/coverage effort; parser/planner complexity; integration overhead with domain tools; risk of over-constraining exploration without neural heuristics.

### Open questions
- Domain focus (physics, bio, climate, materials, code?).
- Entity/ontology source (bespoke vs Wikidata slice); update cadence and governance.
- Strength of constraints vs heuristic exploration; confidence thresholds; human-in-the-loop points.
- Evaluation metrics: task accuracy, consistency, novelty, reproducibility; cost and latency budgets.

### Open-source graph database options (for the data layer)
- **Dgraph**: distributed, GraphQL/DQL, low-latency; suitable for large KBs.
- **NebulaGraph**: distributed, millisecond latency on very large graphs; C++/Go/Java/Python clients.
- **QLever**: high-performance triplestore for semantic web KBs; strong full-text support.
- **ArangoDB**: multi-model (graph/document/key-value) with AQL; flexible for mixed workloads.
- **OrientDB**: multi-model with graph/document/object features; high performance and scalability.

### Local training on M1 Max (practical next steps)
- **Setup**: PyTorch (MPS), JAX/Flax (Metal), or tinygrad; bf16/float16 where possible; mixed precision.
- **Toy WM tasks**: learn simple dynamics (e.g., point mass, pendulum) with masked/contrastive predictive loss; measure rollout error and uncertainty calibration.
- **Counterfactuals**: implement `rollout(policy, horizon)` and compare against a small simulator; use information gain to pick trajectories to simulate next.
- **Executor coupling**: round-trip a WM hypothesis into a typed plan, execute math/solvers, and log verifier traces and provenance.
- **Data pipeline**: start with synthetic sims, then integrate small real datasets; snapshot KB states for reproducibility.
- **Licensing & repo**: MIT/Apache-2.0, public issues/PRs, contribution guide, CI to validate plans (schema + unit checks).

### Next steps
- Add unit conversion and dimension checks; composite properties (e.g., volume=area×height).
- Improve entity/property resolution (e.g., map “size→height”); better NL→plan parser (constrained JSON via LLM or grammar).
- Integrate one CAS/numeric solver for scientific ops; optional small embedding space for fuzzy linking with deterministic guards.
- Define logging schema with KB snapshot IDs; set up a replay buffer and periodic trainer.

### Appendix
- **Key files (prototype)**: `src/types.ts`, `src/graph.ts`, `src/ops.ts`, `src/measure.ts`, `src/planner.ts`, `src/nl.ts`, `src/demo.ts`.
- **Run**: `npm run dev`
- **Example output**: Taj Mahal height 73 m × 3 → `219 m`.


