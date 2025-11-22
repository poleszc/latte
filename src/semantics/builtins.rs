use crate::parser::ast::Ident;
use crate::parser::ast::{SourcePosition, Type, TypeKind};
use crate::semantics::symbols::FunctionInfo;

type BuiltinDef = (&'static str, TypeKind, Vec<TypeKind>);
fn get_raw_builtins() -> Vec<BuiltinDef> {
    vec![
        ("printInt", TypeKind::Void, vec![TypeKind::Int]),
        ("printString", TypeKind::Void, vec![TypeKind::String]),
        ("error", TypeKind::Void, vec![]),
        ("readInt", TypeKind::Int, vec![]),
        ("readString", TypeKind::String, vec![]),
    ]
}

pub fn get_builtin_functions() -> Vec<FunctionInfo> {
    get_raw_builtins()
        .into_iter()
        .map(|(name, ret_kind, param_kinds)| FunctionInfo {
            ident: Ident {
                name: name.to_string(),
                position: SourcePosition(0, 0),
            },
            return_type: Type {
                kind: ret_kind,
                position: SourcePosition(0, 0),
            },
            params: param_kinds
                .into_iter()
                .map(|k| Type {
                    kind: k,
                    position: SourcePosition(0, 0),
                })
                .collect(),
            position: SourcePosition(0, 0),
        })
        .collect()
}

pub fn is_builtin(name: &str) -> bool {
    matches!(
        name,
        "printInt" | "printString" | "error" | "readInt" | "readString"
    )
}

pub fn get_builtin_signature(name: &str) -> Option<String> {
    let builtins = get_raw_builtins();

    if let Some((func_name, ret_kind, param_kinds)) =
        builtins.into_iter().find(|(n, _, _)| *n == name)
    {
        let params: Vec<String> = param_kinds.iter().map(|k| k.to_string()).collect();
        // TODO: Implement display for Type?
        Some(format!(
            "{} {}({})",
            ret_kind.to_string(),
            func_name,
            params.join(", ")
        ))
    } else {
        None
    }
}
