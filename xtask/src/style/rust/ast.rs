//! Rules that inspect the parsed Rust syntax tree.

use std::collections::HashSet;
use std::path::Path;

use proc_macro2::Span;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{
    File, ImplItem, Item, ItemEnum, ItemImpl, ItemMod, ItemTrait, ReturnType, TraitItem, Type,
    UseTree,
};

use crate::style::{CrateKind, StyleViolation};

const FUNCTION_SIZE_LIMIT: usize = 60;
const INLINE_TESTS_LIMIT: usize = 100;

pub(super) fn inspect_items(
    parsed: &File,
    path: &Path,
    crate_kind: CrateKind,
    file_line_count: usize,
    violations: &mut Vec<StyleViolation>,
) {
    inspect_item_list(&parsed.items, path, crate_kind, file_line_count, violations);
}

fn inspect_item_list(
    items: &[Item],
    path: &Path,
    crate_kind: CrateKind,
    file_line_count: usize,
    violations: &mut Vec<StyleViolation>,
) {
    let imported_anyhow = collect_anyhow_imports(items);

    for item in items {
        match item {
            Item::Use(item_use) if item_use.tree.contains_glob() => {
                violations.push(StyleViolation {
                    rule_id: "STYLE-IMP-001",
                    path: path.to_path_buf(),
                    line: Some(item_use.span().start().line),
                    message: "glob imports are forbidden".to_owned(),
                })
            }
            Item::Fn(item_fn) => {
                check_function_length(
                    item_fn.sig.ident.to_string(),
                    item_fn.span(),
                    path,
                    violations,
                );
                if crate_kind == CrateKind::Library {
                    check_public_anyhow_return(
                        &item_fn.vis,
                        &item_fn.sig.output,
                        &imported_anyhow,
                        path,
                        item_fn.sig.ident.span().start().line,
                        item_fn.sig.ident.to_string(),
                        violations,
                    );
                }
            }
            Item::Impl(item_impl) => {
                inspect_impl(item_impl, path, crate_kind, &imported_anyhow, violations)
            }
            Item::Trait(item_trait) => {
                inspect_trait(item_trait, path, crate_kind, &imported_anyhow, violations)
            }
            Item::Enum(item_enum) if crate_kind == CrateKind::Library => {
                check_error_enum(item_enum, &imported_anyhow, path, violations)
            }
            Item::Mod(item_mod) => {
                inspect_mod(item_mod, path, crate_kind, file_line_count, violations)
            }
            _ => {}
        }
    }
}

fn inspect_impl(
    item_impl: &ItemImpl,
    path: &Path,
    crate_kind: CrateKind,
    imported_anyhow: &HashSet<String>,
    violations: &mut Vec<StyleViolation>,
) {
    for item in &item_impl.items {
        if let ImplItem::Fn(item_fn) = item {
            check_function_length(
                item_fn.sig.ident.to_string(),
                item_fn.span(),
                path,
                violations,
            );
            if crate_kind == CrateKind::Library {
                check_public_anyhow_return(
                    &item_fn.vis,
                    &item_fn.sig.output,
                    imported_anyhow,
                    path,
                    item_fn.sig.ident.span().start().line,
                    item_fn.sig.ident.to_string(),
                    violations,
                );
            }
        }
    }
}

fn inspect_trait(
    item_trait: &ItemTrait,
    path: &Path,
    crate_kind: CrateKind,
    imported_anyhow: &HashSet<String>,
    violations: &mut Vec<StyleViolation>,
) {
    for item in &item_trait.items {
        if let TraitItem::Fn(item_fn) = item {
            if let Some(default) = &item_fn.default {
                check_function_length(
                    item_fn.sig.ident.to_string(),
                    default.span(),
                    path,
                    violations,
                );
            }
            if crate_kind == CrateKind::Library
                && matches!(item_trait.vis, syn::Visibility::Public(_))
            {
                check_anyhow_return_type(
                    &item_fn.sig.output,
                    imported_anyhow,
                    path,
                    item_fn.sig.ident.span().start().line,
                    item_fn.sig.ident.to_string(),
                    violations,
                );
            }
        }
    }
}

fn inspect_mod(
    item_mod: &ItemMod,
    path: &Path,
    crate_kind: CrateKind,
    file_line_count: usize,
    violations: &mut Vec<StyleViolation>,
) {
    if file_line_count > INLINE_TESTS_LIMIT
        && item_mod.ident == "tests"
        && item_mod.content.is_some()
        && has_cfg_test(&item_mod.attrs)
    {
        violations.push(StyleViolation {
            rule_id: "STYLE-TEST-001",
            path: path.to_path_buf(),
            line: Some(item_mod.ident.span().start().line),
            message: "inline #[cfg(test)] mod tests blocks are forbidden above 100 lines"
                .to_owned(),
        });
    }
    if let Some((_, items)) = &item_mod.content {
        inspect_item_list(items, path, crate_kind, file_line_count, violations);
    }
}

fn check_function_length(
    name: String,
    span: Span,
    path: &Path,
    violations: &mut Vec<StyleViolation>,
) {
    let start = span.start().line;
    let end = span.end().line;
    let line_count = end.saturating_sub(start) + 1;
    if line_count > FUNCTION_SIZE_LIMIT {
        violations.push(StyleViolation {
            rule_id: "STYLE-FUNC-001",
            path: path.to_path_buf(),
            line: Some(start),
            message: format!("function `{name}` has {line_count} lines"),
        });
    }
}

fn check_public_anyhow_return(
    visibility: &syn::Visibility,
    output: &ReturnType,
    imported_anyhow: &HashSet<String>,
    path: &Path,
    line: usize,
    name: String,
    violations: &mut Vec<StyleViolation>,
) {
    if matches!(visibility, syn::Visibility::Public(_)) {
        check_anyhow_return_type(output, imported_anyhow, path, line, name, violations);
    }
}

fn check_anyhow_return_type(
    output: &ReturnType,
    imported_anyhow: &HashSet<String>,
    path: &Path,
    line: usize,
    name: String,
    violations: &mut Vec<StyleViolation>,
) {
    let ReturnType::Type(_, ty) = output else {
        return;
    };
    if type_mentions_anyhow(ty, imported_anyhow) {
        violations.push(StyleViolation {
            rule_id: "STYLE-ERR-001",
            path: path.to_path_buf(),
            line: Some(line),
            message: format!("public API `{name}` exposes an anyhow-typed return"),
        });
    }
}

fn check_error_enum(
    item_enum: &ItemEnum,
    imported_anyhow: &HashSet<String>,
    path: &Path,
    violations: &mut Vec<StyleViolation>,
) {
    if !matches!(item_enum.vis, syn::Visibility::Public(_))
        || !item_enum.ident.to_string().ends_with("Error")
    {
        return;
    }
    for variant in &item_enum.variants {
        if variant.ident == "Anyhow" || variant.ident.to_string().contains("Anyhow") {
            violations.push(StyleViolation {
                rule_id: "STYLE-ERR-002",
                path: path.to_path_buf(),
                line: Some(variant.ident.span().start().line),
                message: format!(
                    "public error enum `{}` contains a catch-all `{}` variant",
                    item_enum.ident, variant.ident
                ),
            });
            continue;
        }
        if variant
            .fields
            .iter()
            .any(|field| type_mentions_anyhow(&field.ty, imported_anyhow))
        {
            violations.push(StyleViolation {
                rule_id: "STYLE-ERR-002",
                path: path.to_path_buf(),
                line: Some(variant.ident.span().start().line),
                message: format!(
                    "public error enum `{}` contains an anyhow-typed `{}` variant",
                    item_enum.ident, variant.ident
                ),
            });
        }
    }
}

fn collect_anyhow_imports(items: &[Item]) -> HashSet<String> {
    let mut imported = HashSet::new();
    for item in items {
        if let Item::Use(item_use) = item {
            collect_anyhow_from_use_tree(None, &item_use.tree, &mut imported);
        }
    }
    imported
}

fn collect_anyhow_from_use_tree(
    prefix: Option<String>,
    tree: &UseTree,
    imported: &mut HashSet<String>,
) {
    match tree {
        UseTree::Path(use_path) => {
            let current = match prefix {
                Some(prefix) => format!("{prefix}::{}", use_path.ident),
                None => use_path.ident.to_string(),
            };
            collect_anyhow_from_use_tree(Some(current), &use_path.tree, imported);
        }
        UseTree::Name(name) => {
            if prefix.as_deref() == Some("anyhow") {
                imported.insert(name.ident.to_string());
            }
        }
        UseTree::Rename(rename) => {
            if prefix.as_deref() == Some("anyhow") {
                imported.insert(rename.rename.to_string());
            }
        }
        UseTree::Group(group) => {
            for item in &group.items {
                collect_anyhow_from_use_tree(prefix.clone(), item, imported);
            }
        }
        UseTree::Glob(_) => {
            if prefix.as_deref() == Some("anyhow") {
                imported.insert("*".to_owned());
            }
        }
    }
}

fn has_cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("cfg")
            && attr
                .parse_args::<syn::Path>()
                .is_ok_and(|path| path.is_ident("test"))
    })
}

fn type_mentions_anyhow(ty: &Type, imported_anyhow: &HashSet<String>) -> bool {
    let mut visitor = AnyhowTypeVisitor::new(imported_anyhow);
    visitor.visit_type(ty);
    visitor.found
}

struct AnyhowTypeVisitor<'a> {
    imported_anyhow: &'a HashSet<String>,
    found: bool,
}

impl<'a> AnyhowTypeVisitor<'a> {
    fn new(imported_anyhow: &'a HashSet<String>) -> Self {
        Self {
            imported_anyhow,
            found: false,
        }
    }
}

impl<'ast, 'a> Visit<'ast> for AnyhowTypeVisitor<'a> {
    fn visit_type_path(&mut self, node: &'ast syn::TypePath) {
        if self.found {
            return;
        }
        let segments = node
            .path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string());
        if segments
            .clone()
            .any(|segment| segment == "anyhow" || self.imported_anyhow.contains(&segment))
        {
            self.found = true;
            return;
        }
        visit::visit_type_path(self, node);
    }
}

trait UseTreeExt {
    fn contains_glob(&self) -> bool;
}

impl UseTreeExt for UseTree {
    fn contains_glob(&self) -> bool {
        match self {
            UseTree::Glob(_) => true,
            UseTree::Group(group) => group.items.iter().any(UseTreeExt::contains_glob),
            UseTree::Path(path) => path.tree.contains_glob(),
            UseTree::Name(_) | UseTree::Rename(_) => false,
        }
    }
}
