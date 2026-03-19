use vm::Contract;
use store::KV;
use entity::{Registry, Value};
use eval::{StoreGetter, eval_property};
use query::{parse, resolve};

/// KB system contract:
/// method 1: seed_demo_data()  -> mutates state
/// method 2: query(q: utf8 string) -> returns encoded Value
pub struct KbContract {
    reg: Registry,
}

impl KbContract {
    pub fn new() -> Self { Self { reg: Registry::default() } }
}

impl Contract for KbContract {
    fn call(&mut self, state: &mut KV, method: u32, calldata: &[u8]) -> Vec<u8> {
        match method {
            1 => { // seed
                ingest::seed_demo_data(state, &self.reg);
                Vec::new()
            },
            2 => { // query
                let s = std::str::from_utf8(calldata).unwrap_or("");
                if let Some(q) = parse(s) {
                    if let Some(v) = resolve(state, &self.reg, &q) {
                        return entity::codec::encode(&v);
                    }
                }
                Vec::new()
            },
            _ => Vec::new()
        }
    }
}