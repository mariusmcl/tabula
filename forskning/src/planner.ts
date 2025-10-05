import { KnowledgeGraph, OpContext, UnitValue } from "./types.js";
import { getMeasurementOp, multiplyUnitOp } from "./ops.js";
import { createMeasurementStore, getMeasurement } from "./measure.js";

export interface ScaleByFactorPlanInput {
  entityLabel: string; // e.g., "Taj Mahal"
  property: string; // e.g., "height"
  factor: number; // e.g., 3
}

export interface ScaleByFactorResult {
  original: UnitValue | null;
  scaled: UnitValue | null;
}

export function planScaleByFactor(
  graph: KnowledgeGraph,
  input: ScaleByFactorPlanInput,
  measurements?: ReturnType<typeof createMeasurementStore>
): { ctx: OpContext; run: () => ScaleByFactorResult } {
  // Resolve label to entity id via simple lookup
  const entity = [...graph.entities.values()].find(
    (e) => e.label.toLowerCase() === input.entityLabel.toLowerCase()
  );
  if (!entity) {
    const ctx: OpContext = { graph, measurements };
    return { ctx, run: () => ({ original: null, scaled: null }) };
  }

  const ctx: OpContext = { graph, measurements };
  const run = (): ScaleByFactorResult => {
    const original = getMeasurementOp.run(
      { entity: entity.id, property: input.property },
      ctx
    );
    if (!original) return { original: null, scaled: null };
    const scaled = multiplyUnitOp.run(
      { x: original, factor: input.factor },
      ctx
    );
    return { original, scaled };
  };

  return { ctx, run };
}
