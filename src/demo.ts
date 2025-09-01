import { createGraph, addEntity, addEdge } from "./graph.js";
import { followOp, pathOp, dedupeOp, intersectOp } from "./ops.js";
import { sequenceFromNL } from "./nl.js";
import { EntityToken, RelationEdge } from "./types.js";
import { createMeasurementStore, setMeasurement } from "./measure.js";
import { planScaleByFactor } from "./planner.js";

function seedWorld() {
  const entities: EntityToken[] = [
    { id: "taj_mahal", label: "Taj Mahal", types: ["Place", "Monument"] },
    { id: "agra", label: "Agra", types: ["City", "Place"] },
    { id: "india", label: "India", types: ["Country", "Place"] },
    { id: "mass", label: "Mass", types: ["PhysicalQuantity"] },
    { id: "kilogram", label: "Kilogram", types: ["Unit"] },
  ];
  const edges: RelationEdge[] = [
    { subject: "taj_mahal", predicate: "locatedIn", object: "agra" },
    { subject: "agra", predicate: "locatedIn", object: "india" },
    { subject: "mass", predicate: "hasUnit", object: "kilogram" },
  ];
  const graph = createGraph(entities, []);
  for (const e of entities) addEntity(graph, e);
  for (const ed of edges) addEdge(graph, ed);
  return graph;
}

async function main() {
  const graph = seedWorld();
  const measurements = createMeasurementStore();
  // Seed Taj Mahal height ~ 73 m
  setMeasurement(measurements, "taj_mahal", "height", { value: 73, unit: "m" });

  // NL → entity token sequence
  const nl1 = "Where is the Taj Mahal located?";
  const seq1 = sequenceFromNL(nl1, graph);
  console.log("NL seq1:", seq1);

  // Custom op: follow locatedIn
  const firstHop = followOp.run(
    { start: "taj_mahal", predicate: "locatedIn" },
    { graph }
  );
  console.log("firstHop (locatedIn):", firstHop);

  // Path op: locatedIn -> locatedIn
  const paths = pathOp.run(
    { start: "taj_mahal", predicates: ["locatedIn", "locatedIn"], maxDepth: 3 },
    { graph }
  );
  console.log(
    "paths to country:",
    paths.map((p) => p.nodes)
  );

  // Set ops on entity sequences
  const setA = ["agra", "india", "kilogram"];
  const setB = ["india", "kilogram"];
  console.log(
    "intersect:",
    intersectOp.run({ sets: [setA, setB] }, { graph } as any)
  );
  console.log(
    "dedupe:",
    dedupeOp.run(["india", "india", "agra"], { graph } as any)
  );

  // NL example for physics
  const nl2 = "What unit does mass have?";
  const seq2 = sequenceFromNL(nl2, graph);
  console.log("NL seq2:", seq2);
  const units = followOp.run(
    { start: "mass", predicate: "hasUnit" },
    { graph }
  );
  console.log("mass units:", units);

  // Scale-by-factor planner demo
  const q = "how tall would the taj mahal be if we scaled it by a factor of 3";
  const plan = planScaleByFactor(
    graph,
    { entityLabel: "Taj Mahal", property: "height", factor: 3 },
    measurements
  );
  const result = plan.run();
  console.log("scale-by-factor:", {
    original: result.original,
    scaled: result.scaled,
    verbalized: result.scaled
      ? `${result.scaled.value} ${result.scaled.unit}`
      : "unknown",
  });
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
