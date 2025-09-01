import {
  EntityId,
  EntityToken,
  KnowledgeGraph,
  RelationEdge,
  GraphQuery,
  QueryResultPath,
} from "./types.js";

export function createGraph(
  initialEntities?: EntityToken[],
  initialEdges?: RelationEdge[]
): KnowledgeGraph {
  const entities = new Map<EntityId, EntityToken>();
  const adjacency = new Map<EntityId, RelationEdge[]>();
  if (initialEntities) {
    for (const e of initialEntities) entities.set(e.id, e);
  }
  if (initialEdges) {
    for (const edge of initialEdges) addEdge({ entities, adjacency }, edge);
  }
  return { entities, adjacency };
}

export function addEntity(graph: KnowledgeGraph, entity: EntityToken): void {
  graph.entities.set(entity.id, entity);
}

export function addEdge(graph: KnowledgeGraph, edge: RelationEdge): void {
  if (!graph.entities.has(edge.subject) || !graph.entities.has(edge.object)) {
    throw new Error(
      `Edge endpoints must exist: ${edge.subject} -> ${edge.predicate} -> ${edge.object}`
    );
  }
  const list = graph.adjacency.get(edge.subject) ?? [];
  list.push(edge);
  graph.adjacency.set(edge.subject, list);
}

export function neighborsByPredicate(
  graph: KnowledgeGraph,
  node: EntityId,
  predicate?: string
): RelationEdge[] {
  const list = graph.adjacency.get(node) ?? [];
  if (!predicate) return list;
  return list.filter((e) => e.predicate === predicate);
}

export function runPathQuery(
  graph: KnowledgeGraph,
  query: GraphQuery
): QueryResultPath[] {
  if (query.start == null) return [];
  const results: QueryResultPath[] = [];
  const start = query.start;

  function dfs(
    current: EntityId,
    depth: number,
    pathNodes: EntityId[],
    pathEdges: RelationEdge[]
  ): void {
    if (depth === query.predicates.length || depth === query.maxDepth) {
      results.push({ nodes: [...pathNodes], edges: [...pathEdges] });
      return;
    }
    const predicate = query.predicates[depth];
    const out = neighborsByPredicate(graph, current, predicate);
    for (const edge of out) {
      dfs(
        edge.object,
        depth + 1,
        [...pathNodes, edge.object],
        [...pathEdges, edge]
      );
    }
  }

  dfs(start, 0, [start], []);
  return results;
}
