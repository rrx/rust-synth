use std::collections::HashMap;

pub struct Parameters<R> {
    h: HashMap<String, R>,
    defaults: HashMap<String, R>
}
impl<R> Default for Parameters<R> {
    fn default() -> Self {
        Self { h: HashMap::new(), defaults: HashMap::new() }
    }
}
impl<R> Parameters<R> 
    where R: dasp::sample::Sample
    {
    pub fn get(&self, key: &str) -> R {
        match self.h.get(key) {
            Some(v) => *v,
            None => {
                match self.defaults.get(key) {
                    Some(v) => *v,
                    None => R::EQUILIBRIUM
                }
            }
        }
    } 
    pub fn update(&mut self, key: &str, value: &R) {
        if !self.defaults.contains_key(key) {
            self.defaults.insert(key.to_string(), *value);
        }
        self.h.insert(key.to_string(), *value);
    }
}


