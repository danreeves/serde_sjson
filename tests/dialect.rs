//! The toolchain's own declaration dialect, as the `.shader_node` files write
//! it. Three things there are looser than this crate was upstream, and the
//! material files - which are strict SJSON - are unaffected by all three:
//!
//! - `key: value` separates as well as `key = value`, sometimes in the same table;
//! - a table's entries do not have to be on their own lines;
//! - a key may be a quoted string, which is how the toolchain writes the keys of
//!   its define tables (`"macros": [...]`).

use std::collections::BTreeMap;

use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
struct Fragment {
    #[serde(default)]
    permutation_sets: BTreeMap<String, Vec<Choice>>,
}

#[derive(Debug, Default, Deserialize)]
struct Choice {
    #[serde(rename = "if", default)]
    condition: Option<String>,
    #[serde(default)]
    define: Option<Define>,
    #[serde(rename = "default", default)]
    is_default: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
struct Define {
    #[serde(default)]
    macros: Vec<String>,
    #[serde(default)]
    stages: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
struct Inputs {
    #[serde(default)]
    inputs: BTreeMap<String, Input>,
}

#[derive(Debug, Default, Deserialize)]
struct Input {
    name: String,
    #[serde(rename = "type")]
    kind: BTreeMap<String, Vec<String>>,
}

#[test]
fn reads_a_colon_separated_table() {
    #[derive(Debug, Default, Deserialize)]
    struct Root {
        #[serde(default)]
        contexts: BTreeMap<String, BTreeMap<String, String>>,
    }
    let root: Root = serde_sjson::from_str(
        "contexts = {\n\tbase = {\n\t\tpasses_sort_mode: \"immediate\"\n\t}\n}",
    )
    .expect("parse");
    assert_eq!(root.contexts["base"]["passes_sort_mode"], "immediate");
}

#[test]
fn reads_a_quoted_key() {
    // A derived struct reads its field names as identifiers, and the toolchain
    // quotes the keys of a define table.
    #[derive(Debug, Default, Deserialize)]
    struct Root {
        #[serde(default)]
        define: Define,
    }
    let root: Root =
        serde_sjson::from_str("define = { \"macros\": [\"A\"] \"stages\": [\"vertex\"] }")
            .expect("parse");
    assert_eq!(root.define.macros, vec!["A".to_string()]);
    assert_eq!(root.define.stages, vec!["vertex".to_string()]);
}

#[test]
fn reads_a_type_table() {
    // How an input declares its type and the permutation flags that enable it.
    let inputs: Inputs = serde_sjson::from_str(
        "inputs = {\n\t\"1\" = {\n\t\tname = \"base_color\"\n\t\ttype = { vector3: [\"HAS_BASE_COLOR\"] }\n\t}\n}",
    )
    .expect("parse");
    let input = &inputs.inputs["1"];
    assert_eq!(input.name, "base_color");
    assert_eq!(input.kind["vector3"], vec!["HAS_BASE_COLOR".to_string()]);
}

#[test]
fn reads_packed_choices() {
    // A permutation set as the toolchain writes it: `:` separators, a define
    // table packed onto one line, and a `default` choice.
    let fragment: Fragment = serde_sjson::from_str(
        r#"permutation_sets = {
    vertex_modifiers = [
        { if: "num_skin_weights() == 4" define: { "macros": ["SKINNED_4WEIGHTS"] stages: ["vertex"] } }
        { default = true }
    ]
}"#,
    )
    .expect("parse");
    let choices = &fragment.permutation_sets["vertex_modifiers"];
    assert_eq!(choices.len(), 2);
    assert_eq!(
        choices[0].condition.as_deref(),
        Some("num_skin_weights() == 4")
    );
    let define = choices[0].define.as_ref().expect("define");
    assert_eq!(define.macros, vec!["SKINNED_4WEIGHTS".to_string()]);
    assert_eq!(define.stages, vec!["vertex".to_string()]);
    assert_eq!(choices[0].is_default, None);
    assert_eq!(choices[1].is_default, Some(true));
}

#[test]
fn reads_an_equals_separated_define() {
    let fragment: Fragment = serde_sjson::from_str(
        "permutation_sets = {\n\tset = [\n\t\t{ if: \"instanced()\" define = { \"macros\": [\"INSTANCED\"] } }\n\t\t{ default = true }\n\t]\n}",
    )
    .expect("parse");
    let choices = &fragment.permutation_sets["set"];
    let define = choices[0].define.as_ref().expect("define");
    assert_eq!(define.macros, vec!["INSTANCED".to_string()]);
    // No stages means every stage.
    assert!(define.stages.is_empty());
}

#[test]
fn a_strict_file_still_parses() {
    // The material files are strict SJSON, so loosening the grammar must not
    // change how they read.
    let fragment: Fragment =
        serde_sjson::from_str("permutation_sets = {\n\tset = [\n\t\t{ default = true }\n\t]\n}\n")
            .expect("parse");
    assert_eq!(fragment.permutation_sets["set"].len(), 1);
}
