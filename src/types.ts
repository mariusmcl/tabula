export type EntityId = string;

export interface EntityToken {
  id: EntityId;
  label: string; // human-readable name, e.g., "Taj Mahal" or "mass"
  types: string[]; // ontology types, e.g., ["Place"], ["PhysicalQuantity"]
}

export interface RelationEdge {
  subject: EntityId;
  predicate: string; // e.g., "locatedIn", "hasUnit", "instanceOf"
  object: EntityId;
  weight?: number; // optional confidence/strength
}

export interface KnowledgeGraph {
  entities: Map<EntityId, EntityToken>;
  adjacency: Map<EntityId, RelationEdge[]>; // outgoing edges per node
}

export interface EntityEmbeddingSpace {
  dimension: number;
  vectorById: Map<EntityId, Float64Array>;
}

export type TokenSequence = EntityId[];

export interface GraphQuery {
  start: EntityId | null;
  predicates: string[]; // path predicates to follow
  maxDepth: number;
}

export interface QueryResultPath {
  nodes: EntityId[];
  edges: RelationEdge[];
}

export interface OpContext {
  graph: KnowledgeGraph;
  embeddings?: EntityEmbeddingSpace;
  measurements?: MeasurementStore;
}

export interface EntityOp<I, O> {
  name: string;
  run: (input: I, ctx: OpContext) => O;
}

export interface UnitValue {
  value: number;
  unit: string; // e.g., "m", "kg"
}

export interface MeasurementStore {
  byEntity: Map<EntityId, Map<string, UnitValue>>; // property -> UnitValue
}
