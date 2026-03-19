use units::{Quantity, Unit, FixI128};
use core::fmt;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct EntityTypeId(pub u32);

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct EntityKey(pub String);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PropertyId(pub u32);

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Value {
    Text(String),
    Quantity(Quantity),
    Integer(i128),
    Bool(bool),
    EntityRef(EntityRef),
    List(Vec<Value>),
    Record(Vec<(u32, Value)>), // field-id sorted ascending
    None,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct EntityRef { pub ty: EntityTypeId, pub key: EntityKey }

/// Canonical codec (self-describing, deterministic). Not intended for general-purpose use.
pub mod codec {
    use super::*;

    pub fn encode(v: &Value) -> Vec<u8> {
        let mut out = Vec::new();
        enc_value(v, &mut out);
        out
    }

    fn enc_u8(b: u8, out: &mut Vec<u8>) { out.push(b); }
    fn enc_u32(x: u32, out: &mut Vec<u8>) { out.extend_from_slice(&x.to_be_bytes()); }
    fn enc_i128(x: i128, out: &mut Vec<u8>) { out.extend_from_slice(&x.to_be_bytes()); }

    fn enc_bytes(b: &[u8], out: &mut Vec<u8>) {
        enc_u32(b.len() as u32, out);
        out.extend_from_slice(b);
    }

    fn enc_text(s: &str, out: &mut Vec<u8>) {
        enc_bytes(s.as_bytes(), out);
    }

    fn enc_quantity(q: &Quantity, out: &mut Vec<u8>) {
        // tag Quantity=4
        enc_u8(4, out);
        enc_i128(q.val.0, out);
        // unit: dims then scale
        let d = q.unit.dim;
        enc_u8(d.m as u8, out); enc_u8(d.kg as u8, out); enc_u8(d.s as u8, out);
        enc_u8(d.A as u8, out); enc_u8(d.K as u8, out); enc_u8(d.mol as u8, out); enc_u8(d.cd as u8, out);
        enc_i128(q.unit.scale.0, out);
    }

    fn enc_value(v: &Value, out: &mut Vec<u8>) {
        match v {
            Value::None => enc_u8(0, out),
            Value::Integer(i) => { enc_u8(1, out); enc_i128(*i, out); },
            Value::Bool(b) => { enc_u8(2, out); enc_u8(if *b {1} else {0}, out); },
            Value::Text(s) => { enc_u8(3, out); enc_text(s, out); },
            Value::Quantity(q) => { enc_quantity(q, out); },
            Value::EntityRef(r) => {
                enc_u8(5, out);
                enc_u32(r.ty.0, out);
                enc_text(&r.key.0, out);
            },
            Value::List(vs) => {
                enc_u8(6, out);
                enc_u32(vs.len() as u32, out);
                for x in vs { enc_value(x, out); }
            },
            Value::Record(fields) => {
                enc_u8(7, out);
                enc_u32(fields.len() as u32, out);
                // assume sorted by field id
                for (fid, val) in fields {
                    enc_u32(*fid, out);
                    enc_value(val, out);
                }
            },
        }
    }

    // Minimal decoding for internal use; unwraps on malformed data (trusted ingestion)
    pub fn decode(mut bytes: &[u8]) -> Value {
        fn read_u8(b: &mut &[u8]) -> u8 { let (h,t)=b.split_at(1); *b=t; h[0] }
        fn read_u32(b: &mut &[u8]) -> u32 { let (h,t)=b.split_at(4); *b=t; u32::from_be_bytes([h[0],h[1],h[2],h[3]]) }
        fn read_i128(b:&mut &[u8]) -> i128 { let (h,t)=b.split_at(16); *b=t; i128::from_be_bytes(h.try_into().unwrap()) }
        fn read_bytes(b:&mut &[u8]) -> Vec<u8> { let n=read_u32(b) as usize; let (h,t)=b.split_at(n); *b=t; h.to_vec() }
        fn read_text(b:&mut &[u8]) -> String { String::from_utf8(read_bytes(b)).unwrap() }

        fn dec(b:&mut &[u8]) -> Value {
            let tag = read_u8(b);
            match tag {
                0 => Value::None,
                1 => Value::Integer(read_i128(b)),
                2 => Value::Bool(read_u8(b) != 0),
                3 => Value::Text(read_text(b)),
                4 => {
                    let val = FixI128(read_i128(b));
                    let m = read_u8(b) as i8; let kg = read_u8(b) as i8; let s = read_u8(b) as i8;
                    let A = read_u8(b) as i8; let K = read_u8(b) as i8; let mol = read_u8(b) as i8; let cd = read_u8(b) as i8;
                    let scale = FixI128(read_i128(b));
                    let unit = Unit{ dim: units::Dim{m,kg,s,A,K,mol,cd}, scale };
                    Value::Quantity(Quantity{ val, unit })
                },
                5 => {
                    let ty = read_u32(b);
                    let key = read_text(b);
                    Value::EntityRef(EntityRef{ ty: EntityTypeId(ty), key: EntityKey(key) })
                },
                6 => {
                    let n = read_u32(b) as usize;
                    let mut xs = Vec::with_capacity(n);
                    for _ in 0..n { xs.push(dec(b)); }
                    Value::List(xs)
                },
                7 => {
                    let n = read_u32(b) as usize;
                    let mut fs = Vec::with_capacity(n);
                    for _ in 0..n {
                        let fid = read_u32(b);
                        let val = dec(b);
                        fs.push((fid, val));
                    }
                    Value::Record(fs)
                },
                _ => panic!("unknown tag")
            }
        }
        dec(&mut bytes)
    }
}

/// Public registry of canonical type & property ids.
#[derive(Clone)]
pub struct Registry {
    // Types
    pub ty_food: EntityTypeId,      // 1
    pub ty_country: EntityTypeId,   // 2
    pub ty_city: EntityTypeId,      // 3
    pub ty_element: EntityTypeId,   // 4
    pub ty_material: EntityTypeId,  // 5
    pub ty_body: EntityTypeId,      // 6
    pub ty_constant: EntityTypeId,  // 7

    // Food props
    pub p_energy_per_100g: PropertyId,   // 100
    pub p_mass_per_serving: PropertyId,  // 101
    pub p_energy_per_serving: PropertyId,// 102 (computed)
    pub p_recipe: PropertyId,            // 103

    // Country props
    pub p_area_km2: PropertyId,          // 200
    pub p_population: PropertyId,        // 201
    pub p_capital: PropertyId,           // 202
    pub p_population_density: PropertyId,// 203 (computed)

    // City props
    pub p_in_country: PropertyId,        // 300

    // Element props
    pub p_symbol: PropertyId,            // 400
    pub p_atomic_number: PropertyId,     // 401
    pub p_atomic_weight_g_per_mol: PropertyId, // 402

    // Material props
    pub p_formula: PropertyId,           // 500
    pub p_density: PropertyId,           // 501
    pub p_molar_mass: PropertyId,        // 502 (computed)

    // Body props
    pub p_mass: PropertyId,              // 600
    pub p_rest_energy: PropertyId,       // 601 (computed)

    // Constant props
    pub p_value: PropertyId,             // 700
}

impl Default for Registry {
    fn default() -> Self {
        Self {
            ty_food: EntityTypeId(1),
            ty_country: EntityTypeId(2),
            ty_city: EntityTypeId(3),
            ty_element: EntityTypeId(4),
            ty_material: EntityTypeId(5),
            ty_body: EntityTypeId(6),
            ty_constant: EntityTypeId(7),

            p_energy_per_100g: PropertyId(100),
            p_mass_per_serving: PropertyId(101),
            p_energy_per_serving: PropertyId(102),
            p_recipe: PropertyId(103),

            p_area_km2: PropertyId(200),
            p_population: PropertyId(201),
            p_capital: PropertyId(202),
            p_population_density: PropertyId(203),

            p_in_country: PropertyId(300),

            p_symbol: PropertyId(400),
            p_atomic_number: PropertyId(401),
            p_atomic_weight_g_per_mol: PropertyId(402),

            p_formula: PropertyId(500),
            p_density: PropertyId(501),
            p_molar_mass: PropertyId(502),

            p_mass: PropertyId(600),
            p_rest_energy: PropertyId(601),

            p_value: PropertyId(700),
        }
    }
}

impl Registry {
    /// Canonical state key: ty(4) 0x1F key-bytes 0x1F prop(4)
    pub fn canonical_key(&self, ty: EntityTypeId, key: &str, prop: PropertyId) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&ty.0.to_be_bytes());
        out.push(0x1F);
        // normalize: ascii lower, NFC assumptions left as-is (keep simple demo)
        out.extend_from_slice(key.to_ascii_lowercase().as_bytes());
        out.push(0x1F);
        out.extend_from_slice(&prop.0.to_be_bytes());
        out
    }
}

impl fmt::Display for EntityRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ty:{} key:{}", self.ty.0, self.key.0)
    }
}