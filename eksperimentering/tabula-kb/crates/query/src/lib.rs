use entity::{Registry, EntityTypeId, PropertyId, EntityRef, EntityKey, Value};
use eval::{Getter, StoreGetter, eval_property};
use store::KV;

#[derive(Debug)]
pub struct Query { pub ty_name: String, pub key: String, pub prop_name: String }

pub fn parse(q: &str) -> Option<Query> {
    // Format: type:key.property  (key may be quoted with ")
    let mut parts = q.rsplitn(2, '.'); // split at last dot
    let prop_name = parts.next()?.to_string();
    let left = parts.next()?;
    let mut tp = left.splitn(2, ':');
    let ty_name = tp.next()?.to_string();
    let key_raw = tp.next()?;
    let key = if key_raw.starts_with('"') && key_raw.ends_with('"') && key_raw.len()>=2 {
        key_raw[1..key_raw.len()-1].to_string()
    } else {
        key_raw.to_string()
    };
    Some(Query{ ty_name, key, prop_name })
}

pub fn resolve<'a>(kv: &'a KV, reg: &'a Registry, q: &Query) -> Option<Value> {
    let (ty, prop) = (ty_by_name(&q.ty_name, reg)?, prop_by_name(&q.prop_name, reg)?);
    let ent = EntityRef{ ty, key: EntityKey(q.key.clone()) };
    let getter = StoreGetter{ kv, reg };
    // First try stored; else computed
    eval_property(&getter, reg, &ent, prop)
        .or_else(|| getter.get_prop(&ent, prop))
}

fn ty_by_name(name: &str, reg: &Registry) -> Option<EntityTypeId> {
    match name.to_ascii_lowercase().as_str() {
        "food" => Some(reg.ty_food),
        "country" => Some(reg.ty_country),
        "city" => Some(reg.ty_city),
        "element" => Some(reg.ty_element),
        "material" => Some(reg.ty_material),
        "body" => Some(reg.ty_body),
        "constant" => Some(reg.ty_constant),
        _ => None
    }
}

fn prop_by_name(name: &str, reg: &Registry) -> Option<PropertyId> {
    match name.to_ascii_lowercase().as_str() {
        // food
        "energyper100g" => Some(reg.p_energy_per_100g),
        "massperserving" => Some(reg.p_mass_per_serving),
        "energyperserving" => Some(reg.p_energy_per_serving),
        "recipe" => Some(reg.p_recipe),
        // country
        "area_km2" | "areakm2" => Some(reg.p_area_km2),
        "population" => Some(reg.p_population),
        "capital" => Some(reg.p_capital),
        "populationdensity" => Some(reg.p_population_density),
        // city
        "incountry" => Some(reg.p_in_country),
        // element
        "symbol" => Some(reg.p_symbol),
        "atomicnumber" => Some(reg.p_atomic_number),
        "atomicweight" | "atomicweight_g_per_mol" => Some(reg.p_atomic_weight_g_per_mol),
        // material
        "formula" => Some(reg.p_formula),
        "density" => Some(reg.p_density),
        "molarmass" => Some(reg.p_molar_mass),
        // body
        "mass" => Some(reg.p_mass),
        "restenergy" => Some(reg.p_rest_energy),
        // constant
        "value" => Some(reg.p_value),
        _ => None
    }
}