use std::{env, path::Path};

fn main() {
    let atoms = include_str!("atoms.txt")
        .lines()
        .map(|l| l.trim())
        .collect::<Vec<_>>();
    string_cache_codegen::AtomType::new("Atom", "atom!")
        .atoms(atoms)
        .write_to_file(&Path::new(&env::var("OUT_DIR").unwrap()).join("atom.rs"))
        .unwrap();
}
