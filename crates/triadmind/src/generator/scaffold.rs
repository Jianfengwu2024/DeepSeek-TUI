//! Code scaffolding — Rust and TypeScript function templates.

use crate::protocol::TriadFission;

/// Generate a Rust function skeleton from a fission triple.
pub fn generate_rust_function(node_id: &str, fission: &TriadFission) -> String {
    let fn_name = node_id
        .split('.')
        .last()
        .unwrap_or(node_id)
        .to_lowercase();
    let params: Vec<String> = fission
        .demand
        .iter()
        .enumerate()
        .map(|(i, name)| format!("arg{}: {}", i, rust_type_hint(name)))
        .collect();
    let return_type = fission
        .answer
        .first()
        .map(|s| rust_type_hint(s))
        .unwrap_or_else(|| "()".into());

    let mut code = String::new();
    code.push_str(&format!("/// {}\n", fission.problem));
    code.push_str(&format!(
        "pub fn {}({}) -> {} {{\n",
        fn_name,
        params.join(", "),
        return_type
    ));
    code.push_str("    // TODO: implement\n");
    if return_type != "()" {
        code.push_str("    todo!()\n");
    }
    code.push_str("}\n");
    code
}

/// Generate a TypeScript function skeleton from a fission triple.
pub fn generate_ts_function(node_id: &str, fission: &TriadFission) -> String {
    let fn_name = node_id
        .split('.')
        .last()
        .unwrap_or(node_id);
    let params: Vec<String> = fission
        .demand
        .iter()
        .enumerate()
        .map(|(i, name)| format!("arg{}: {}", i, ts_type_hint(name)))
        .collect();
    let return_type = fission
        .answer
        .first()
        .map(|s| ts_type_hint(s))
        .unwrap_or_else(|| "void".into());

    let mut code = String::new();
    code.push_str(&format!("/**\n * {}\n */\n", fission.problem));
    code.push_str(&format!(
        "export function {}({}): {} {{\n",
        fn_name,
        params.join(", "),
        return_type
    ));
    code.push_str("  // TODO: implement\n");
    if return_type != "void" {
        code.push_str("  throw new Error('Not implemented');\n");
    }
    code.push_str("}\n");
    code
}

fn rust_type_hint(name: &str) -> String {
    match name.to_lowercase().as_str() {
        "string" | "str" => "String".into(),
        "int" | "i32" => "i32".into(),
        "i64" => "i64".into(),
        "u32" => "u32".into(),
        "u64" => "u64".into(),
        "f32" => "f32".into(),
        "f64" => "f64".into(),
        "bool" | "boolean" => "bool".into(),
        "void" | "()" => "()".into(),
        other => pascal_case(other),
    }
}

fn ts_type_hint(name: &str) -> String {
    match name.to_lowercase().as_str() {
        "string" | "str" => "string".into(),
        "int" | "i32" | "i64" | "u32" | "u64" | "f32" | "f64" | "number" => "number".into(),
        "bool" | "boolean" => "boolean".into(),
        "void" | "()" => "void".into(),
        other => pascal_case(other),
    }
}

fn pascal_case(s: &str) -> String {
    s.split(['_', '-', ' '])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().to_string() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join("")
}
