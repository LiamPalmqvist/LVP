//! Generates `Icon`, an enum over every SVG in assets/icons, with the SVG
//! text embedded.

use std::fmt::Write;
use std::path::Path;

fn variant(stem: &str) -> String {
    let mut out = String::new();
    for part in stem.split(['-', '_', ' ']).filter(|p| !p.is_empty()) {
        let mut c = part.chars();
        if let Some(f) = c.next() {
            out.extend(f.to_uppercase());
            out.push_str(c.as_str());
        }
    }
    if out.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        out.insert(0, 'N');
    }
    out
}

fn main() {
    let dir = Path::new("assets/icons");
    println!("cargo:rerun-if-changed=assets/icons");
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .expect("assets/icons")
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            if p.extension()? == "svg" { Some(p.file_stem()?.to_string_lossy().into_owned()) } else { None }
        })
        .collect();
    names.sort();
    let mut s = String::new();
    s.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]\npub enum Icon {\n");
    for n in &names {
        writeln!(s, "    {},", variant(n)).unwrap();
    }
    s.push_str("}\n\nimpl Icon {\n    pub const ALL: &'static [Icon] = &[\n");
    for n in &names {
        writeln!(s, "        Icon::{},", variant(n)).unwrap();
    }
    s.push_str("    ];\n\n    pub fn name(self) -> &'static str {\n        match self {\n");
    for n in &names {
        writeln!(s, "            Icon::{} => {:?},", variant(n), n).unwrap();
    }
    s.push_str("        }\n    }\n\n    pub fn svg(self) -> &'static str {\n        match self {\n");
    let abs = std::fs::canonicalize(dir).unwrap();
    for n in &names {
        let path = abs.join(format!("{n}.svg"));
        writeln!(s, "            Icon::{} => include_str!({:?}),", variant(n), path.display().to_string()).unwrap();
    }
    s.push_str("        }\n    }\n}\n");
    let out = Path::new(&std::env::var("OUT_DIR").unwrap()).join("icons.rs");
    std::fs::write(out, s).unwrap();
}
