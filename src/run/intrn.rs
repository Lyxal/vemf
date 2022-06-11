use std::rc::Rc;
use crate::{Bstr, b};
use super::{Val::{self, Num, Lis}, Env, NAN, adverb::AvT};
use smallvec::smallvec;

impl Val {

pub fn monad(&self, env: &mut Env, a: &Val) -> Val { 
    self.call(env, a, None)
}

pub fn dyad(&self, env: &mut Env, a: &Val, b: &Val) -> Val {
    self.call(env, a, Some(b))
}

pub fn call(&self, env: &mut Env, a: &Val, b: Option<&Val>) -> Val { 
    let ba = b.unwrap_or(a);
    match self {

        Val::LoadIntrinsics => {
            macro_rules! load { ($($name:ident,)*) => { $( {
                let mut name = Bstr::from(&b"in"[..]);
                name.extend(stringify!($name).to_ascii_lowercase().bytes());
                env.locals.insert(name, Val::$name)
            } );* }}
            load!(
                Add, Sub, Mul, Div, Mod, Pow, Log, Lt, Eq, Gt, Max, Min, Atanb,
                Abs, Neg, Ln, Exp, Sin, Asin, Cos, Acos, Tan, Atan, Sqrt, Round, Ceil, Floor, Isnan,
                Left, Right, Len, Shape, Index, Iota, Pair, Enlist, Ravel, Concat, Reverse, GetFill, SetFill,
                Print, Println, Exit, Format, Numfmt, Parse, Takeleft, Takeright, Dropleft, Dropright, Replist, Cycle, Match,
            );
            macro_rules! load_av {($($name:ident,)*) => { $( {
                let mut name = Bstr::from(&b"in"[..]);
                name.extend(stringify!($name).to_ascii_lowercase().bytes());
                env.locals.insert(name, Val::AvBuilder(AvT::$name))
            } );* }}
            load_av!(
                Swap, Const, Monadic,
                Each, EachLeft, Conform, Extend,
                Scan, ScanPairs, Reduce, Stencil, Valences,
                Overleft, Overright, Over,
                Until, UntilScan, Power, PowerScan,
            ); Num(1.)
        }

        Num(_) | Lis { .. } => self.clone(),
        Val::FSet(name) => {
            env.locals.insert(name.clone(), a.clone());
            a.clone()
        },
        Val::Dfn { s, loc } => {
            let mut inner = Env { locals: (**loc).clone(), outer: Some(env) };
            inner.locals.insert(smallvec![b!('α')], a.clone());
            inner.locals.insert(smallvec![b!('β')], ba.clone());
            inner.locals.insert(smallvec![b!('ƒ')], self.clone());
            inner.eval_block(s)
        },

        Val::Bind { f: aa, b: bb } => aa.dyad(env, a, bb),
        Val::Trn2 { a: aa, f: ff }        => { let x = aa.call(env, a, b); ff.monad(env, &x) },
        Val::Trn3 { a: aa, f: ff, b: bb } => { let x = aa.call(env, a, b); ff.dyad(env, &x, bb) },
        Val::Fork { a: aa, f: ff, b: bb } => {
            let l = aa.call(env, a, b);
            let r = bb.call(env, a, b);
            ff.dyad(env, &l, &r)
        }

        Val::AvBuilder(t) => Val::Av(*t, b.map(|x| x.clone().rc()), a.clone().rc()),
        Val::Av(t, f, g) => t.call(env, a, b, f.as_ref(), g),
        Val::Add  => match (a, b) {(Num(a), Some(Num(b))) => Num(a + b), _ => NAN },
        Val::Sub  => match (a, b) {(Num(a), Some(Num(b))) => Num(a - b), _ => NAN },
        Val::Mul  => match (a, b) {(Num(a), Some(Num(b))) => Num(a * b), _ => NAN },
        Val::Div  => match (a, b) {(Num(a), Some(Num(b))) => Num(a / b), _ => NAN },
        Val::Mod  => match (a, b) {(Num(a), Some(Num(b))) => Num(a.rem_euclid(*b)), _ => NAN },
        Val::Pow  => match (a, b) {(Num(a), Some(Num(b))) => Num(a.powf(*b)), _ => NAN },
        Val::Log  => match (a, b) {(Num(a), Some(Num(b))) => Num(a.log(*b)), _ => NAN },
        Val::Lt   => match (a, b) {(Num(a), Some(Num(b))) => Val::from_bool(a < b), _ => NAN },
        Val::Eq   => match (a, b) {(Num(a), Some(Num(b))) => Val::from_bool(a == b), _ => NAN },
        Val::Gt   => match (a, b) {(Num(a), Some(Num(b))) => Val::from_bool(a > b), _ => NAN },
        Val::Max  => match (a, b) {(Num(a), Some(Num(b))) => Num(a.max(*b)), _ => NAN },
        Val::Min  => match (a, b) {(Num(a), Some(Num(b))) => Num(a.min(*b)), _ => NAN },
        Val::Atanb=> match (a, b) {(Num(a), Some(Num(b))) => Num(a.atan2(*b)), _ => NAN },
        Val::Isnan=> match a { Num(a) => Val::from_bool(a.is_nan()), _ => NAN },
        Val::Abs  => match a { Num(a) => Num(a.abs()  ), _ => NAN },
        Val::Neg  => match a { Num(a) => Num(-a       ), _ => NAN },
        Val::Ln   => match a { Num(a) => Num(a.ln()   ), _ => NAN },
        Val::Exp  => match a { Num(a) => Num(a.exp()  ), _ => NAN },
        Val::Sin  => match a { Num(a) => Num(a.sin()  ), _ => NAN },
        Val::Asin => match a { Num(a) => Num(a.asin() ), _ => NAN },
        Val::Cos  => match a { Num(a) => Num(a.cos()  ), _ => NAN },
        Val::Acos => match a { Num(a) => Num(a.acos() ), _ => NAN },
        Val::Tan  => match a { Num(a) => Num(a.tan()  ), _ => NAN },
        Val::Atan => match a { Num(a) => Num(a.atan() ), _ => NAN },
        Val::Sqrt => match a { Num(a) => Num(a.sqrt() ), _ => NAN },
        Val::Round=> match a { Num(a) => Num(a.round()), _ => NAN },
        Val::Ceil => match a { Num(a) => Num(a.ceil() ), _ => NAN },
        Val::Floor=> match a { Num(a) => Num(a.floor()), _ => NAN },

        Val::Print   => { print  !("{}", a.display_string()); a.clone() },
        Val::Println => { println!("{}", a.display_string()); a.clone() },
        Val::Exit => match a {
            Num(n) => std::process::exit(*n as i32),
            _ => { eprintln!("{}", a.display_string()); std::process::exit(1); }
        }
        Val::Format => super::disp::format(a, &b.map_or_else(Vec::new, 
            |x| x.iterf().cloned().collect::<Vec<_>>())
        ).chars().map(|x| Num(x as u32 as f64)).collect(),
        Val::Numfmt => match a { // TODO support more bases and stuff
            Num(a) => format!("{}", a).chars().map(|x| Num(x as u32 as f64)).collect(),
            _ => NAN 
        }
        Val::Parse => a.display_string().parse::<f64>().map(Num).unwrap_or(NAN),
        Val::Takeleft => super::list::takeleft(env, a, ba),
        Val::Takeright => super::list::takeright(env, a, ba),
        Val::Dropleft => match ba {Num(n) => super::list::dropleft(a, *n), _ => NAN}
        Val::Dropright => match ba {Num(n) => super::list::dropright(a, *n), _ => NAN},

        Val::Left => a.clone(),
        Val::Right => ba.clone(),
        Val::Len => Num(a.lenf()),
        Val::Index => a.index_at_depth(env, ba),
        Val::Iota => match a {
            Lis{l, ..} => super::list::iota(
                Vec::new(), &l.iter().cloned().filter_map(|x| match x {
                    Num(b) => Some(b as isize), _ => None
                }).collect::<Vec<isize>>()),
            Num(n) => if *n == f64::INFINITY {Val::Left} else {
                super::list::iota_scalar(*n as isize)},
            _ => Val::Av(AvT::Const, None, NAN.rc()),
        }
        Val::Pair => Val::lis(vec![a.clone(), ba.clone()]),
        Val::Enlist => Val::lis(vec![a.clone()]),
        Val::Ravel => {
            let mut list = Vec::new();
            super::list::ravel(a, &mut list);
            Val::lis(list)
        },
        Val::Concat => super::list::concat(a, ba),
        Val::Reverse => super::list::reverse(a),
        Val::GetFill => match a {
            Lis {fill, ..} => (**fill).clone(),
            _ => NAN
        },
        Val::SetFill => match a {
            Lis {l, ..} => Lis {l: Rc::clone(l), fill: ba.clone().rc()},
            _ => a.clone(),
        },
        Val::Replist => if a.is_finite() {
            let num = match ba {Num(n) => *n as usize, _ => return NAN};
            (0..num).flat_map(|_| a.iterf().cloned()).collect()
        } else {a.clone()},
        Val::Cycle => if a.is_finite() {
            Val::DCycle(Rc::from(&a.iterf().cloned().collect::<Vec<_>>()[..]))
        } else {a.clone()},
        Val::DCycle(l) => match a {
            Num(a) => l[(*a as usize) % l.len()].clone(),
            _ => NAN,
        },
        Val::Match => Val::from_bool(a == ba),
        Val::Shape => Val::lis_fill(
            super::list::shape(a).iter().map(|x| Num(*x as f64)).collect(),
            Num(1.),
        )
    }
}


fn from_bool(b: bool) -> Val { Num(f64::from(u8::from(b))) }

pub fn is_nan(&self) -> bool { match self { Num(n) => n.is_nan(), _ => false }}

pub fn is_finite(&self) -> bool { matches!(self, Num(_) | Lis {..})}

pub fn is_scalar(&self) -> bool { matches!(self, Num(_))}

pub fn rc(self) -> Rc<Self> { Rc::new(self) }
}
