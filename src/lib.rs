use std::error::Error;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::process::Command;

use syn::visit_mut::VisitMut;
use syn::{Block, Expr, Type};

mod alloc;

/// TODO
/// * string literals
/// * Slices should be translated into raw pointers?
/// * Slice indexing does not work
/// * Alloc interface
pub fn transform(input_file_name: &str) -> Result<impl AsRef<Path>, Box<dyn Error>> {
    let source = {
        let mut s = String::new();
        let mut file = File::open(input_file_name)?;
        file.read_to_string(&mut s)?;
        s
    };

    log::trace!("original source:\n{}", source);
    let code = transform_source(&source)?;

    let mut prefix = Path::new(input_file_name)
        .file_stem()
        .ok_or_else(|| {
            StringError(format!(
                "Unable to create temp file from \"{}\"",
                input_file_name
            ))
        })?
        .to_owned();
    prefix.push(OsStr::new("-"));
    let mut out_file = tempfile::Builder::new()
        .prefix(&prefix)
        .suffix(".rs")
        .tempfile()?;

    out_file.write_all(code.as_bytes())?;
    let new_path = out_file.into_temp_path();

    // Best-effort only
    if let Err(err) = Command::new("rustfmt")
        .arg(&new_path)
        .spawn()
        .and_then(|mut child| child.wait())
    {
        log::debug!("Error formatting: {:?}", err);
    }

    if log::log_enabled!(log::Level::Trace) {
        log::trace!(
            "formatted output:\n{}",
            std::fs::read_to_string(&new_path).unwrap()
        );
    }

    Ok(new_path)
}

#[derive(Debug)]
struct StringError(String);

impl std::fmt::Display for StringError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(&format!("Error: {}", self.0), f)
    }
}

impl Error for StringError {}

fn transform_source(source: &str) -> Result<String, Box<dyn Error>> {
    let mut ast = syn::parse_file(&source)?;

    log::trace!("{:?}", ast);
    let mut replacer = ReplacerVisitor;
    replacer.visit_file_mut(&mut ast);

    let std_ast = syn::parse_file(include_str!("../unsafe_std.rs"))?;

    let code = quote::quote! {
        #![allow(unused_mut)]
        #![allow(unused_unsafe)]

        #ast

        #std_ast
    };

    log::debug!("after replacer: {}", code);

    Ok(format!("{}", code))
}

struct ReplacerVisitor;

impl ReplacerVisitor {
    fn cast_reference(&self, ref_expr: &syn::ExprReference) -> Expr {
        log::trace!("cast_reference");
        let inner = &ref_expr.expr;

        let mutability = ref_expr.mutability;
        let pointer_type: syn::Type = match mutability {
            Some(_) => syn::parse_quote![*mut _],
            None => syn::parse_quote![*const _],
        };
        let wrapper: syn::Ident = if mutability.is_some() {
            syn::parse_quote![__amargo_ref_mut]
        } else {
            syn::parse_quote![__amargo_ref]
        };

        syn::parse_quote![
            #wrapper(#inner) as #pointer_type
        ]
    }

    fn fn_ident<'ex, 'ident>(
        call_expr: &'ex syn::ExprCall,
        ident: &'ident str,
    ) -> Option<&'ex Expr> {
        match &*call_expr.func {
            Expr::Path(path_expr) => {
                if path_expr.path.is_ident(ident) {
                    Some(
                        call_expr
                            .args
                            .first()
                            .expect("Unexpected empty field deref"),
                    )
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn derive_attr(attributes: &mut [syn::Attribute]) -> Option<&mut syn::Attribute> {
        attributes
            .iter_mut()
            .find(|attr| attr.path.is_ident("derive"))
    }

    fn make_copy(attributes: &mut Vec<syn::Attribute>) {
        let derive_attr = match ReplacerVisitor::derive_attr(attributes.as_mut_slice()) {
            Some(attr) => attr,
            _ => {
                attributes.push(syn::parse_quote![#[derive(Clone, Copy)]]);
                return;
            }
        };

        let mut meta_list = match derive_attr.parse_meta() {
            Ok(syn::Meta::List(meta_list)) => meta_list,
            _ => return,
        };

        let mut has_clone = false;
        let mut has_copy = false;

        for nested in meta_list.nested.iter() {
            match nested {
                syn::NestedMeta::Meta(syn::Meta::Path(meta_path)) => {
                    if meta_path.is_ident("Clone") {
                        has_clone = true;
                    } else if meta_path.is_ident("Copy") {
                        has_copy = true;
                    }
                }
                _ => {}
            }
        }

        if !has_clone {
            let meta: syn::Meta = syn::parse_quote![Clone];
            meta_list.nested.push(syn::NestedMeta::Meta(meta));
        }

        if !has_copy {
            let meta: syn::Meta = syn::parse_quote![Copy];
            meta_list.nested.push(syn::NestedMeta::Meta(meta));
        }

        *derive_attr = syn::parse_quote![ #[ #meta_list ] ];
    }
}

impl VisitMut for ReplacerVisitor {
    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        match expr {
            Expr::Reference(reference) => {
                let casted = self.cast_reference(reference);
                *expr = casted;
            }
            Expr::Call(call) => {
                let mut new = None;

                if let Some(reference) = ReplacerVisitor::fn_ident(call, "__amargo_ref") {
                    // let decomposed = self.decompose(field_ref);
                    new = Some(syn::parse_quote![&#reference]);
                }

                if let Some(reference) = ReplacerVisitor::fn_ident(call, "__amargo_ref_mut") {
                    // let decomposed = self.decompose(field_ref);
                    new = Some(syn::parse_quote![&mut #reference]);
                }

                if let Some(new) = new {
                    *expr = new;
                }
            }
            _ => {}
        };
        syn::visit_mut::visit_expr_mut(self, expr);
    }

    fn visit_type_mut(&mut self, ty: &mut Type) {
        if let Type::Reference(reference_ty) = ty {
            let inner = reference_ty.elem.clone();
            let quoted: Type = if reference_ty.mutability.is_some() {
                syn::parse_quote! [
                    *mut #inner
                ]
            } else {
                syn::parse_quote! [
                    *const #inner
                ]
            };
            *ty = quoted;
        }

        syn::visit_mut::visit_type_mut(self, ty);
    }

    fn visit_item_fn_mut(&mut self, fun: &mut syn::ItemFn) {
        let inner = &fun.block;
        let block: Block = syn::parse_quote! [
            { unsafe #inner }
        ];
        *fun.block = block;

        syn::visit_mut::visit_item_fn_mut(self, fun);
    }

    fn visit_impl_item_method_mut(&mut self, method: &mut syn::ImplItemMethod) {
        let inner = &method.block;
        let block: Block = syn::parse_quote! [
            { unsafe #inner }
        ];
        method.block = block;

        syn::visit_mut::visit_impl_item_method_mut(self, method);
    }

    fn visit_trait_item_method_mut(&mut self, method: &mut syn::TraitItemMethod) {
        let default: Option<Block> = method.default.as_ref().map(|inner| {
            syn::parse_quote! [
                { unsafe #inner }
            ]
        });
        method.default = default;

        syn::visit_mut::visit_trait_item_method_mut(self, method);
    }

    fn visit_item_struct_mut(&mut self, struct_item: &mut syn::ItemStruct) {
        ReplacerVisitor::make_copy(&mut struct_item.attrs);
        syn::visit_mut::visit_item_struct_mut(self, struct_item);
    }
}
