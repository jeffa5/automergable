use std::{collections::HashMap, convert::Infallible};

use automerge::{MapType, Path, Primitive, Value};
use automergeable::diff_values;
use quickcheck::{empty_shrinker, Arbitrary, Gen, QuickCheck, TestResult};

#[derive(Debug, Clone, PartialEq)]
struct Prim(Primitive);

impl Arbitrary for Prim {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        let prims = [
            0, // Str(String),
            1, // Int(i64),
            2, // Uint(u64),
            3, // F64(f64),
            4, // F32(f32),
            5, // Counter(i64),
            6, // Timestamp(i64),
            7, // Boolean(bool),
            8, // Cursor(Cursor),
            9, // Null
        ];
        let prim = g.choose(&prims).unwrap();
        let p = match prim {
            0 => Primitive::Str(String::arbitrary(g)),
            1 => Primitive::Int(i64::arbitrary(g)),
            2 => Primitive::Uint(u64::arbitrary(g)),
            3 => Primitive::F64(i32::arbitrary(g) as f64), // avoid having NaN in as it breaks the equality
            4 => Primitive::F32(i32::arbitrary(g) as f32), // avoid having NaN in as it breaks the equality
            5 => Primitive::Counter(i64::arbitrary(g)),
            6 => Primitive::Timestamp(i64::arbitrary(g)),
            7 => Primitive::Boolean(bool::arbitrary(g)),
            8 => Primitive::Null, // TODO: convert this case to use an arbitrary cursor
            _ => Primitive::Null,
        };
        Self(p)
    }

    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        match &self.0 {
            Primitive::Str(s) => Box::new(s.shrink().map(Primitive::Str).map(Prim)),
            Primitive::Int(i) => Box::new(i.shrink().map(Primitive::Int).map(Prim)),
            Primitive::Uint(u) => Box::new(u.shrink().map(Primitive::Uint).map(Prim)),
            Primitive::F64(f) => Box::new(f.shrink().map(Primitive::F64).map(Prim)),
            Primitive::F32(f) => Box::new(f.shrink().map(Primitive::F32).map(Prim)),
            Primitive::Counter(c) => Box::new(c.shrink().map(Primitive::Counter).map(Prim)),
            Primitive::Timestamp(t) => Box::new(t.shrink().map(Primitive::Timestamp).map(Prim)),
            Primitive::Boolean(b) => Box::new(b.shrink().map(Primitive::Boolean).map(Prim)),
            Primitive::Cursor(_) => empty_shrinker(),
            Primitive::Null => empty_shrinker(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct MapTy(MapType);

impl Arbitrary for MapTy {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        if *g.choose(&[0, 1]).unwrap() == 0 {
            MapTy(MapType::Map)
        } else {
            MapTy(MapType::Table)
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Val(Value);

impl Arbitrary for Val {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        let depth = g.choose(&[1, 2, 3]).unwrap();
        arbitrary_value(g, *depth)
    }

    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        match &self.0 {
            Value::Map(m, mt) => {
                let m = m
                    .iter()
                    .map(|(k, v)| (k.clone(), Val(v.clone())))
                    .collect::<HashMap<_, _>>();
                let mt = *mt;
                Box::new(
                    m.shrink()
                        .map(move |m| {
                            let m = m
                                .into_iter()
                                .map(|(k, v)| (k, v.0))
                                .collect::<HashMap<_, _>>();
                            Value::Map(m, mt)
                        })
                        .map(Val),
                )
            }
            Value::Sequence(v) => {
                let v = v.iter().map(|v| Val(v.clone())).collect::<Vec<_>>();
                Box::new(
                    v.shrink()
                        .map(|v| {
                            let v = v.into_iter().map(|i| i.0).collect::<Vec<_>>();
                            Value::Sequence(v)
                        })
                        .map(Val),
                )
            }
            Value::Text(v) => Box::new(v.shrink().map(Value::Text).map(Val)),
            Value::Primitive(p) => Box::new(
                Prim(p.clone())
                    .shrink()
                    .map(|p| p.0)
                    .map(Value::Primitive)
                    .map(Val),
            ),
        }
    }
}

fn arbitrary_value(g: &mut Gen, depth: usize) -> Val {
    let vals = if depth == 0 {
        &[
            2, // Text(Vec<char, Global>),
            3, // Primitive(Primitive),
        ][..]
    } else {
        &[
            0, // Map(HashMap<String, Value, RandomState>, MapType),
            1, // Sequence(Vec<Value, Global>),
            2, // Text(Vec<char, Global>),
            3, // Primitive(Primitive),
        ][..]
    };
    let val = g.choose(vals).unwrap();
    let v = match val {
        0 => {
            let smaller_depth = depth / 2;
            let map = HashMap::<String, ()>::arbitrary(g);
            let map = map
                .into_iter()
                .map(|(k, ())| (k, arbitrary_value(g, smaller_depth).0))
                .collect::<HashMap<_, _>>();
            let map_type = MapTy::arbitrary(g);
            Value::Map(map, map_type.0)
        }
        1 => {
            let smaller_depth = depth / 2;
            let vec = Vec::<()>::arbitrary(g);
            let vec = vec
                .into_iter()
                .map(|()| arbitrary_value(g, smaller_depth).0)
                .collect::<Vec<_>>();
            Value::Sequence(vec)
        }
        2 => {
            let vec = Vec::<char>::arbitrary(g);
            Value::Text(vec)
        }
        _ => Value::Primitive(Prim::arbitrary(g).0),
    };
    Val(v)
}

#[test]
fn equal_primitives_give_no_diff() {
    fn no_diff(p1: Prim, p2: Prim) -> TestResult {
        if p1 != p2 {
            return TestResult::discard();
        }
        let v1 = Value::Primitive(p1.0);
        let v2 = Value::Primitive(p2.0);
        let changes = diff_values(&v1, &v2);
        if let Ok(changes) = changes {
            if changes.is_empty() {
                TestResult::passed()
            } else {
                println!("{:?}", changes);
                TestResult::failed()
            }
        } else {
            TestResult::discard()
        }
    }
    QuickCheck::new()
        .tests(100_000_000)
        .quickcheck(no_diff as fn(Prim, Prim) -> TestResult)
}

#[test]
fn equal_values_give_no_diff() {
    fn no_diff(v1: Val, v2: Val) -> TestResult {
        if v1 != v2 {
            return TestResult::discard();
        }
        let changes = diff_values(&v1.0, &v2.0);
        if let Ok(changes) = changes {
            if changes.is_empty() {
                TestResult::passed()
            } else {
                println!("{:?}", changes);
                TestResult::failed()
            }
        } else {
            TestResult::discard()
        }
    }
    QuickCheck::new()
        .tests(100_000_000)
        .gen(Gen::new(20))
        .quickcheck(no_diff as fn(Val, Val) -> TestResult)
}

#[test]
fn applying_primitive_diff_result_to_old_gives_new() {
    fn apply_diff(p1: Prim, p2: Prim) -> TestResult {
        let mut h1 = HashMap::new();
        h1.insert("k".to_owned(), Value::Primitive(p1.0));
        let v1 = Value::Map(h1, MapType::Map);
        let mut h2 = HashMap::new();
        h2.insert("k".to_owned(), Value::Primitive(p2.0));
        let v2 = Value::Map(h2, MapType::Map);
        let changes = diff_values(&v1, &v2);
        let changes = if let Ok(changes) = changes {
            changes
        } else {
            return TestResult::discard();
        };
        let mut b = automerge::Backend::init();
        // new with old value
        let (mut f, c) = automerge::Frontend::new_with_initial_state(v2).unwrap();
        let (p, _) = b.apply_local_change(c).unwrap();
        f.apply_patch(p).unwrap();

        // apply changes to reach new value
        let c = f
            .change::<_, Infallible>(None, |d| {
                for change in changes {
                    d.add_change(change).unwrap()
                }
                Ok(())
            })
            .unwrap();
        if let Some(c) = c {
            let (p, _) = b.apply_local_change(c).unwrap();
            f.apply_patch(p).unwrap();
        }

        let val = f.get_value(&Path::root()).unwrap();
        if val == v1 {
            TestResult::passed()
        } else {
            println!("expected: {:?}, found: {:?}", v1, val);
            TestResult::failed()
        }
    }

    QuickCheck::new()
        .tests(100_000_000)
        .quickcheck(apply_diff as fn(Prim, Prim) -> TestResult)
}

#[test]
fn applying_value_diff_result_to_old_gives_new() {
    fn apply_diff(v1: Val, v2: Val) -> TestResult {
        if let Val(Value::Map(_, MapType::Map)) = v1 {
        } else {
            return TestResult::discard();
        }
        if let Val(Value::Map(_, MapType::Map)) = v2 {
        } else {
            return TestResult::discard();
        }
        let changes = diff_values(&v1.0, &v2.0);
        let changes = if let Ok(changes) = changes {
            changes
        } else {
            return TestResult::discard();
        };
        let mut b = automerge::Backend::init();
        // new with old value
        let (mut f, c) = automerge::Frontend::new_with_initial_state(v2.0).unwrap();
        let (p, _) = b.apply_local_change(c).unwrap();
        f.apply_patch(p).unwrap();

        // apply changes to reach new value
        let c = f
            .change::<_, Infallible>(None, |d| {
                for change in &changes {
                    d.add_change(change.clone()).unwrap()
                }
                Ok(())
            })
            .unwrap();
        if let Some(c) = c {
            let (p, _) = b.apply_local_change(c).unwrap();
            if let Err(e) = f.apply_patch(p) {
                println!("{:?}", changes);
                panic!("{}", e)
            }
        }

        let val = f.get_value(&Path::root()).unwrap();
        if val == v1.0 {
            TestResult::passed()
        } else {
            println!("expected: {:?}, found: {:?}", v1, val);
            TestResult::failed()
        }
    }

    QuickCheck::new()
        .tests(100_000_000)
        .quickcheck(apply_diff as fn(Val, Val) -> TestResult)
}