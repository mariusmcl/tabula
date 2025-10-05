import {
  EntityId,
  EntityOp,
  OpContext,
  TokenSequence,
  GraphQuery,
  QueryResultPath,
  UnitValue,
} from "./types.js";
import { neighborsByPredicate, runPathQuery } from "./graph.js";

export const followOp: EntityOp<
  { start: EntityId; predicate: string },
  EntityId[]
> = {
  name: "follow",
  run: ({ start, predicate }, ctx: OpContext) => {
    const edges = neighborsByPredicate(ctx.graph, start, predicate);
    return edges.map((e) => e.object);
  },
};

export const pathOp: EntityOp<GraphQuery, QueryResultPath[]> = {
  name: "path",
  run: (query, ctx: OpContext) => runPathQuery(ctx.graph, query),
};

export const intersectOp: EntityOp<{ sets: EntityId[][] }, EntityId[]> = {
  name: "intersect",
  run: ({ sets }) => {
    if (sets.length === 0) return [];
    const [first, ...rest] = sets.map((s) => new Set(s));
    const result: EntityId[] = [];
    for (const id of first) {
      if (rest.every((s) => s.has(id))) result.push(id);
    }
    return result;
  },
};

export const dedupeOp: EntityOp<TokenSequence, TokenSequence> = {
  name: "dedupe",
  run: (seq) => Array.from(new Set(seq)),
};

export const multiplyUnitOp: EntityOp<
  { x: UnitValue; factor: number },
  UnitValue
> = {
  name: "multiplyUnit",
  run: ({ x, factor }) => ({ value: x.value * factor, unit: x.unit }),
};

export const getMeasurementOp: EntityOp<
  { entity: EntityId; property: string },
  UnitValue | null
> = {
  name: "getMeasurement",
  run: ({ entity, property }, ctx: OpContext) => {
    const v = ctx.measurements?.byEntity.get(entity)?.get(property);
    return v ?? null;
  },
};
