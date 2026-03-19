use store::KV;

/// Minimal deterministic VM interface for "on-chain" system contracts.
pub trait Contract {
    /// Execute contract method with call data, possibly mutating state.
    /// Returns return bytes (e.g., encoded Value) for read methods.
    fn call(&mut self, state: &mut KV, method: u32, calldata: &[u8]) -> Vec<u8>;
}

pub struct Vm<'a> {
    pub state: KV,
    pub contracts: Vec<(&'a str, u32, Box<dyn Contract>)>, // (name, id, handler)
}

impl<'a> Vm<'a> {
    pub fn new() -> Self { Self { state: KV::new(), contracts: Vec::new() } }

    pub fn register(&mut self, name: &'a str, id: u32, c: Box<dyn Contract>) {
        self.contracts.push((name, id, c));
    }

    pub fn call(&mut self, id: u32, method: u32, calldata: &[u8]) -> Vec<u8> {
        for (_, cid, c) in self.contracts.iter_mut() {
            if *cid == id {
                return c.call(&mut self.state, method, calldata);
            }
        }
        Vec::new()
    }

    pub fn state_root(&self) -> [u8;32] { self.state.ordered_merkle_root() }
}