use rustc_hash::FxHashMap;

#[derive(Debug, Default, Clone)]
pub struct Interner {
    forward: FxHashMap<String, u32>,
    backward: FxHashMap<u32, String>,
    next_id: u32,
}

impl Interner {
    pub fn intern(&mut self, s: String) -> u32 {
        if let Some(&id) = self.forward.get(&s) {
            return id;
        }
        let id = self.next_id;
        self.next_id += 1;
        self.forward.insert(s.clone(), id);
        self.backward.insert(id, s);
        id
    }

    pub fn lookup(&self, id: u32) -> Option<&str> {
        self.backward.get(&id).map(|s| s.as_str())
    }
}
