import { EntityId, KnowledgeGraph, TokenSequence } from "./types.js";

export interface NLParseResult {
  entities: EntityId[];
  predicates: string[];
}

// Simple rule-based phrase to entity/predicate mapper for demo purposes only
export function parseToEntitiesAndPredicates(
  text: string,
  graph: KnowledgeGraph
): NLParseResult {
  const lower = text.toLowerCase();
  const entities: EntityId[] = [];
  const predicates: string[] = [];

  for (const [id, ent] of graph.entities.entries()) {
    const parts = [
      ent.label.toLowerCase(),
      ...ent.types.map((t) => t.toLowerCase()),
    ];
    if (parts.some((p) => lower.includes(p))) {
      entities.push(id);
    }
  }

  const predicateLexicon: Record<string, string> = {
    "located in": "locatedIn",
    in: "locatedIn",
    "instance of": "instanceOf",
    "type of": "instanceOf",
    "has unit": "hasUnit",
    unit: "hasUnit",
  };

  for (const [phrase, pred] of Object.entries(predicateLexicon)) {
    if (lower.includes(phrase)) predicates.push(pred);
  }

  return { entities, predicates };
}

export function sequenceFromNL(
  text: string,
  graph: KnowledgeGraph
): TokenSequence {
  const { entities } = parseToEntitiesAndPredicates(text, graph);
  return entities;
}
