#![feature(drain_filter)]
extern crate proc_macro;
use std::string::String;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, ToTokens};
use regex::Regex;
use syn::{
  parse::{Parse, ParseStream},
  parse_macro_input, parse_quote,
  punctuated::{Pair, Punctuated},
  visit::{self, Visit},
  visit_mut::{self, VisitMut},
  Attribute, Block, Expr, ExprClosure, ExprLit, ExprPath, GenericArgument, Ident, ImplItemMethod, ItemFn,
  ItemImpl, ItemMacro, Lit, Local, Macro, Pat, Path, PathArguments, PredicateType, Result, Stmt, Token,
  TraitBound, TraitBoundModifier, Type, TypeParamBound, WhereClause, WherePredicate,
};

pub fn transform(metadata: TokenStream, input: TokenStream) -> TokenStream {
  let mut ast = parse_macro_input!(input as ItemImpl);
  let mut w = Syntax::default();
  w.visit_item_impl_mut(&mut ast);
  w.visit_item_impl_mut(&mut ast);
  TokenStream::from(quote!(#ast))
}

struct Syntax {
  aliases: Vec<(Ident, Type)>,
  bounds:  Vec<(PredicateType, Type, Path)>,
  qselfs:  Vec<(Ident, Type, Path)>,
}

impl Default for Syntax {
  fn default() -> Self { Syntax { aliases: vec![], bounds: vec![], qselfs: vec![] } }
}

impl VisitMut for Syntax {
  fn visit_expr_path_mut(&mut self, node: &mut ExprPath) {
    if node.path.segments.len() >= 2 {
      for it in &self.aliases {
        if it.0 == node.path.segments[0].ident {
          let mut a = node.path.segments.clone();
          let mut b = node.path.segments.clone();
          b.clear();
          for i in 1..a.len() {
            b.push(a[i].clone());
          }
          let ty = &it.1;
          *node = parse_quote!(#ty::#b);
        }
      }
    }

    if node.path.segments.len() >= 2 {
      for qs in &self.qselfs {
        if qs.0 == node.path.segments[0].ident {
          let mut a = node.path.segments.clone();
          let mut b = node.path.segments.clone();
          b.clear();
          for i in 1..a.len() {
            b.push(a[i].clone());
          }
          let ty = &qs.1;
          let pa = &qs.2;
          *node = parse_quote!(<#ty as #pa>::#b);
        }
      }
    }

    visit_mut::visit_expr_path_mut(self, node);
  }

  fn visit_type_mut(&mut self, node: &mut Type) {
    match node {
      | Type::Path(tp) => {
        if tp.path.segments.len() == 1 {
          for it in &self.aliases {
            if it.0 == tp.path.segments[0].ident {
              *node = it.1.clone();
              break;
            }
          }
        }
      }
      | _ => (),
    }
    match node {
      | Type::Path(tp) => {
        if tp.path.segments.len() >= 2 {
          for qs in &self.qselfs {
            if qs.0 == tp.path.segments[0].ident {
              let mut a = tp.path.segments.clone();
              let mut b = tp.path.segments.clone();
              b.clear();
              for i in 1..a.len() {
                b.push(a[i].clone());
              }
              let ty = &qs.1;
              let pa = &qs.2;
              *tp = parse_quote!(<#ty as #pa>::#b);
            }
          }
        }
      }
      | _ => (),
    }
    visit_mut::visit_type_mut(self, node);
  }

  fn visit_where_clause_mut(&mut self, node: &mut WhereClause) {
    visit_mut::visit_where_clause_mut(self, node);
    for btp in &self.bounds {
      let mut pt = btp.0.clone();
      pt.bounded_ty = btp.1.clone();
      pt.bounds.clear();
      pt.bounds.push(TypeParamBound::Trait(TraitBound {
        paren_token: None,
        modifier:    TraitBoundModifier::None,
        lifetimes:   None,
        path:        btp.2.clone(),
      }));
      //println!("KUKU {:#?}", btp.2);
      node.predicates.push(WherePredicate::Type(pt));
    }
  }

  fn visit_where_predicate_mut(&mut self, node: &mut WherePredicate) {
    let mut norec = false;
    match node {
      | WherePredicate::Type(pt) => {
        if pt.bounds.len() >= 1 {
          match &mut pt.bounds[0] {
            | TypeParamBound::Trait(tb) => {
              if tb.path.segments.len() >= 1 {
                if tb.path.segments[0].ident == "MakeAlias" {
                  match &mut tb.path.segments[0].arguments {
                    | PathArguments::AngleBracketed(abga) => {
                      if abga.args.len() == 1 {
                        match &mut abga.args[0] {
                          | GenericArgument::Type(Type::Path(tp)) => {
                            if tp.path.segments.len() == 1 {
                              let ident = tp.path.segments[0].ident.clone();
                              pt.bounds[0] = pt.bounds[pt.bounds.len() - 1].clone();
                              pt.bounds.pop();
                              self.visit_type_mut(&mut pt.bounded_ty);
                              self.aliases.push((ident, pt.bounded_ty.clone()));
                              for bn in &mut pt.bounds {
                                self.visit_type_param_bound_mut(bn);
                              }
                              match &pt.bounded_ty {
                                | Type::Path(tp) => {
                                  if let Some(qs) = &tp.qself {
                                    let mut tpath = tp.path.clone();
                                    tpath.segments.pop();
                                    if let Some(Pair::Punctuated(t, _)) = tpath.segments.pop() {
                                      tpath.segments.push_value(t);
                                    }
                                    self.bounds.push((pt.clone(), *qs.ty.clone(), tpath));
                                  }
                                }
                                | _ => (),
                              }
                              norec = true;
                            }
                          }
                          | _ => (),
                        }
                      }
                    }
                    | _ => (),
                  }
                }
              }
            }
            | _ => (),
          }
        }
        if pt.bounds.len() == 2 {
          match (pt.bounds[0].clone(), &mut pt.bounds[1]) {
            | (TypeParamBound::Trait(mut ta), TypeParamBound::Trait(tb)) => {
              if tb.path.segments.len() >= 1 {
                if tb.path.segments[0].ident == "MakeQSelf" {
                  match &mut tb.path.segments[0].arguments {
                    | PathArguments::AngleBracketed(abga) => {
                      if abga.args.len() == 1 {
                        match &mut abga.args[0] {
                          | GenericArgument::Type(Type::Path(tp)) => {
                            if tp.path.segments.len() == 1 {
                              let ident = tp.path.segments[0].ident.clone();
                              pt.bounds.pop();
                              self.visit_type_mut(&mut pt.bounded_ty);
                              self.visit_type_param_bound_mut(&mut pt.bounds[0]);
                              for sg in &mut ta.path.segments {
                                match &mut sg.arguments {
                                  | PathArguments::AngleBracketed(abga) => {
                                    let nargs = abga.args.clone();
                                    abga.args.clear();
                                    for ar in nargs {
                                      match ar {
                                        | GenericArgument::Binding(_) => (),
                                        | _ => {
                                          abga.args.push(ar);
                                        }
                                      }
                                    }
                                  }
                                  | _ => (),
                                }
                              }
                              self.qselfs.push((ident, pt.bounded_ty.clone(), ta.path.clone()));
                              match &pt.bounded_ty {
                                | Type::Path(tp) => {
                                  if let Some(qs) = &tp.qself {
                                    let mut tpath = tp.path.clone();
                                    tpath.segments.pop();
                                    if let Some(Pair::Punctuated(t, _)) = tpath.segments.pop() {
                                      tpath.segments.push_value(t);
                                    }
                                    self.bounds.push((pt.clone(), *qs.ty.clone(), tpath));
                                  }
                                }
                                | _ => (),
                              }
                              norec = true;
                            }
                          }
                          | _ => (),
                        }
                      }
                    }
                    | _ => (),
                  }
                }
              }
            }
            | _ => (),
          }
        }
      }
      | _ => (),
    }
    if !norec {
      visit_mut::visit_where_predicate_mut(self, node);
    }
  }
}
