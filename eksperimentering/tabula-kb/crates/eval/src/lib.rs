use entity::{EntityRef, PropertyId, Value, Registry};
use units::{Quantity, FixI128};
use units::defs as U;
use store::KV;

pub trait Getter {
    fn get_prop(&self, ent: &EntityRef, prop: PropertyId) -> Option<Value>;
}

pub struct StoreGetter<'a> {
    pub kv: &'a KV,
    pub reg: &'a Registry,
}

impl<'a> Getter for StoreGetter<'a> {
    fn get_prop(&self, ent: &EntityRef, prop: PropertyId) -> Option<Value> {
        let key = self.reg.canonical_key(ent.ty, &ent.key.0, prop);
        self.kv.get(&key).map(|bytes| entity::codec::decode(bytes))
    }
}

/// Dispatch computed properties deterministically.
pub fn eval_property(get: &dyn Getter, reg: &Registry, subject: &EntityRef, prop: PropertyId) -> Option<Value> {
    // If stored value exists, return it
    if let Some(v) = get.get_prop(subject, prop) { return Some(v); }

    // Otherwise computed ones
    match (subject.ty.0, prop.0) {
        // Food: EnergyPerServing = (MassPerServing in g /100) * EnergyPer100g
        (1, 102) => {
            let e100 = match get.get_prop(subject, reg.p_energy_per_100g)? { Value::Quantity(q)=>q, _=>return None };
            let mass = match get.get_prop(subject, reg.p_mass_per_serving)? { Value::Quantity(q)=>q, _=>return None };
            let mass_g = mass.convert_to(U::G)?;
            let factor = mass_g.val.div(FixI128::from_i64(100));
            let val = e100.val.mul(factor);
            Some(Value::Quantity(Quantity{ val, unit: e100.unit }))
        },
        // Country: PopulationDensity (1/km^2) = population / area(km^2) in base per m^2 then to per km^2
        (2, 203) => {
            let pop = match get.get_prop(subject, reg.p_population)? { Value::Integer(i)=>i, _=>return None };
            let area = match get.get_prop(subject, reg.p_area_km2)? { Value::Quantity(q)=>q, _=>return None };
            let area_m2 = area.convert_to(U::M2)?; // km^2 to m^2 base
            // population is dimensionless; density (per m^2) = pop / area_m2
            let pop_fix = FixI128::from_i64(pop as i64); // if > i64, would need checked path
            let per_m2_val = pop_fix.div(area_m2.val);
            // convert to per km^2 by multiplying by 1e6 via unit conversion
            let per_m2 = Quantity{ val: per_m2_val, unit: units::Unit{ dim: units::defs::pow_dim(U::M.dim,-2), scale: FixI128::ONE } };
            Some(Value::Quantity(per_m2.convert_to(U::PER_KM2)?))
        },
        // Material: MolarMass = sum(count * Element.AtomicWeight)
        (5, 502) => {
            let formula = match get.get_prop(subject, reg.p_formula)? { Value::List(xs) => xs, _=> return None };
            let mut sum = FixI128(0);
            for term in formula {
                // Each term is Record{ 1: element(EntityRef), 2: count(Integer) }
                let (mut element_ref, mut count): (Option<EntityRef>, Option<i128>) = (None, None);
                if let Value::Record(fields) = term {
                    for (fid, v) in fields {
                        match (fid, v) {
                            (1, Value::EntityRef(r)) => element_ref = Some(r.clone()),
                            (2, Value::Integer(i)) => count = Some(i),
                            _ => {}
                        }
                    }
                }
                let el = element_ref?;
                let n = count.unwrap_or(1);
                let w = match get.get_prop(&el, reg.p_atomic_weight_g_per_mol)? { Value::Quantity(q)=>q, _=>return None };
                // ensure g/mol
                let w_gpm = w.convert_to(U::G_PER_MOL)?;
                let term_val = w_gpm.val.mul(FixI128::from_i64(n as i64));
                sum = FixI128(sum.0 + term_val.0);
            }
            Some(Value::Quantity(Quantity{ val: sum, unit: U::G_PER_MOL }))
        },
        // Body: RestEnergy = mass(kg) * c^2  (c = 299792458 m/s exact), result in Joules
        (6, 601) => {
            let mass = match get.get_prop(subject, reg.p_mass)? { Value::Quantity(q)=>q, _=>return None };
            let mass_kg = mass.convert_to(U::KG)?;
            let c = 299_792_458i64;
            let cfix = FixI128::from_i64(c);
            let c2 = cfix.mul(cfix);
            let val = mass_kg.val.mul(c2);
            Some(Value::Quantity(Quantity{ val, unit: U::J }))
        },
        _ => None
    }
}