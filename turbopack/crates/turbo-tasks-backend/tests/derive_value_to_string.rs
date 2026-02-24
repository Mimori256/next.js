#![feature(arbitrary_self_types)]
#![feature(arbitrary_self_types_pointers)]
#![allow(clippy::needless_return)] // tokio macro-generated code doesn't respect this

use std::fmt;

use turbo_rcstr::RcStr;
use turbo_tasks::{ReadRef, ResolvedVc, ValueToString, Vc};
use turbo_tasks_testing::{Registration, register, run_once};

static REGISTRATION: Registration = register!();

// --- Test types ---

#[turbo_tasks::value(shared)]
#[derive(ValueToString)]
struct SimpleDisplay(u32);

impl fmt::Display for SimpleDisplay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "simple:{}", self.0)
    }
}

#[turbo_tasks::value(shared)]
#[derive(ValueToString)]
#[value_to_string("item {name} (count: {count})")]
struct NamedFields {
    name: RcStr,
    count: u32,
}

#[turbo_tasks::value(shared)]
#[derive(ValueToString)]
#[value_to_string("wrapped({0})")]
struct TupleStruct(u32);

#[turbo_tasks::value(shared)]
#[derive(ValueToString)]
#[value_to_string("constant-value")]
struct ConstantString;

#[turbo_tasks::value(shared)]
#[derive(ValueToString)]
#[value_to_string(self.name)]
struct DirectExpr {
    name: RcStr,
    #[allow(dead_code)]
    other: u32,
}

#[turbo_tasks::value(shared)]
#[derive(ValueToString)]
#[value_to_string("prefix({name}) suffix({count})")]
struct FormatExprs {
    name: RcStr,
    count: u32,
}

#[turbo_tasks::value(shared)]
#[derive(ValueToString)]
#[value_to_string("inner: {inner}")]
struct VcExprDelegate {
    inner: ResolvedVc<NamedFields>,
}

#[turbo_tasks::value(shared)]
#[derive(ValueToString)]
enum Kind {
    #[value_to_string("module")]
    Module,
    #[value_to_string("asset({0})")]
    Asset(RcStr),
    #[value_to_string("entry {name}")]
    Entry { name: RcStr },
}

#[turbo_tasks::value(shared)]
#[derive(ValueToString)]
enum DefaultNames {
    Alpha,
    Beta,
}

#[turbo_tasks::value(shared)]
#[derive(ValueToString)]
enum MixedEnum {
    #[value_to_string("literal")]
    Literal,
    #[value_to_string(_0)]
    Delegate(ResolvedVc<ConstantString>),
    #[value_to_string("wrapped({})", name)]
    ExprNamed { name: RcStr },
}

// --- Tests ---

/// No attribute: delegates to Display::to_string(self).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_display_delegation() {
    run_once(&REGISTRATION, || async {
        let v: Vc<SimpleDisplay> = SimpleDisplay(42).cell();
        assert_eq!(&*v.to_string().await?, "simple:42");
        anyhow::Ok(())
    })
    .await
    .unwrap()
}

/// FormatAutoFields on structs: named fields, positional fields, and constant strings.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_struct_format_strings() {
    run_once(&REGISTRATION, || async {
        let v1: Vc<NamedFields> = NamedFields {
            name: "foo".into(),
            count: 7,
        }
        .cell();
        assert_eq!(&*v1.to_string().await?, "item foo (count: 7)");

        let v2: Vc<TupleStruct> = TupleStruct(99).cell();
        assert_eq!(&*v2.to_string().await?, "wrapped(99)");

        let v3: Vc<ConstantString> = ConstantString.cell();
        assert_eq!(&*v3.to_string().await?, "constant-value");

        anyhow::Ok(())
    })
    .await
    .unwrap()
}

/// DirectExpr form: single expression delegation.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_struct_direct_expr() {
    run_once(&REGISTRATION, || async {
        let v: Vc<DirectExpr> = DirectExpr {
            name: "hello".into(),
            other: 42,
        }
        .cell();
        assert_eq!(&*v.to_string().await?, "hello");
        anyhow::Ok(())
    })
    .await
    .unwrap()
}

/// FormatExprs on structs: format string with explicit expressions, including Vc delegation.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_struct_format_exprs() {
    run_once(&REGISTRATION, || async {
        let v1: Vc<FormatExprs> = FormatExprs {
            name: "test".into(),
            count: 5,
        }
        .cell();
        assert_eq!(&*v1.to_string().await?, "prefix(test) suffix(5)");

        let inner = NamedFields {
            name: "bar".into(),
            count: 3,
        }
        .resolved_cell();
        let v2: Vc<VcExprDelegate> = VcExprDelegate { inner }.cell();
        assert_eq!(&*v2.to_string().await?, "inner: item bar (count: 3)");

        anyhow::Ok(())
    })
    .await
    .unwrap()
}

/// Enum with per-variant auto-field format strings and default variant names.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_enum_variants() {
    run_once(&REGISTRATION, || async {
        // Per-variant attributes
        assert_eq!(&*Kind::Module.cell().to_string().await?, "module");
        assert_eq!(
            &*Kind::Asset("main.js".into()).cell().to_string().await?,
            "asset(main.js)"
        );
        assert_eq!(
            &*(Kind::Entry {
                name: "index".into()
            })
            .cell()
            .to_string()
            .await?,
            "entry index"
        );

        // Default variant names (no attribute)
        assert_eq!(&*DefaultNames::Alpha.cell().to_string().await?, "Alpha");
        assert_eq!(&*DefaultNames::Beta.cell().to_string().await?, "Beta");

        anyhow::Ok(())
    })
    .await
    .unwrap()
}

/// Enum with mixed forms: constant literal, Vc delegation, and format exprs.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_mixed_enum() {
    run_once(&REGISTRATION, || async {
        assert_eq!(&*MixedEnum::Literal.cell().to_string().await?, "literal");

        let inner = ConstantString.resolved_cell();
        let v2: Vc<MixedEnum> = MixedEnum::Delegate(inner).cell();
        assert_eq!(&*v2.to_string().await?, "constant-value");

        let v3: Vc<MixedEnum> = (MixedEnum::ExprNamed {
            name: "world".into(),
        })
        .cell();
        assert_eq!(&*v3.to_string().await?, "wrapped(world)");

        anyhow::Ok(())
    })
    .await
    .unwrap()
}

/// A Display type for torture-testing enum field resolution.
#[turbo_tasks::value(shared)]
struct DisplayVal(u32);

impl fmt::Display for DisplayVal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "dv:{}", self.0)
    }
}

/// Torture test: one enum variant with Display, RcStr, Vc, and ResolvedVc fields,
/// exercising all ValueToStringify dispatch levels in a single match arm.
#[turbo_tasks::value(shared)]
#[derive(ValueToString)]
enum TortureEnum {
    /// Auto-field format with Display, RcStr, and ResolvedVc in one variant.
    #[value_to_string("d={display_val} r={rc_str} rv1={resolved_named} rv2={resolved_const}")]
    AllFieldTypes {
        display_val: DisplayVal,
        rc_str: RcStr,
        resolved_named: ResolvedVc<NamedFields>,
        resolved_const: ResolvedVc<ConstantString>,
    },
    /// FormatExprs form with a ResolvedVc positional arg.
    #[value_to_string("expr({})", _0)]
    ExprVc(ResolvedVc<NamedFields>),
    /// DirectExpr form delegating to a ResolvedVc field.
    #[value_to_string(_0)]
    DelegateVc(ResolvedVc<ConstantString>),
    /// ReadRef field.
    #[value_to_string("read={0}")]
    WithReadRef(ReadRef<NamedFields>),
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_torture_enum() {
    run_once(&REGISTRATION, || async {
        let named_resolved = NamedFields {
            name: "x".into(),
            count: 1,
        }
        .resolved_cell();
        let named_resolved2 = NamedFields {
            name: "y".into(),
            count: 2,
        }
        .resolved_cell();
        let const_resolved = ConstantString.resolved_cell();

        // AllFieldTypes: Display + RcStr + ResolvedVc + ResolvedVc
        let v1 = (TortureEnum::AllFieldTypes {
            display_val: DisplayVal(42),
            rc_str: "hello".into(),
            resolved_named: named_resolved,
            resolved_const: const_resolved,
        })
        .cell();
        assert_eq!(
            &*v1.to_string().await?,
            "d=dv:42 r=hello rv1=item x (count: 1) rv2=constant-value"
        );

        // ExprVc: FormatExprs with ResolvedVc positional arg
        let v2 = TortureEnum::ExprVc(named_resolved2).cell();
        assert_eq!(&*v2.to_string().await?, "expr(item y (count: 2))");

        // DelegateVc: DirectExpr delegating to ResolvedVc
        let v3 = TortureEnum::DelegateVc(const_resolved).cell();
        assert_eq!(&*v3.to_string().await?, "constant-value");

        // WithReadRef: ReadRef field
        let named_read: ReadRef<NamedFields> = named_resolved.await?;
        let v4 = TortureEnum::WithReadRef(named_read).cell();
        assert_eq!(&*v4.to_string().await?, "read=item x (count: 1)");

        anyhow::Ok(())
    })
    .await
    .unwrap()
}
