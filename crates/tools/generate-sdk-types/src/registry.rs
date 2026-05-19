use crate::schema::TypeDef;
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

/// Collects type definitions, deduplicates by name, and emits them
/// in dependency order (enums first, then structs in topological order).
pub struct TypeRegistry {
    types: BTreeMap<String, TypeDef>,
}

impl TypeRegistry {
    pub fn new() -> Self {
        Self {
            types: BTreeMap::new(),
        }
    }

    /// Insert a type definition. If the name already exists, verify the new
    /// definition is identical (by debug representation). Panics on conflict.
    pub fn insert(&mut self, typedef: TypeDef) {
        let name = typedef.name().to_string();
        if let Some(existing) = self.types.get(&name) {
            let existing_dbg = format!("{existing:?}");
            let new_dbg = format!("{typedef:?}");
            if existing_dbg != new_dbg {
                panic!(
                    "Conflicting definitions for type '{name}':\n\
                     Existing: {existing_dbg}\n\
                     New:      {new_dbg}"
                );
            }
            // Identical — skip.
            return;
        }
        self.types.insert(name, typedef);
    }

    /// Return all types in dependency order. Enums and string types (no deps)
    /// come first, followed by structs/unions sorted topologically.
    pub fn into_sorted(self) -> Vec<TypeDef> {
        let names: Vec<String> = self.types.keys().cloned().collect();
        let mut adj: HashMap<String, BTreeSet<String>> = HashMap::new();
        let mut in_degree: HashMap<String, usize> = HashMap::new();

        for name in &names {
            adj.entry(name.clone()).or_default();
            in_degree.entry(name.clone()).or_insert(0);
        }

        // Build edges: if type A depends on type B, A has an edge to B
        // meaning B must come before A.
        for name in &names {
            let typedef = &self.types[name];
            for dep in typedef.dependencies() {
                if self.types.contains_key(&dep) && dep != *name {
                    adj.entry(dep.clone()).or_default().insert(name.clone());
                    *in_degree.entry(name.clone()).or_insert(0) += 1;
                }
            }
        }

        // Kahn's algorithm for topological sort
        let mut queue: VecDeque<String> = VecDeque::new();
        for name in &names {
            if in_degree[name] == 0 {
                queue.push_back(name.clone());
            }
        }

        let mut result: Vec<TypeDef> = Vec::new();
        while let Some(name) = queue.pop_front() {
            if let Some(typedef) = self.types.get(&name) {
                result.push(typedef.clone());
            }
            if let Some(dependents) = adj.get(&name) {
                for dep in dependents {
                    let deg = in_degree.get_mut(dep).expect("in_degree entry");
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(dep.clone());
                    }
                }
            }
        }

        if result.len() != names.len() {
            let emitted: BTreeSet<String> = result.iter().map(|t| t.name().to_string()).collect();
            let missing: Vec<&String> = names.iter().filter(|n| !emitted.contains(*n)).collect();
            panic!("Cycle detected in type dependencies: {missing:?}");
        }

        result
    }
}
