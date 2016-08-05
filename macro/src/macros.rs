#![feature(quote, plugin)]
#![plugin(matches)]
extern crate syntex_syntax as syntax;
extern crate syntex;
#[no_link]
#[macro_use]
extern crate matches;

use std::default::Default;

use syntax::codemap::*;
use syntax::parse::*;
use syntax::tokenstream::TokenTree;
use syntax::parse::token::Token;
use syntax::abi::Abi;
use syntax::ast::*;
use syntax::ext::base::*;
use syntax::ext::build::*;
use syntax::ext::source_util::*;
use syntax::ptr::P;
use syntex::Registry;

static BAD_STRUCT:&'static str = "jit-compatible structs must be packed, mark with #[repr(packed)] to fix";
static BAD_ITEM:&'static str = "only structs can be compatible with LibJIT";

pub struct Spanner(pub Span);
impl Spanner {
    pub fn span<T>(&self, v: T) -> Spanned<T> {
        Spanned {
            node: v,
            span: self.0.clone()
        }
    }
}

fn simple_type(cx: &mut ExtCtxt, name: &'static str, as_cow:bool) -> P<Expr> {
    let new_name = format!("get_{}", name);
    let name = cx.ident_of(&new_name);
    let mut expr = quote_expr!(cx, jit::typecs::$name());
    if as_cow {
        expr = quote_expr!(cx, $expr.into())
    }
    expr
}
fn type_expr(cx: &mut ExtCtxt, sp: Span, ty: P<Ty>, as_cow: bool) -> P<Expr> {
    match ty.node {
        TyKind::Paren(ref ty) => type_expr(cx, sp, ty.clone(), as_cow),
        TyKind::Ptr(_) | TyKind::Rptr(_, _) => simple_type(cx, "VOID_PTR", as_cow),
        TyKind::Path(ref self_, ref path) => {
            if self_.is_none() && path.segments.len() == 1 {
                match &*path.segments[0].identifier.name.as_str() {
                    "i8" => return simple_type(cx, "sbyte", as_cow),
                    "u8" => return simple_type(cx, "ubyte", as_cow),
                    "i16" => return simple_type(cx, "short", as_cow),
                    "u16" => return simple_type(cx, "ushort", as_cow),
                    "i32" => return simple_type(cx, "int", as_cow),
                    "u32" => return simple_type(cx, "uint", as_cow),
                    "i64" => return simple_type(cx, "long", as_cow),
                    "u64" => return simple_type(cx, "ulong", as_cow),
                    "isize" => return simple_type(cx, "nint", as_cow),
                    "usize" => return simple_type(cx, "nuint", as_cow),
                    "f32" => return simple_type(cx, "float32", as_cow),
                    "f64" => return simple_type(cx, "float64", as_cow),
                    "bool" => return simple_type(cx, "sys_bool", as_cow),
                    "char" => return simple_type(cx, "sys_char", as_cow),
                    _ => {/* fall through */}
                }
            }
            if as_cow {
                quote_expr!(cx, ::jit::get::<$ty>())
            } else {
                quote_expr!(cx, &::jit::get::<$ty>())
            }
        },

        _ => {
            if as_cow {
                quote_expr!(cx, ::jit::get::<$ty>())
            } else {
                quote_expr!(cx, &::jit::get::<$ty>())
            }
        }
    }
}
fn expand_derive_compile(cx: &mut ExtCtxt, sp: Span, _meta: &MetaItem, item: &Annotatable, push: &mut FnMut(Annotatable)) {
    let s = Spanner(sp.clone());
    let item = item.clone().expect_item();
    let name = item.ident;
    let jit = cx.ident_of("jit");
    let jit_life = cx.lifetime(sp, token::intern("'a"));
    let jit_compile = cx.path_all(sp, false, vec![jit, cx.ident_of("Compile")], vec![jit_life], vec![], vec![]);
    let jit_cow_type = cx.path_all(sp, false, vec![jit, cx.ident_of("CowType")], vec![cx.lifetime(sp, token::intern("'static"))], vec![], vec![]);
    let jit_func = cx.path_all(sp, false, vec![jit, cx.ident_of("UncompiledFunction")], vec![jit_life], vec![], vec![]);
    let jit_val = cx.path(sp, vec![jit, cx.ident_of("Val")]);
    let jit_val_new = cx.path(sp, vec![jit, cx.ident_of("Val"), cx.ident_of("new")]);
    let jit_value = cx.ty_rptr(sp, cx.ty_path(jit_val), Some(jit_life), Mutability::Immutable);
    let new_struct = cx.path(sp, vec![jit, cx.ident_of("Type"), cx.ident_of("new_struct")]);
    let func = cx.ident_of("func");
    let value = cx.ident_of("value");
    let offset = cx.ident_of("offset");
    let mut repr = None;
    for attr in &item.attrs {
        if let MetaItemKind::List(ref name, ref items) = attr.node.value.node {
            if &**name == "repr" && items.len() == 1 {
                if let MetaItemKind::Word(ref text) = items[0].node {
                    repr = Some(&**text)
                }
            }
        }
    }
    match item.node {
        ItemKind::Enum(_, _) => {
            if let Some(kind) = repr {
                let inner_ty = cx.ty_ident(sp, cx.ident_of(kind));
                let type_expr = type_expr(cx, sp, inner_ty.clone(), true);
                let expr = quote_expr!(cx, (self as $inner_ty).compile(&func));
                let kind = s.span(SelfKind::Value(Mutability::Immutable));
                let item = cx.item(sp, name, vec![], ItemKind::Impl(
                    Unsafety::Normal,
                    ImplPolarity::Positive,
                    Generics {
                        lifetimes: vec![ LifetimeDef {lifetime: jit_life, bounds: vec![]}],
                        ty_params: Vec::new().into(),
                        where_clause: WhereClause {
                            id: DUMMY_NODE_ID,
                            predicates: vec![]
                        }
                    },
                    Some(cx.trait_ref(jit_compile)),
                    cx.ty_ident(sp, name),
                    vec![
                        ImplItem {
                            attrs: vec![],
                            id: DUMMY_NODE_ID,
                            span: sp,
                            defaultness: Defaultness::Final,
                            ident: cx.ident_of("get_type"),
                            vis: Visibility::Inherited,
                            node: ImplItemKind::Method(
                                MethodSig {
                                    constness: Constness::NotConst,
                                    unsafety: Unsafety::Normal,
                                    abi: Abi::Rust,
                                    //explicit_self: respan(sp, ExplicitSelfKind::SelfStatic),
                                    decl: cx.fn_decl(vec![], cx.ty_path(jit_cow_type)),
                                    generics: Generics::default(),
                                },
                                cx.block_expr(type_expr))
                        },
                        ImplItem {
                            attrs: vec![],
                            id: DUMMY_NODE_ID,
                            span: sp,
                            defaultness: Defaultness::Final,
                            ident: cx.ident_of("compile"),
                            vis: Visibility::Inherited,
                            node: ImplItemKind::Method(
                                MethodSig {
                                    constness: Constness::NotConst,
                                    unsafety: Unsafety::Normal,
                                    abi: Abi::Rust,
                                    /*explicit_self: respan(
                                        sp,
                                        ExplicitSelfKind::SelfValue(cx.ident_of("b"))),*/
                                    decl: cx.fn_decl(
                                        vec![
                                            Arg::from_self(kind, s.span(cx.ident_of("self"))),
                                            cx.arg(sp, func, cx.ty_rptr(sp, cx.ty_path(jit_func),
                                                                        None, Mutability::Immutable))],
                                        jit_value),
                                    generics: Generics::default(),
                                },
                                cx.block_expr(expr)
                            )
                        }
                    ]
                ));
                push(Annotatable::Item(item));
            } else {
                cx.span_err(sp, BAD_ITEM)
            }
        },
        ItemKind::Struct(ref def, ref gen) => {
            if !matches!(repr, Some("packed") | Some("c") | Some("C")) {
                cx.span_err(sp, BAD_STRUCT);
                return;
            }
            let def_fields = def.fields();
            let mut fields = Vec::with_capacity(def_fields.len());
            let mut names = Some(Vec::with_capacity(fields.len()));
            let mut compiler = Vec::with_capacity(def_fields.len() + 1);
            let types = gen.ty_params.iter().map(|param| cx.ty_ident(sp, param.ident)).collect();
            let self_ty = cx.ty_path(cx.path_all(sp, false, vec![name], vec![], types, vec![]));
            let self_type = type_expr(cx, sp, self_ty.clone(), false);
            compiler.push(cx.stmt_let(sp, false, value, cx.expr_call(
                sp,
                cx.expr_path(jit_val_new),
                vec![
                    cx.expr_ident(sp, func),
                    self_type
                ]
            )));
            let lit_usize = LitIntType::Unsigned(UintTy::Us);
            if def_fields.len() > 1 {
                compiler.push(cx.stmt_let(sp, true, offset, cx.expr_lit(sp, LitKind::Int(0, lit_usize))));
            }
            for (index, field) in def_fields.iter().enumerate() {
                let expr = type_expr(cx, sp, field.ty.clone(), false);
                fields.push(expr);
                let has_name = field.ident.is_some();
                if has_name && names.is_some() {
                    let ident = field.ident.unwrap();
                    let expr = expand_stringify(cx, sp, &[TokenTree::Token(sp, Token::Ident(ident))]);
                    names.as_mut().unwrap().push(expr.make_expr().unwrap());
                } else {
                    names = None
                }
                let current_offset = if index == 0 {
                    cx.expr_lit(sp, LitKind::Int(0, lit_usize))
                } else {
                    cx.expr_ident(sp, offset)
                };
                let name = field.ident.unwrap();
                compiler.push(quote_stmt!(cx, func.insn_store_relative(value, $current_offset, self.$name.compile(func))).unwrap());
                let size_of = cx.expr_path(cx.path_all(sp, false, vec![cx.ident_of("std"), cx.ident_of("mem"), cx.ident_of("size_of")], vec![], vec![field.ty.clone()], vec![]));
                if def_fields.len() > 1 && index < def_fields.len() - 1 {
                    compiler.push(quote_stmt!(cx, offset += $size_of()).unwrap());
                }
            }
            let fields = cx.expr_mut_addr_of(sp, cx.expr_vec(sp, fields));
            let mut type_expr = cx.expr_call(sp, cx.expr_path(new_struct), vec![fields]);
            if let Some(names) = names {
                let names = cx.expr_vec(sp, names);
                type_expr = quote_expr!(cx, {
                    let mut ty: Type = $type_expr;
                    ty.set_names(&$names);
                    ty.into()
                })
            }
            compiler.push(cx.stmt_expr(cx.expr_ident(sp, value)));
            let kind = s.span(SelfKind::Value(Mutability::Immutable));
            let item = cx.item(sp, name, vec![], ItemKind::Impl(
                Unsafety::Normal,
                ImplPolarity::Positive,
                Generics {
                    lifetimes: vec![ LifetimeDef {lifetime: jit_life, bounds: vec![]}],
                    ty_params: gen.ty_params.clone(),
                    where_clause: WhereClause {
                        id: DUMMY_NODE_ID,
                        predicates: gen.ty_params.iter()
                            .map(|param| WherePredicate::BoundPredicate(
                                WhereBoundPredicate {
                                    span: sp,
                                    bound_lifetimes: vec![],
                                    bounded_ty: cx.ty_ident(sp, param.ident),
                                    bounds: vec![
                                        TyParamBound::TraitTyParamBound(
                                            cx.poly_trait_ref(sp, jit_compile.clone()),
                                            TraitBoundModifier::None
                                        ),
                                    ].into()
                                }
                            ))
                            .collect()
                    }
                },
                Some(cx.trait_ref(jit_compile)),
                self_ty,
                vec![
                    ImplItem {
                        attrs: vec![],
                        id: DUMMY_NODE_ID,
                        span: sp,
                            defaultness: Defaultness::Final,
                        ident: cx.ident_of("get_type"),
                        vis: Visibility::Inherited,
                        node: ImplItemKind::Method(
                            MethodSig {
                                constness: Constness::NotConst,
                                unsafety: Unsafety::Normal,
                                abi: Abi::Rust,
                                // explicit_self: respan(sp, ExplicitSelfKind::SelfStatic),
                                decl: cx.fn_decl(vec![], cx.ty_path(jit_cow_type)),
                                generics: Generics::default(),
                            },
                            cx.block_expr(type_expr)
                        )
                    },
                    ImplItem {
                        attrs: vec![],
                        id: DUMMY_NODE_ID,
                        span: sp,
                            defaultness: Defaultness::Final,
                        ident: cx.ident_of("compile"),
                        vis: Visibility::Inherited,
                        node: ImplItemKind::Method(
                            MethodSig {
                                constness: Constness::NotConst,
                                unsafety: Unsafety::Normal,
                                abi: Abi::Rust,
                                /*explicit_self: respan(
                                    sp,
                                    ExplicitSelfKind::SelfValue(cx.ident_of("b"))),*/
                                decl: cx.fn_decl(
                                    vec![
                                        Arg::from_self(kind,
                                                      s.span(cx.ident_of("self"))),
                                        cx.arg(sp, func, cx.ty_rptr(sp, cx.ty_path(jit_func),
                                                                    None, Mutability::Immutable))],
                                    jit_value),
                                generics: Generics::default(),
                            },
                            cx.block(sp, compiler)
                        )
                    }
                ]
            ));
            push(Annotatable::Item(item));
        },
        _ => {
            cx.span_err(sp, BAD_ITEM);
            return;
        }
    }
}
macro_rules! error(
    ($cx:expr, $span:expr, $text:expr) => ({
        $cx.span_err($span, $text);
        return MacEager::expr($cx.expr_none($span));
    });
);

struct ExprCtxt {
    sp: Span
}

fn compile_expr(cx: &mut ExtCtxt, ctx: &ExprCtxt, expr: P<Expr>) -> P<Expr> {
    let sp = expr.span;
    match expr.node {
        ExprKind::Lit(_) => {
            quote_expr!(cx, $expr.compile(&func))
        },
        ExprKind::Unary(op, ref value) => {
            let value = compile_expr(cx, ctx, value.clone());
            match op {
                UnOp::Deref => {
                    quote_expr!(cx, {
                        let value = $value;
                        func.insn_load_relative(value, 0, value.get_type().get_ref().unwrap())
                    })
                },
                UnOp::Not => quote_expr!(cx, func.insn_not($value)),
                UnOp::Neg => quote_expr!(cx, func.insn_neg($value))
            }
        },
        ExprKind::Binary(op, ref x, ref y) => {
            let x = compile_expr(cx, ctx, x.clone());
            let y = compile_expr(cx, ctx, y.clone());
            match op.node {
                BinOpKind::Add => quote_expr!(cx, func.insn_add($x, $y)),
                BinOpKind::Sub => quote_expr!(cx, func.insn_sub($x, $y)),
                BinOpKind::Mul => quote_expr!(cx, func.insn_mul($x, $y)),
                BinOpKind::Div => quote_expr!(cx, func.insn_div($x, $y)),
                BinOpKind::Rem => quote_expr!(cx, func.insn_rem($x, $y)),
                BinOpKind::And | BinOpKind::BitAnd => quote_expr!(cx, func.insn_and($x, $y)),
                BinOpKind::Or | BinOpKind::BitOr => quote_expr!(cx, func.insn_or($x, $y)),
                BinOpKind::BitXor => quote_expr!(cx, func.insn_xor($x, $y)),
                BinOpKind::Shl => quote_expr!(cx, func.insn_shl($x, $y)),
                BinOpKind::Shr => quote_expr!(cx, func.insn_shr($x, $y)),
                BinOpKind::Eq => quote_expr!(cx, func.insn_eq($x, $y)),
                BinOpKind::Lt => quote_expr!(cx, func.insn_lt($x, $y)),
                BinOpKind::Le => quote_expr!(cx, func.insn_le($x, $y)),
                BinOpKind::Ne => quote_expr!(cx, func.insn_ne($x, $y)),
                BinOpKind::Ge => quote_expr!(cx, func.insn_ge($x, $y)),
                BinOpKind::Gt => quote_expr!(cx, func.insn_gt($x, $y)),
            }
        },
        ExprKind::Cast(ref value, ref ty) => {
            let value = compile_expr(cx, ctx, value.clone());
            let ty = type_expr(cx, sp, ty.clone(), false);
            quote_expr!(cx, func.insn_convert($value, $ty, false))
        },
        ExprKind::If(ref cond, ref block, None) => {
            let cond = compile_expr(cx, ctx, cond.clone());
            let block = cx.expr_block(block.clone());
            let block = compile_expr(cx, ctx, block);
            quote_expr!(cx, func.insn_if($cond, || $block))
        },
        ExprKind::If(ref cond, ref block, Some(ref else_block)) => {
            let cond = compile_expr(cx, ctx, cond.clone());
            let block = cx.expr_block(block.clone());
            let block = compile_expr(cx, ctx, block);
            let else_block = compile_expr(cx, ctx, else_block.clone());
            quote_expr!(cx, func.insn_if_else($cond, || $block, || $else_block))
        },
        ExprKind::Ret(None) => quote_expr!(cx,  func.insn_default_return()),
        ExprKind::Ret(Some(ref value)) => {
            let value = compile_expr(cx, ctx, value.clone());
            quote_expr!(cx, func.insn_return($value))
        },
        ExprKind::Loop(ref block, _) => {
            let block = cx.expr_block(block.clone());
            let block = compile_expr(cx, ctx, block);
            quote_expr!(cx, func.insn_loop(|| $block))
        },
        ExprKind::While(ref cond, ref block, _) => {
            let cond = compile_expr(cx, ctx, cond.clone());
            let block = cx.expr_block(block.clone());
            let block = compile_expr(cx, ctx, block);
            quote_expr!(cx, func.insn_while(|| $cond, || $block))
        },
        ExprKind::AddrOf(_, ref value) => {
            let value = compile_expr(cx, ctx, value.clone());
            quote_expr!(cx,  func.insn_addr_of($value))
        },
        ExprKind::Path(None, _) => expr.clone(),
        ExprKind::MethodCall(name, ref tys, ref args) if tys.len() == 0 && args.len() == 1 => {
            let name = name.node.name.as_str();
            let value = args[0].clone();
            let value = compile_expr(cx, ctx, value);
            match &*name {
                "abs" => quote_expr!(cx, func.insn_abs($value)),
                "acos" => quote_expr!(cx, func.insn_acos($value)),
                "asin" => quote_expr!(cx, func.insn_asin($value)),
                "atan" => quote_expr!(cx, func.insn_atan($value)),
                "ceil" => quote_expr!(cx, func.insn_ceil($value)),
                "cos" => quote_expr!(cx, func.insn_cos($value)),
                "floor" => quote_expr!(cx, func.insn_floor($value)),
                "is_finite" => quote_expr!(cx, func.insn_is_finite($value)),
                "is_infinite" => quote_expr!(cx, func.insn_is_inf($value)),
                "is_nan" => quote_expr!(cx, func.insn_is_nan($value)),
                "sin" => quote_expr!(cx, func.insn_sin($value)),
                "sqrt" => quote_expr!(cx, func.insn_sqrt($value)),
                "tan" => quote_expr!(cx, func.insn_tan($value)),
                "trunc" => quote_expr!(cx, func.insn_trunc($value)),
                _ => {
                    cx.span_err(sp, &format!("Method {} is not supported by LibJIT", name));
                    quote_expr!(cx, ())
                }
            }
        },
        ExprKind::Paren(ref ex) => compile_expr(cx, ctx, ex.clone()),
        _ => {
            use syntax::print::pprust::expr_to_string;
            cx.span_err(expr.span, &format!("bad expr {:?}", &expr_to_string(&*expr)));
            quote_expr!(cx, ())
        }
    }
}
fn expand_jit<'a>(cx: &'a mut ExtCtxt, sp: Span, _: Ident, tt: Vec<TokenTree>) -> Box<MacResult + 'a> {
    if let Some(exprs) = get_exprs_from_tts(cx, sp, &tt) {
        let ctx = ExprCtxt {
            sp: sp
        };
        let mut stmts = Vec::new();
        if let ExprKind::Closure(_, ref decl, ref block, _) = exprs[1].node {
            let ty = cx.ty(sp, TyKind::BareFn(P(BareFnTy {
                unsafety: Unsafety::Normal,
                abi: Abi::Rust,
                lifetimes: vec![],
                decl: decl.clone()
            })));
            let ty_expr = type_expr(cx, sp, ty, false);
            let ctx_expr = exprs[0].clone();
            for (index, arg) in decl.inputs.iter().enumerate() {
                if let PatKind::Ident(_, ref ident, None) = arg.pat.node {
                    let arg = ident.node;
                    stmts.push(quote_stmt!(cx, let $arg = &func[$index]).unwrap());
                }
            }
            for stmt in &block.stmts {
                match stmt.node {
                    StmtKind::Local(ref local) => {
                        if let PatKind::Ident(_, ref name, None) = local.pat.node {
                            let name = name.node.clone();
                            if let Some(ref init) = local.init {
                                let expr = compile_expr(cx, &ctx, init.clone());
                                stmts.push(quote_stmt!(cx, let $name = $expr).unwrap());
                            }
                        }
                    },
                    StmtKind::Expr(ref expr) => {
                        let compiled = compile_expr(cx, &ctx, expr.clone());
                        stmts.push(cx.stmt_expr(compiled));
                    },
                    _ => ()
                }
            }/*
            if let Some(ref value) = block {
                let value = compile_expr(cx, &ctx, value.clone());
                stmts.push(quote_stmt!(cx, func.insn_return($value)).unwrap());
            } else {
                stmts.push(quote_stmt!(cx, func.insn_default_return()).unwrap());
            }*/
            let usage = exprs[2].clone();
            let expr = cx.expr_block(cx.block(sp, stmts));
            let expr = quote_expr!(cx, {
                let func = jit::UncompiledFunction::new($ctx_expr, $ty_expr);
                $expr;
                func
            }.compile().with($usage));
            MacEager::expr(expr)
        } else {
            error!(cx, sp, "Function should be given as closure")
        }
    } else {
        error!(cx, sp, "Failed to parse")
    }
}
pub fn register(reg: &mut Registry) {
    reg.add_decorator("derive_Compile", expand_derive_compile);
    reg.add_ident_macro("jit", expand_jit);
}

#[macro_export]
/// Construct a JIT struct with the fields given
macro_rules! jit_struct(
    ($($name:ident: $ty:ty),*) => ({
        let mut ty = Type::new_struct(&mut [
            $(&get::<$ty>()),*
        ]);
        ty.set_names(&[$(stringify!($name)),*]);
        ty
    });
    ($($ty:ty),+ ) => (
        Type::new_struct(&mut [
            $(&get::<$ty>()),+
        ])
    )
);

#[macro_export]
/// Construct a JIT union with the fields given
macro_rules! jit_union(
    ($($name:ident: $ty:ty),*) => ({
        let union = Type::new_union(&mut [
            $(&get::<$ty>()),*
        ]);
        union.set_names(&[$(stringify!($name)),*]);
        union
    });
    ($($ty:ty),+ ) => (
        Type::new_union(&mut [
            $(&get::<$ty>()),*
        ])
    )
);
#[macro_export]
/// Construct a JIT function signature with the arguments and return type given
macro_rules! jit_fn(
    ($($arg:ty),* => $ret:ty) => ({
        use std::default::Default;
        Type::new_signature(Default::default(), &get::<$ret>(), &mut [
            $(&get::<$arg>()),*
        ])
    });
    (raw $($arg:expr),* => $ret:expr) => ({
        use std::default::Default;
        Type::new_signature(Default::default(), &$ret, &mut [
            $(&$arg),*
        ])
    });
);

#[macro_export]
macro_rules! jit(
    ($func:ident, return) => (
        $func.insn_default_return()
    );
    ($func:ident, return $($t:tt)+) => (
        $func.insn_return(jit!($func, $($t)+))
    );
    ($func:ident, $var:ident += $($t:tt)+) => (
        $func.insn_store($var, &$func.insn_add($var, jit!($func, $($t)+)));
    );
    ($func:ident, $var:ident -= $($t:tt)+) => (
        $func.insn_store($var, &$func.insn_sub($var, jit!($func, $($t)+)));
    );
    ($func:ident, $var:ident *= $($t:tt)+) => (
        $func.insn_store($var, &$func.insn_mul($var, jit!($func, $($t)+)));
    );
    ($func:ident, $var:ident /= $($t:tt)+) => (
        $func.insn_store($var, &$func.insn_div($var, jit!($func, $($t)+)));
    );
    ($func:ident, $($a:tt)+ + $($b:tt)+) => (
        $func.insn_add(jit!($func, $($a)+), jit!($func, $($b)+))
    );
    ($func:ident, $($a:tt)+ - $($b:tt)+) => (
        $func.insn_sub(jit!($func, $($a)+), jit!($func, $($b)+))
    );
    ($func:ident, $($a:tt)+ * $($b:tt)+) => (
        $func.insn_mul(jit!($func, $($a)+), jit!($func, $($b)+))
    );
    ($func:ident, $($a:tt)+ / $($b:tt)+) => (
        $func.insn_div(jit!($func, $($a)+), jit!($func, $($b)+))
    );
    ($func:ident, $($a:tt)+ % $($b:tt)+) => (
        $func.insn_rem(jit!($func, $($a)+), jit!($func, $($b)+))
    );
    ($func:ident, ($($t:tt)+).sqrt()) => (
        $func.insn_sqrt(&jit!($func, $($t)+))
    );
    ($func:ident, $var:ident = $($t:tt)+) => (
        $func.insn_store($var, jit!($func, $val));
    );
    ($func:ident, *$var:ident) => (
        $func.insn_load($var)
    );
    ($func:ident, call($call:expr,
        $($arg:expr),+
    )) => (
        $func.insn_call(None::<String>, $call, None, [$($arg),+].as_mut_slice())
    );
    ($func:ident, jump_table($value:expr,
        $($label:ident),+
    )) => (
    let ($($label),+) = {
        $(let $label:Label = Label::new($func);)+
        $func.insn_jump_table($value, [
            $($label),+
        ].as_mut_slice());
        ($($label),+)
    });
);
#[macro_export]
macro_rules! jit_func(
    ($ctx:expr, $name:ident, fn() -> $ret:ty {$($st:stmt;)+}, $value:expr) => ({
        let sig = get::<fn() -> $ret>();
        let func = UncompiledFunction::new($ctx, &sig);
        {
            let $name = &func;
            $($st;)+
        };
        let compiled = UncompiledFunction::compile(func);
        CompiledFunction::to_closure::<(), $ret>(compiled)
    });
    ($ctx:expr, $name:ident, fn($($arg:ident:$ty:ty),+) -> $ret:ty {$($st:stmt;)+}, $value:expr) => ({
        let sig = get::<fn($($ty),+) -> $ret>();
        let func = UncompiledFunction::new($ctx, &sig);
        {
            let $name = &func;
            let mut i = 0;
            $(let $arg = {
                i += 1;
                &$name[i - 1]
            };)*
            $($st;)+
        };
        let compiled = UncompiledFunction::compile(func);
        CompiledFunction::to_closure::<($($ty),+), $ret>(compiled)
    });
);
