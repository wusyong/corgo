use std::sync::Arc;

use atoms::{Atom, AtomIndex};
use log::{trace, warn};
use scc::HashMap;

pub struct Runtime {
    atoms: Arc<HashMap<AtomIndex, Atom>>,
    // TODO Define JS Class
    classes: Arc<HashMap<usize, u8>>,
}

impl Runtime {
    pub fn new() -> Self {
        trace!(target: "Runtime", "Create new runtime.");
        Self {
            atoms: Arc::new(atoms::init()),
            classes: Arc::new(HashMap::default()),
        }
    }

    pub fn context(&self) -> Context {
        trace!(target: "Runtime", "Create new context.");
        Context {
            atoms: self.atoms.clone(),
            classes: self.classes.clone(),
        }
    }

    pub fn atom(&self, name: impl AsRef<str>) -> u32 {
        let name = name.as_ref();
        let atom = Atom::from(name);
        dbg!(&atom, &atom.unsafe_data(), &atom.get_hash());

        // self.atoms.read(name, |_, v| v.clone()).unwrap_or_else(|| {
        //     let atom = DefaultAtom::from(name);
        //     match self.atoms.insert(name.to_string(), atom.clone()) {
        //         Ok(_) => atom,
        //         Err((_, atom)) => {
        //             // This is extreme unlikely to happen unless CPU ordering goes wrong.
        //             // Return the supplied atom because we assume the atom in the map is the same.
        //             warn!(target: "Runtime", "Insert atom failed. Return supplied atom.");
        //             atom
        //         },
        //     }
        // })
        atom.get_hash()
    }
}

pub struct Context {
    atoms: Arc<HashMap<AtomIndex, Atom>>,
    classes: Arc<HashMap<usize, u8>>,
}
