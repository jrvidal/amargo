use std::error::Error;
use std::fs::File;
use std::io::{Read, Write};
use std::process::Command;

use syn::visit_mut::VisitMut;
use syn::{Block, Expr, Type};

/// TODO
/// * string literals
/// * Slices should be translated into raw pointers?
/// * Slice indexing does not work
/// * Alloc interface
fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    if std::env::var_os("AMARGO_RUSTC").is_some() {
        main_rustc()
    } else {
        let exit = Command::new("cargo")
            // We don't respect existing RUSTC_WRAPPER
            .env("RUSTC_WRAPPER", "amargo")
            .env("AMARGO_RUSTC", "on")
            .args(std::env::args().skip(1))
            .spawn()?
            .wait()?;
        if !exit.success() {
            std::process::exit(exit.code().unwrap_or(1));
        }
        Ok(())
    }
}

fn main_rustc() -> Result<(), Box<dyn Error>> {
    let mut args: Vec<_> = std::env::args().skip(2).collect();

    let _any = transform_args(&mut args)?;

    let exit = Command::new("rustc").args(&args).spawn()?.wait()?;

    if exit.success() {
        Ok(())
    } else {
        Err(StringError("rustc invocation failed".into()))?
    }
}

fn transform_args(args: &mut [String]) -> Result<impl std::any::Any, Box<dyn Error>> {
    let (index, input_file_name) = match find_input_file(args) {
        None => return Ok(None),
        Some(res) => res,
    };

    let source = {
        let mut s = String::new();
        let mut file = File::open(input_file_name)?;
        file.read_to_string(&mut s)?;
        s
    };

    let code = transform_source(&source)?;

    let mut out_file = tempfile::NamedTempFile::new()?;

    out_file.write(format!("{}", code).as_bytes())?;
    let new_path = out_file.into_temp_path();

    // Best-effort only
    let _ = Command::new("rustfmt").arg(&new_path).spawn()?.wait()?;

    args[index] = new_path.to_string_lossy().to_string();

    Ok(Some(new_path))
}

fn find_input_file(args: &[String]) -> Option<(usize, &String)> {
    let mut skip = false;

    for (i, arg) in args.iter().enumerate() {
        if arg == "-" {
            return None;
        } else if arg.starts_with("-") {
            skip = true;
            continue;
        } else if skip {
            skip = false;
            continue;
        }
        if arg.ends_with(".rs") {
            return Some((i, arg));
        }
    }

    return None;
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
        let default: Option<Block> = method.default.as_mut().map(|inner| {
            syn::parse_quote! {
                { unsafe #inner }
            }
        });
        method.default = default;

        syn::visit_mut::visit_trait_item_method_mut(self, method);
    }
}

fn expr_from_tree(tree: proc_macro2::TokenTree) -> proc_macro2::TokenStream {
    match tree {
        proc_macro2::TokenTree::Group(group) => group.stream(),
        _ => unreachable!(),
    }
}
