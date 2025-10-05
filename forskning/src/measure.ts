import { EntityId, MeasurementStore, UnitValue } from "./types.js";

export function createMeasurementStore(): MeasurementStore {
  return { byEntity: new Map() };
}

export function setMeasurement(
  store: MeasurementStore,
  entityId: EntityId,
  property: string,
  value: UnitValue
): void {
  const m = store.byEntity.get(entityId) ?? new Map<string, UnitValue>();
  m.set(property, value);
  store.byEntity.set(entityId, m);
}

export function getMeasurement(
  store: MeasurementStore,
  entityId: EntityId,
  property: string
): UnitValue | undefined {
  return store.byEntity.get(entityId)?.get(property);
}
