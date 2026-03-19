use units::{Quantity, FixI128};
use units::defs as U;
use entity::{Registry, EntityTypeId, PropertyId, EntityRef, EntityKey, Value};
use store::KV;

fn put_q(kv: &mut KV, reg: &Registry, ty: EntityTypeId, key: &str, prop: PropertyId, q: Quantity) {
    let k = reg.canonical_key(ty, key, prop);
    kv.put(k, entity::codec::encode(&Value::Quantity(q)));
}
fn put_i(kv: &mut KV, reg: &Registry, ty: EntityTypeId, key: &str, prop: PropertyId, i: i128) {
    let k = reg.canonical_key(ty, key, prop);
    kv.put(k, entity::codec::encode(&Value::Integer(i)));
}
fn put_ref(kv: &mut KV, reg: &Registry, ty: EntityTypeId, key: &str, prop: PropertyId, r: EntityRef) {
    let k = reg.canonical_key(ty, key, prop);
    kv.put(k, entity::codec::encode(&Value::EntityRef(r)));
}
fn put_list(kv: &mut KV, reg: &Registry, ty: EntityTypeId, key: &str, prop: PropertyId, v: Value) {
    let k = reg.canonical_key(ty, key, prop);
    kv.put(k, entity::codec::encode(&v));
}

pub fn seed_demo_data(kv: &mut KV, reg: &Registry) {
    // ----- Foods -----
    // Salad: 44 kcal/100g; 150 g per serving
    put_q(kv, reg, reg.ty_food, "Salad", reg.p_energy_per_100g,
        Quantity{ val: FixI128::from_i64(44), unit: U::KCAL } );
    put_q(kv, reg, reg.ty_food, "Salad", reg.p_mass_per_serving,
        Quantity{ val: FixI128::from_i64(150), unit: U::G } );

    // Banana: 89 kcal/100g; 118 g serving (approx demo)
    put_q(kv, reg, reg.ty_food, "Banana", reg.p_energy_per_100g,
        Quantity{ val: FixI128::from_i64(89), unit: U::KCAL } );
    put_q(kv, reg, reg.ty_food, "Banana", reg.p_mass_per_serving,
        Quantity{ val: FixI128::from_i64(118), unit: U::G } );

    // GreekSalad recipe: (Lettuce 70g, Tomato 80g, Feta 30g, OliveOil 15g) — demo recipe values
    // We'll store as a formula-style list of (entity,count) but for foods we skip aggregation demo here.

    // ----- Countries / Cities -----
    // France
    put_q(kv, reg, reg.ty_country, "France", reg.p_area_km2,
        // area = 551_695 km^2
        Quantity{ val: FixI128::from_i64(551_695), unit: U::KM2 } );
    put_i(kv, reg, reg.ty_country, "France", reg.p_population, 67_000_000);
    put_ref(kv, reg, reg.ty_country, "France", reg.p_capital,
        entity::EntityRef{ ty: reg.ty_city, key: EntityKey("Paris".into()) });

    // USA
    put_q(kv, reg, reg.ty_country, "UnitedStates", reg.p_area_km2,
        Quantity{ val: FixI128::from_i64(9_833_520), unit: U::KM2 } );
    put_i(kv, reg, reg.ty_country, "UnitedStates", reg.p_population, 331_000_000);
    put_ref(kv, reg, reg.ty_country, "UnitedStates", reg.p_capital,
        entity::EntityRef{ ty: reg.ty_city, key: EntityKey("WashingtonDC".into()) });

    // India
    put_q(kv, reg, reg.ty_country, "India", reg.p_area_km2,
        Quantity{ val: FixI128::from_i64(3_287_263), unit: U::KM2 } );
    put_i(kv, reg, reg.ty_country, "India", reg.p_population, 1_400_000_000);
    put_ref(kv, reg, reg.ty_country, "India", reg.p_capital,
        entity::EntityRef{ ty: reg.ty_city, key: EntityKey("NewDelhi".into()) });

    // Cities (minimal)
    put_ref(kv, reg, reg.ty_city, "Paris", reg.p_in_country,
        entity::EntityRef{ ty: reg.ty_country, key: EntityKey("France".into()) });
    put_ref(kv, reg, reg.ty_city, "WashingtonDC", reg.p_in_country,
        entity::EntityRef{ ty: reg.ty_country, key: EntityKey("UnitedStates".into()) });
    put_ref(kv, reg, reg.ty_city, "NewDelhi", reg.p_in_country,
        entity::EntityRef{ ty: reg.ty_country, key: EntityKey("India".into()) });

    // ----- Elements -----
    // Hydrogen
    put_i(kv, reg, reg.ty_element, "H", reg.p_atomic_number, 1);
    put_q(kv, reg, reg.ty_element, "H", reg.p_atomic_weight_g_per_mol,
        Quantity{ val: FixI128::from_ratio(100784, 100000), unit: U::G_PER_MOL }); // 1.00784 g/mol

    // Oxygen
    put_i(kv, reg, reg.ty_element, "O", reg.p_atomic_number, 8);
    put_q(kv, reg, reg.ty_element, "O", reg.p_atomic_weight_g_per_mol,
        Quantity{ val: FixI128::from_ratio(15999, 1000), unit: U::G_PER_MOL }); // 15.999 g/mol

    // Iron
    put_i(kv, reg, reg.ty_element, "Fe", reg.p_atomic_number, 26);
    put_q(kv, reg, reg.ty_element, "Fe", reg.p_atomic_weight_g_per_mol,
        Quantity{ val: FixI128::from_ratio(55845, 1000), unit: U::G_PER_MOL }); // 55.845 g/mol

    // ----- Materials -----
    // Water H2O
    let h_ref = entity::EntityRef{ ty: reg.ty_element, key: entity::EntityKey("H".into()) };
    let o_ref = entity::EntityRef{ ty: reg.ty_element, key: entity::EntityKey("O".into()) };
    let h_term = Value::Record(vec![(1, Value::EntityRef(h_ref.clone())), (2, Value::Integer(2))]);
    let o_term = Value::Record(vec![(1, Value::EntityRef(o_ref.clone())), (2, Value::Integer(1))]);
    put_list(kv, reg, reg.ty_material, "Water", reg.p_formula, Value::List(vec![h_term, o_term]));
    // density ~ 1000 kg/m^3
    put_q(kv, reg, reg.ty_material, "Water", reg.p_density, Quantity{ val: FixI128::from_i64(1000), unit: U::KG_PER_M3 });

    // Iron (Fe)
    let fe_ref = entity::EntityRef{ ty: reg.ty_element, key: entity::EntityKey("Fe".into()) };
    let fe_term = Value::Record(vec![(1, Value::EntityRef(fe_ref.clone())), (2, Value::Integer(1))]);
    put_list(kv, reg, reg.ty_material, "Iron", reg.p_formula, Value::List(vec![fe_term]));
    put_q(kv, reg, reg.ty_material, "Iron", reg.p_density, Quantity{ val: FixI128::from_i64(7874), unit: U::KG_PER_M3 });

    // ----- Physics demo -----
    // Body of 1 kg
    put_q(kv, reg, reg.ty_body, "Body1kg", reg.p_mass, Quantity{ val: FixI128::from_i64(1), unit: U::KG });
}