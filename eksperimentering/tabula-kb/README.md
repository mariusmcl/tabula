# Tabula KB — Deterministic, On-Chain Knowledge Base (Rust)

This is a minimal, production-shaped Rust workspace that implements a **Wolfram-style knowledge base**
with **entities**, **quantities/units**, **computed properties**, and a **canonical ordered-Merkle state**,
all executed through a tiny **deterministic VM + system contract** — suitable for your "everything on-chain" requirement.

## Highlights

- **Fixed-point math** (`i128` with 32 fractional bits) — no floats, truncation at every op.
- **Units & dimensions** (SI bases; `kcal`, `J`, `g`, `km²`, `g/mol`, `kg/m³`, `1/km²`, etc.).
- **Entity model** (Food, Country, City, Element, Material, Body, Constant) with canonical codec.
- **Computed properties**:
  - `Food.EnergyPerServing` = `EnergyPer100g * MassPerServing / 100 g`
  - `Country.PopulationDensity` = `Population / Area` (reported in `1/km²`)
  - `Material.MolarMass` from chemical formula and element atomic weights
  - `Body.RestEnergy` = `m * c²` (Joules; `c=299,792,458 m/s` exact)
- **On-chain execution** simulator:
  - `vm` crate: minimalist VM and state
  - `contracts` crate: `KbContract` with `seed_demo_data()` (method 1) and `query()` (method 2)
  - `store` crate: ordered-Merkle root over canonical key/value encodings
- **Seed data**: foods (Salad, Banana), countries (France, UnitedStates, India), cities, elements (H, O, Fe), materials (Water, Iron), physics (Body1kg).

## Build & Run

```bash
cd tabula-kb
cargo run -p node
```

You should see something like:

```
State root after seed: [..32-byte root..]
food:"Salad".EnergyPerServing -> 66.000000 kcal
country:"France".PopulationDensity -> 121.449000 1/km^2
material:"Water".MolarMass -> 18.015680 g/mol
body:"Body1kg".RestEnergy -> 89875517873681760.000000 J
```

*(Values are deterministic; rounding is **truncation toward zero**.)*

## Design Choices

- **Rounding policy**: truncation toward zero for all fixed-point ops (mul/div) and for decimal printing.
- **Canonical keys**: `ty(4) 0x1F key 0x1F prop(4)` with lowercase ASCII keys.
- **Canonical value codec**: simple self-describing encoding; no external serde/bincode drift.
- **Merkle root**: SHA-256 over `(len(k)||k||len(v)||v)` in lexicographic key order.
- **Determinism**: no system clock, no randomness, no floats, no IO in contracts.

## Extending

- Add more props/entities by assigning new ids in `entity::Registry` (freeze by governance).
- Add more computed properties in `eval::eval_property` with pure fixed-point arithmetic.
- Extend unit pretty-printer in `units::fmt_unit` to label more unit combos.
- Swap `store::KV` for a Sparse Merkle Tree once you want efficient proofs.
- Wire into your consensus/PoUW runtime by treating `vm` calls as transactions.

---

> This is a deterministic scaffold, not a dataset. The seed values are **illustrative** for testing the pipeline.