use std::fmt::Display;

use smallvec::smallvec;
use crate::{b, Bstr};
use crate::codepage::tochars;
use crate::token::Tok;

// Value expression
#[derive(Clone, Debug)]
pub enum Ve {
    Var(Bstr),
    Num(f64),
    Snd(Vec<Ve>),  // strand
    Nom(Fe),       // function as value
    Afn1 { a: Box<Ve>, f: Fe             },  // apply monadic function
    Afn2 { a: Box<Ve>, f: Fe, b: Box<Ve> },  // apply dyadic  function 
}
// Function expression
#[derive(Clone, Debug)]
pub enum Fe {
    Var(Bstr),
    SetVar(Bstr),
    Aav1 {            v: Bstr   , g: Box<Tg>}, //     apply monadic adverb
    Aav2 {f: Box<Tg>, v: Bstr   , g: Box<Tg>}, //     apply dyadic  adverb
    Bind {            f: Box<Fe>, b: Box<Ve>}, // +1
    Trn1 {a: Box<Fe>, f: Box<Fe>            }, // +/
    Trn2 {a: Box<Fe>, f: Box<Fe>, b: Box<Ve>}, // +/2
    Dfn(Vec<Ve>),
}

// a thing (Tg) is either:
// - a function
// - a value
#[derive(Clone, Debug)]
pub enum Tg {
    Fe(Fe),
    Ve(Ve)
}

fn displayname(bytes: &[u8]) -> String {
    if bytes.contains(&b' ') {
        format!("\"{}\"", tochars(bytes).replace('"', "\""))
    } else {
        tochars(bytes)
    }
}

impl Display for Ve {
    fn fmt(&self, m: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ve::Var(v) => write!(m, ".{}", displayname(v)),
            Ve::Num(n) => write!(m, "'{}", n),
            Ve::Snd(l) => {
                write!(m, "(")?;
                for v in l { write!(m, "{}", v)?; }
                write!(m, ")")?;
                Ok(())
            },
            Ve::Nom(v) => write!(m, "♪{}", v),
            Ve::Afn1 { a, f } => write!(m, "({} {})", a, f),
            Ve::Afn2 { a, f, b } => write!(m, "({} {} {})", a, f, b),
        }
    }
}

impl Display for Fe {
    fn fmt(&self, m: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Fe::Var(v) => write!(m, ":{}", displayname(v)),
            Fe::SetVar(v) => write!(m, "→{}", displayname(v)),
            Fe::Aav1 {    v, g } => write!(m, "[•{} {}]", displayname(v), g),
            Fe::Aav2 { f, v, g } => write!(m, "[{} ○{} {}]", f, displayname(v), g),
            Fe::Bind {    f, b } => write!(m, "[{} with {}]", f, b),
            Fe::Trn1 { a, f    } => write!(m, "[{} {}]", a, f),
            Fe::Trn2 { a, f, b } => write!(m, "[{} {} {}]", a, f, b),
            Fe::Dfn(efs) => {
                write!(m, "{{ ")?;
                for v in efs { write!(m, "{}; ", v)?; }
                write!(m, "}}")?;
                Ok(())
            },
        }
    }
}

impl Display for Tg {
    fn fmt(&self, m: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self { Tg::Fe(f) => write!(m, "{}", f),
                     Tg::Ve(v) => write!(m, "{}", v)}
    }
}

fn slice_offset<T>(code: &[T], slice: &[T]) -> usize {
    if slice.is_empty() { return code.len() }
    let ptr = slice.as_ptr();
    assert!(code.as_ptr_range().contains(&ptr));
    unsafe {
        // SAFETY: we check that `slice` is a subset of `code`, so this SHOULD be fine
        ptr.offset_from(code.as_ptr()) as usize
    }
}

fn function(code: &[Tok]) -> (usize, Option<Fe>) {
    if let Some(t) = code.first() {
        match t {
            // these are functions until when they aren't
            Tok::Stmt(c @ b!('☺''☻''⌂')) => (1, Some(Fe::Var(smallvec![*c]))),
            Tok::VarSet(v) => (1, Some(Fe::SetVar(v.clone()))),
            // monadic adverbs
            Tok::VarAv1(name) => {
                let (offset, thing) = thing(&code[1..]);
                (offset+1, thing.map(|x| Fe::Aav1 {v: name.clone(), g: Box::new(x)}))
            },
            Tok::Just(b'{') => {
                let mut slice = &code[1..];
                let (len, b) = block(slice); slice = &slice[len..];
                (
                    slice_offset(code, slice) + usize::from(matches!(slice.first(), Some(Tok::Just(b'}')))),
                    Some(Fe::Dfn(b))
                )
            }
            Tok::VarFun(v) => (1, Some(Fe::Var(v.clone()))),
            _ => (0, None)
        }
    } else {(0, None)}
}

fn atom_token(chr: Tok) -> Option<Ve> {
    Some(match chr {
        Tok::Just(c @ b'0'..=b'9') => Ve::Num(f64::from(c - b'0')),
        Tok::Just(b!('Φ')) => Ve::Num(10.),
        Tok::Just(b!('Θ')) => Ve::Num(-1.),
        Tok::Just(b!('∞')) => Ve::Num(f64::INFINITY),
        Tok::Just(b!('█')) => Ve::Num(f64::NAN),
        Tok::Just(b!('ϕ')) => Ve::Snd(Vec::new()),
        Tok::VarVal(x) => Ve::Var(x),
        Tok::Chr(x) =>
            Ve::Num(if x <= 10 { -f64::from(x) } else { f64::from(x) }),
        Tok::Chr2(x, y) =>
            Ve::Snd(vec![Ve::Num(f64::from(x)), Ve::Num(f64::from(y))]),
        Tok::Num2(x, y) =>
            Ve::Num(f64::from(x)*253. + f64::from(y)),
        Tok::Num3(x, y, z) =>
            Ve::Num(f64::from(x)*253.*253. + f64::from(y)*253. + f64::from(z)),
        Tok::Num(l) => {
            let mut num = 0.;
            for i in l { num = num * 253. + f64::from(i) }
            Ve::Num(num)
        }
        Tok::HNum(x) => unsafe {
            // safety: HNums have only [0-9.]+, all are ascii characters
            Ve::Num(std::str::from_utf8_unchecked(&x).parse::<f64>().unwrap())
        },
        Tok::Str(x) =>
            Ve::Snd(x.iter().map(|&x| Ve::Num(f64::from(x))).collect()),
        _ => return None,
    })
}

fn value(code: &[Tok]) -> (usize, Option<Ve>) {
    if let Some(t) = code.first() {
        match t.clone() {
            Tok::Just(b'(') => {
                let mut slice = &code[1..];
                let (len, ev) = expression(slice, usize::MAX); slice = &slice[len..];
                (
                    slice_offset(code, slice) + usize::from(matches!(slice[0], Tok::Just(b')'))),
                    Some(ev.unwrap_or(Ve::Snd(vec![])))
                )
            },
            Tok::Just(b!('♪')) => {
                let (len, t) = thing(&code[1..]);
                if let Some(t) = t { (len+1, Some(match t {
                    Tg::Ve(ev) => Ve::Snd(vec![ev]),
                    Tg::Fe(ef) => Ve::Nom(ef),
                }))} else {(0, None)}
            },
            Tok::Just(s @ b!('┌''└''┐''┘')) => {
                let (len, ev) = expression(&code[1..], match s {
                    b!('┌')=>2, b!('└')=>3, b!('┐')=>4, b!('┘')=>5, _=>unreachable!()
                });
                if ev.is_some() {(len + 1, ev)} else {(0, None)}
            },
            Tok::Just(s @ b!('╔''╚''╗''╝')) => {
                let (len, ev) = expression_bites(&code[1..], match s {
                    b!('╔')=>2, b!('╚')=>3, b!('╗')=>4, b!('╝')=>5, _=>unreachable!()
                });
                if ev.is_some() {(len + 1, ev)} else {(0, None)}
            },
            t => { let p = atom_token(t); (usize::from(p.is_some()), p) }
        }
    } else { (0, None) }
}

fn atom(code: &[Tok]) -> (usize, Option<Tg>) {
    let thing: Tg;
    let mut slice = code;
    // try with a value
    let (len, oev) = value(slice); slice = &slice[len..];
    if let Some(ev) = oev {
        thing = Tg::Ve(ev);
    } else {
        // try with a function
        let (len, oef) = function(slice); slice = &slice[len..];
        if let Some(ef) = oef {
            thing = Tg::Fe(ef);
        } else {
            // huh.
            return (0, None);
        }
    }
    (slice_offset(code, slice), Some(thing))
}

fn thing(code: &[Tok]) -> (usize, Option<Tg>) {
    let mut slice = code;
    let (len, tg) = atom(code); slice = &slice[len..];
    if let Some(mut tg) = tg {
        if let Some(Tok::VarAv2(name)) = slice.first() {
            slice = &slice[1..];
            let (len, t) = thing(slice); slice = &slice[len..];
            tg = Tg::Fe(Fe::Aav2 {
                f: Box::new(tg),
                v: name.clone(),
                g: Box::new(t.unwrap_or(Tg::Ve(Ve::Num(f64::NAN))))});
        }
        (slice_offset(code, slice), Some(tg))
    } else {(0, None)}
}

fn thing_bites(code: &[Tok], mut bites: usize) -> (usize, Option<Tg>, usize /*bites*/) {
    if bites == 0 { return (0, None, 0) }
    let mut slice = code;
    let (len, thing) = atom(code); slice = &slice[len..]; bites -= 1;
    if let Some(mut thing) = thing {
        if bites == 0 { return (slice_offset(code, slice), Some(thing), 0) }
        if let Some(Tok::VarAv2(name)) = slice.first() {
            slice = &slice[1..];
            let (len, t, b) = dbg!(thing_bites(slice, bites)); slice = &slice[len..]; bites = b;
            thing = Tg::Fe(Fe::Aav2 {
                f: Box::new(thing),
                v: name.clone(),
                g: Box::new(t.unwrap_or(Tg::Ve(Ve::Num(f64::NAN))))});
        }
        dbg!((slice_offset(code, slice), Some(thing), bites))
    } else {(0, None, 0)}
}

// takes from iterator to make a strand. if it's only one element, it's just the one value. 
// if it's not a strand, returns None
fn strand(iter: &mut std::iter::Peekable<std::vec::IntoIter<Tg>>) -> Option<Ve> {
    let mut evs = Vec::new();
    while let Some(Tg::Ve(v)) = iter.peek() {
        evs.push(v.clone()); iter.next();
    }
    (!evs.is_empty()).then(|| if evs.len() == 1 { evs[0].clone() } else { Ve::Snd(evs) })
}

fn things_to_expr(things: Vec<Tg>) -> Option<Ve> {
    let mut iter = things.into_iter().peekable();
    Some(if let Some(start) = strand(&mut iter) {
        // Function application
        let mut value = start;
        while let Some(Tg::Fe(ef)) = iter.next() {
            value = if let Some(ev) = strand(&mut iter) {  // dyad
                Ve::Afn2 {a: Box::new(value), f: ef, b: Box::new(ev)}
            } else {  // monad
                Ve::Afn1 {a: Box::new(value), f: ef}
            }
        }
        value
    } else { match iter.next() {
        Some(Tg::Fe(ef)) => {
            // Train
            let mut value = if let Some(b) = strand(&mut iter) {
                Fe::Bind {f: Box::new(ef), b: Box::new(b)}
            } else { ef };
            while let Some(Tg::Fe(ef)) = iter.next() {
                value = if let Some(b) = strand(&mut iter) {  // dyad
                    Fe::Trn2 {a: Box::new(value), f: Box::new(ef), b: Box::new(b)}
                } else {  // monad
                    Fe::Trn1 {a: Box::new(value), f: Box::new(ef)}
                }
            }
            Ve::Nom(value)
        },
        Some(Tg::Ve(_)) => unreachable!(),
        None => return None
    }})
}

pub fn expression(code: &[Tok], limit: usize) -> (usize, Option<Ve>) {
    let mut slice = code;
    // makes a list of things before processing
    let mut things = Vec::new();
    let mut last_was_stmt = false;
    while things.len() < limit {
        let (len, t) = thing(slice);
        if let Some(thing) = t {
            if matches!(thing, Tg::Ve(_)) && last_was_stmt { break }
            things.push(thing);
        } else { break }
        last_was_stmt = matches!(slice.first(), Some(Tok::Stmt(_) | Tok::VarSet(_)));
        slice = &slice[len..];
    }
    if limit != usize::MAX { while things.len() < limit {
        things.push(things.last().cloned().unwrap_or(Tg::Ve(Ve::Num(f64::NAN))))
    }}
    let expr = things_to_expr(things);
    (usize::from(expr.is_some()) * slice_offset(code, slice), expr)
}

pub fn expression_bites(code: &[Tok], mut bites: usize) -> (usize, Option<Ve>) {
    let mut slice = code;
    // makes a list of things before processing
    let mut things = Vec::new();
    let mut last_was_stmt = false;
    loop {
        let (len, t, b) = thing_bites(slice, bites);
        if let Some(thing) = t {
            if matches!(thing, Tg::Ve(_)) && last_was_stmt { break }
            things.push(thing);
        } else { break }
        last_was_stmt = matches!(slice.first(), Some(Tok::Stmt(_) | Tok::VarSet(_)));
        slice = &slice[len..]; bites = b;
    }

    let expr = things_to_expr(things);
    (usize::from(expr.is_some()) * slice_offset(code, slice), expr)
}

fn block(code: &[Tok]) -> (usize, Vec<Ve>) {
    let mut slice = code;
    let mut exps = Vec::new();
    loop {
        while let Some(Tok::Just(b!('·'))) = slice.first() { slice = &slice[1..]; }
        let (len, ev) = expression(slice, usize::MAX); slice = &slice[len..];
        if let Some(ev) = ev {
            exps.push(ev);
        } else { break }
    }
    while let Some(Tok::Just(b!('·'))) = slice.first() { slice = &slice[1..]; }
    (slice_offset(code, slice), exps)
}

pub fn parse(code: &[Tok]) -> Vec<Ve> {
    let (len, exps) = block(code);
    if code.len() > len {
        println!("unexpected token {:?}", code[len]);
    }
    exps
}