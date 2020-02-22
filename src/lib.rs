use std::error::Error;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::process::Command;

use syn::visit_mut::VisitMut;
use syn::{Block, Expr, Type};

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

    let mut marker = MarkerVisitor;
    marker.visit_file_mut(&mut ast);
    log::debug!("{}", quote::quote! {#ast});

    let mut replacer = ReplacerVisitor;
    replacer.visit_file_mut(&mut ast);

    let code = quote::quote! {
        #![allow(unused_mut)]
        #![allow(unused_unsafe)]

        mod __amargo {
            #[allow(dead_code)]
            pub fn alloc<T>(n: usize) -> *mut T {
                let layout = std::alloc::Layout::from_size_align(
                    n * std::mem::size_of::<T>(),
                    std::mem::align_of::<T>(),
                )
                .expect("Error calling alloc");
                (unsafe { std::alloc::alloc(layout) }) as *mut T
            }

            #[allow(dead_code)]
            pub fn dealloc<T>(n: usize, ptr: *mut T) {
                let layout = std::alloc::Layout::from_size_align(
                    n * std::mem::size_of::<T>(),
                    std::mem::align_of::<T>(),
                )
                .expect("Error calling dealloc");
                unsafe { std::alloc::dealloc(ptr as *mut u8, layout) }
            }
        }

        #ast
    };

    log::debug!("{}", code);

    Ok(format!("{}", code))
}

struct MarkerVisitor;

impl VisitMut for MarkerVisitor {
    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        match expr {
            Expr::Reference(reference) => {
                let inner = &reference.expr;
                let quoted: Expr = syn::parse_quote! {
                    __make_unsafe![#inner]
                };
                *expr = quoted;
            }
            Expr::Unary(syn::ExprUnary {
                op: syn::UnOp::Deref(..),
                expr: inner,
                ..
            }) => {
                let quoted: Expr = syn::parse_quote! {
                    __make_unsafe_deref![#inner]
                };
                *expr = quoted;
            }
            Expr::Assign(syn::ExprAssign { left, right, .. }) => {
                if let Expr::Unary(syn::ExprUnary {
                    op: syn::UnOp::Deref(..),
                    expr: ref inner,
                    ..
                }) = **left
                {
                    let quoted: Expr = syn::parse_quote! {
                        __make_unsafe_assign_deref![{#inner} {#right}]
                    };
                    *expr = quoted;
                }
            }
            _ => {}
        }

        syn::visit_mut::visit_expr_mut(self, expr);
    }
}

struct ReplacerVisitor;

impl VisitMut for ReplacerVisitor {
    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        if let Expr::Macro(exprmacro) = expr {
            if exprmacro.mac.path.is_ident("__make_unsafe") {
                let inner_expr: Expr =
                    syn::parse2(exprmacro.mac.tokens.clone()).expect("Unexpected parsing error");
                let new_expr: Expr = syn::parse_quote! {
                    &#inner_expr as *const _ as *mut _
                };
                *expr = new_expr;
            } else if exprmacro.mac.path.is_ident("__make_unsafe_deref") {
                let inner_expr: Expr =
                    syn::parse2(exprmacro.mac.tokens.clone()).expect("Unexpected parsing error");
                let new_expr: Expr = syn::parse_quote! {
                    unsafe { *#inner_expr }
                };
                *expr = new_expr;
            } else if exprmacro.mac.path.is_ident("__make_unsafe_assign_deref") {
                let mut tokens = exprmacro.mac.tokens.clone().into_iter();
                let left = expr_from_tree(tokens.next().expect("Unexpected empty tokens"));
                let right = expr_from_tree(tokens.next().expect("Unexpected empty tokens"));
                let new_expr: Expr = syn::parse_quote! {
                    unsafe { *#left = #right }
                };
                *expr = new_expr;
            }
        }
        syn::visit_mut::visit_expr_mut(self, expr);
    }

    fn visit_type_mut(&mut self, ty: &mut Type) {
        if let Type::Reference(reference) = ty {
            let inner = reference.elem.clone();
            let quoted: Type = syn::parse_quote! {
                *mut #inner
            };
            *ty = quoted;
        }

        syn::visit_mut::visit_type_mut(self, ty);
    }

    fn visit_item_fn_mut(&mut self, fun: &mut syn::ItemFn) {
        let inner = &fun.block;
        let block: Block = syn::parse_quote! {
            { unsafe #inner }
        };
        *fun.block = block;

        syn::visit_mut::visit_item_fn_mut(self, fun);
    }

    fn visit_impl_item_method_mut(&mut self, method: &mut syn::ImplItemMethod) {
        let inner = &method.block;
        let block: Block = syn::parse_quote! {
            { unsafe #inner }
        };
        method.block = block;

        syn::visit_mut::visit_impl_item_method_mut(self, method);
    }

    fn visit_trait_item_method_mut(&mut self, method: &mut syn::TraitItemMethod) {
        let default: Option<Block> = method.default.as_ref().map(|inner| {
            syn::parse_quote! {
                { unsafe #inner }
            }
        });
        method.default = default;

        syn::visit_mut::visit_trait_item_method_mut(self, method);
    }

    fn visit_expr_call_mut(&mut self, expr: &mut syn::ExprCall) {
        match &mut *expr.func {
            syn::Expr::Path(path_expr) => {
                if let Some(ident) = path_expr.path.get_ident() {
                    if ident == "alloc" {
                        path_expr.path = syn::parse_quote! {
                            __amargo::alloc
                        };
                    } else if ident == "dealloc" {
                        path_expr.path = syn::parse_quote! {
                            __amargo::dealloc
                        };
                    }
                }
            }
            _ => {}
        }
    }
}

fn expr_from_tree(tree: proc_macro2::TokenTree) -> proc_macro2::TokenStream {
    match tree {
        proc_macro2::TokenTree::Group(group) => group.stream(),
        _ => unreachable!(),
    }
}
