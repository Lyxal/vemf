use std::rc::Rc;
use crate::{Bstr, b, or_nan};
use super::{Val::{self, Lis, Num, Int}, Env, NAN, adverb::AvT, c64, val::complexcmp, list};
use smallvec::smallvec;

impl Val {

pub fn monad(&self, env: &mut Env, a: Val) -> Val { 
    self.call(env, a, None)
}

pub fn dyad(&self, env: &mut Env, a: Val, b: Val) -> Val {
    self.call(env, a, Some(b))
}

pub fn call(&self, env: &mut Env, a: Val, b: Option<Val>) -> Val {
    macro_rules! ba {()=>{  b.unwrap_or(NAN)  }}
    match self {

        Val::LoadIntrinsics => {
            macro_rules! load { ($($name:ident,)*) => { $( {
                let mut name = Bstr::from(&b"in"[..]);
                name.extend(stringify!($name).to_ascii_lowercase().bytes());
                env.set_local(name, Val::$name)
            } );* }}
            load!(
                Add, Sub, Mul, Div, DivE, Mod, Pow, Log, Lt, Eq, Gt, And, Or, Max, Min, Atanb, Approx, BAnd, BOr, BXor, Gamma,
                Gcd, Lcm, Binom, Get, Set, Call,
                Abs, Neg, Ln, Exp, Sin, Asin, Cos, Acos, Tan, Atan, Sqrt, Round, Ceil, Floor, Isnan, Sign, BNot, BRepr,
                Complex, Real, Imag, Conj, Arg, Cis,
                Left, Right, Len, Shape, Index, Iota, Pair, Enlist, Ravel, Concat, Reverse, GetFill, SetFill,
                Print, Println, Exit, Format, Numfmt, Parse, Out, In, ToUtf8, FromUtf8,
                Takeleft, Takeright, Dropleft, Dropright, Replist, Cycle, Match, Deal, Sample, Replicate,
                GradeUp, GradeDown, SortUp, SortDown, BinsUp, BinsDown, Encode, FromCp, ToCp, Group,
            );
            macro_rules! load_av {($($name:ident,)*) => { $( {
                let mut name = Bstr::from(&b"in"[..]);
                name.extend(stringify!($name).to_ascii_lowercase().bytes());
                env.set_local(name, Val::AvBuilder(AvT::$name))
            } );* }}
            load_av!(
                Swap, Const, Monadic,
                Each, EachLeft, Conform, Extend,
                Scan, ScanPairs, Reduce, Stencil, Valences,
                Overleft, Overright, Over, Forkleft, Forkright,
                Until, UntilScan, Power, PowerScan,
                Drill,
            ); Int(1)
        }
        Val::Err(x) => Val::Err(*x),

        Lis { .. } | Num(_) | Int(_) => self.clone(),
        Val::FSet(name) => {
            env.set_local(name.clone(), a.clone());
            b.unwrap_or(a)
        },
        Val::Set  => {
            let name = a.iterf().filter_map(|x| x.try_int().map(|x| x as u8)).collect::<Bstr>();
            let left = ba!();
            env.set_local(name, left.clone()); left
        },
        Val::Get  => {
            let name = a.iterf().filter_map(|x| x.try_int().map(|x| x as u8)).collect::<Bstr>();
            env.get_var(&name).unwrap_or(NAN)
        },
        Val::FCng(name) => {
            env.mutate_var(name, a).unwrap_or(NAN)
        }
        Val::Dfn { s, loc } => {
            env.stack.push((**loc).clone());
            env.set_local(smallvec![b!('Σ')], Int(1 + i64::from(b.is_some())));
            env.set_local(smallvec![b!('α')], a);
            env.set_local(smallvec![b!('β')], b.unwrap_or(NAN));
            env.set_local(smallvec![b!('ƒ')], self.clone());
            let val = env.eval_block(s);
            env.stack.pop();
            val
        },

        Val::Bind { f: aa, b: bb } => aa.dyad(env, a, (**bb).clone()),
        Val::Trn2 { a: aa, f: ff } => {
            let x = aa.call(env, a, b);
            ff.monad(env, x)
        },
        Val::Trn3 { a: aa, f: ff, b: bb } => {
            let x = aa.call(env, a, b);
            ff.dyad(env, x, (**bb).clone())
        },
        Val::Fork { a: aa, f: ff, b: bb } => {
            let l = aa.call(env, a.clone(), b.clone());
            let r = bb.call(env, a, b);
            ff.dyad(env, l, r)
        }

        Val::AvBuilder(t) => Val::Av(*t, b.map(|x| x.rc()), a.rc()),
        Val::Av(t, f, g) => t.call(env, a, b, f.as_ref(), g),
        Val::Add => match (a, ba!()) {
            (Int(a), Int(b)) => Int(a.saturating_add(b)),
            (a, ba) => Num(a.as_c() + ba.as_c()),
        }
        Val::Sub => match (a, ba!()) {
            (Int(a), Int(b)) => Int(a.saturating_sub(b)),
            (a, ba) => Num(a.as_c() - ba.as_c()),
        }
        Val::Mul => match (a, ba!()) {
            (Int(a), Int(b)) => Int(a.saturating_mul(b)),
            (a, ba) => Num(a.as_c() * ba.as_c()),
        }
        Val::Div => Num(a.as_c().fdiv(ba!().as_c())),
        Val::Mod => match (a, ba!()) {
            (Int(a), Int(b)) => Int(a.rem_euclid(b)),
            (a, ba) => {
                let (a, b) = (a.as_c(), ba.as_c());
                // this definition is probably not very useful for imaginary numbers
                let mut r = a % b;
                if r.re < 0.0 { r += b.re.abs(); }
                if r.im < 0.0 { r += b.im.abs(); }
                Num(r)
            },
        }
        Val::DivE => a.try_int().zip(ba!().try_int()).map_or(NAN, |(a, b)| if b == 0 {NAN} else {
            Int(a.div_euclid(b))
        }),
        Val::Pow => match (a, ba!()) {
            (Int(a), Int(b)) if b >= 0 => Int(a.saturating_pow(b as u32)),
            (a, ba) => Num(a.as_c().powc(ba.as_c())),
        },
        Val::Log => Num(a.as_c().log(ba!().as_c().norm())),
        Val::Lt => match (a, ba!()) {
            (Int(a), Int(b)) => Val::bool(a < b),
            (a, ba) => a.try_c().zip(ba.try_c()).map_or(NAN, |(a, b)| Val::bool(complexcmp(a, b).is_lt()))
        },
        Val::Eq => match (a, ba!()) {
            (Int(a), Int(b)) => Val::bool(a == b),
            (a, ba) => a.try_c().zip(ba.try_c()).map_or(NAN, |(a, b)| Val::bool(a == b))
        },
        Val::Gt => match (a, ba!()) {
            (Int(a), Int(b)) => Val::bool(a > b),
            (a, ba) => a.try_c().zip(ba.try_c()).map_or(NAN, |(a, b)| Val::bool(complexcmp(a, b).is_gt()))
        },
        Val::And => Val::bool(a.as_bool() && ba!().as_bool()),
        Val::Or =>  Val::bool(a.as_bool() || ba!().as_bool()),
        Val::Max => match (a, ba!()) {
            (Int(a), Int(b)) => Int(a.max(b)),
            (a, ba) => if complexcmp(a.as_c(), ba.as_c()).is_gt() {a} else {ba}
        },
        Val::Min => match (a, ba!()) {
            (Int(a), Int(b)) => Int(a.min(b)),
            (a, ba) => if complexcmp(a.as_c(), ba.as_c()).is_lt() {a} else {ba}
        },
        Val::Atanb=> {
            let (y, x) = (a.as_c(), ba!().as_c());
            Val::flt(f64::atan2(y.re + x.im, x.re - y.im))
        },
        Val::Approx=> Val::bool(Val::approx(&a, &ba!())),
        Val::Gcd => a.try_int().zip(ba!().try_int()).map_or(NAN, |(a, b)| Int(num_integer::gcd(a, b))),
        Val::Lcm => a.try_int().zip(ba!().try_int()).map_or(NAN, |(a, b)| Int(num_integer::lcm(a, b))),
        Val::Binom => a.try_int().zip(ba!().try_int()).map_or(NAN, |(a, b)| Int(num_integer::binomial(a, b))),
        Val::Isnan=> Val::bool(a.is_nan()),
        Val::BAnd => a.try_int().zip(ba!().try_int()).map_or(NAN, |(a, b)| Int(a & b)),
        Val::BOr  => a.try_int().zip(ba!().try_int()).map_or(NAN, |(a, b)| Int(a | b)),
        Val::BXor => a.try_int().zip(ba!().try_int()).map_or(NAN, |(a, b)| Int(a ^ b)),
        Val::BNot => a.try_int().map_or(NAN, |a| Int(!a)),
        Val::BRepr => a.try_int().map_or(NAN, |n| Val::lis_fill(
            n.to_be_bytes()
                .into_iter().rev()
                .flat_map(|x| (0..8).map(move |y| Val::bool(x & (1 << y) != 0)))
                .collect(),
            Int(0)
        )),
        Val::Abs  => match a { Int(a) => Int(a.abs()), Num(x) => Val::flt(x.norm()), _ => NAN },
        Val::Neg  => match a { Int(a) => Int(-a),      Num(a) => Num(-a), _ => NAN },
        Val::Ln   => a.try_c().map_or(NAN, |a| Num(a.ln()  )),
        Val::Exp  => a.try_c().map_or(NAN, |a| Num(a.exp() )),
        Val::Sin  => a.try_c().map_or(NAN, |a| Num(a.sin() )),
        Val::Asin => a.try_c().map_or(NAN, |a| Num(a.asin())),
        Val::Cos  => a.try_c().map_or(NAN, |a| Num(a.cos() )),
        Val::Acos => a.try_c().map_or(NAN, |a| Num(a.acos())),
        Val::Tan  => a.try_c().map_or(NAN, |a| Num(a.tan() )),
        Val::Atan => a.try_c().map_or(NAN, |a| Num(a.atan())),
        Val::Sqrt => a.try_c().map_or(NAN, |a| Num(a.sqrt())),
        Val::Gamma=> a.try_c().map_or(NAN, |a| Val::flt(libm::tgamma(a.re))),
        Val::Round=> match a { Int(a) => Int(a), Num(a) => Val::flt(a.re.round()), _ => NAN },
        Val::Ceil => match a { Int(a) => Int(a), Num(a) => Val::flt(a.re.ceil()) , _ => NAN },
        Val::Floor=> match a { Int(a) => Int(a), Num(a) => Val::flt(a.re.floor()), _ => NAN },
        Val::Sign => match a { Int(a) => Int(a.signum()), Num(a) => {
            if a == c64::new(0., 0.) { Int(0) } 
            else if a.im == 0. { Int(a.re.signum() as i64) }
            else { Num(c64::from_polar(1., a.arg())) }
        }, _ => NAN },

        Val::Complex=> a.try_c().zip(ba!().try_c()).map_or(NAN, |(a,b)| Num(c64::new(b.re, a.re))),
        Val::Cis  => a.try_c().zip(ba!().try_c()).map_or(NAN, |(a,b)| Num(c64::from_polar(b.re, a.re))),
        Val::Real => a.try_c().map_or(NAN, |a| Val::flt(a.re)),
        Val::Imag => a.try_c().map_or(NAN, |a| Val::flt(a.im)),
        Val::Conj => a.try_c().map_or(NAN, |a| Num(a.conj())),
        Val::Arg  => a.try_c().map_or(NAN, |a| Val::flt(a.arg())),

        Val::Print   => {
            if let Some(mut o) = env.interface.output(0) {
                let _= o.write(a.display_string().as_bytes());
            }
            b.unwrap_or(a)
        },
        Val::Println => {
            if let Some(mut o) = env.interface.output(0) {
                let _= o.write(a.display_string().as_bytes());
                let _= o.write(b"\n");
            }
            b.unwrap_or(a)
        },
        Val::Out => {
            let mut stm = or_nan!(
                ba!().try_int()
                .and_then(|x| usize::try_from(x).ok())
                .and_then(|n| env.interface.output(n))
            );
            let bytes = a.iterf()
                .flat_map(|x| x.try_int().map(|x| (x & 0xff) as u8))
                .collect::<Vec<_>>();
            stm.write(&bytes).map(|x| Int(x as i64)).unwrap_or(NAN)
        }
        Val::In => {
            let chars = or_nan!(a.try_int().and_then(|x| isize::try_from(x).ok()));
            let mut stm = or_nan!(
                ba!().try_int()
                .and_then(|x| usize::try_from(x).ok())
                .and_then(|n| env.interface.input(n)));
            let mut buf;
            let size = or_nan!(
                if chars < 0 { // read line
                    buf = Vec::with_capacity(128); stm.read_until(b'\n', &mut buf)
                } else if chars == isize::MAX { // don't allocate an infinite buffer
                    buf = Vec::with_capacity(128); stm.read_to_end(&mut buf)
                } else {
                    buf = vec![0; chars as usize]; stm.read(&mut buf)
                }.ok()
            );
            buf.into_iter().take(size).map(|i| Int(i64::from(i))).collect()
        }
        Val::FromUtf8 => String::from_utf8_lossy(
            &a.iterf()
            .flat_map(|x| x.try_int().map(|x| (x & 0xff) as u8))
            .collect::<Vec<_>>()
        ).chars().map(|x| Int(x as _)).collect(),
        Val::ToUtf8 => a.iterf()
            .flat_map(|x| x.try_int().map(|x| 
                x.try_into().ok().and_then(char::from_u32).unwrap_or('\u{FFFD}')))
            .collect::<String>()
            .bytes()
            .map(|x| Int(i64::from(x)))
            .collect(),
        Val::Exit => 
            if let Some(n) = a.try_int() {
                Val::Err(n as i32)
            } else {
                if let Some(mut o) = env.interface.output(1) {
                    let _= o.write(a.display_string().as_bytes());
                    let _= o.write(b"\n");
                }
                Val::Err(1)
            }
        Val::Format => a.format(&b.as_ref().map_or_else(Vec::new,
            |x| x.iterf().cloned().collect::<Vec<_>>())
        ).chars().map(|x| Int(x as i64)).collect(),
        Val::Numfmt => if !a.is_scalar() {NAN} else {
            format!("{a}").chars().map(|x| Int(x as i64)).collect() }
        Val::Parse => a.display_string().parse::<c64>().map(Num).unwrap_or(NAN),
        Val::Takeleft => list::reshape(env, a, ba!(), false),
        Val::Takeright => list::reshape(env, a, ba!(), true),
        Val::Dropleft =>  ba!().try_int().map_or(NAN, |b| list::dropleft(a, b)),
        Val::Dropright => ba!().try_int().map_or(NAN, |b| list::dropright(a, b)),

        Val::Left => a, Val::Right => b.unwrap_or(a),
        Val::Len => match a {
            Num(_) | Int(_) => Int(1),
            Lis { l, .. } => Int(l.len() as i64),
            _ => Val::flt(f64::INFINITY),
        },
        Val::Index => a.index_at_depth(env, ba!()),
        Val::Iota => match a {
            Lis{l, ..} => list::iota(
                Vec::new(), &l.iter().cloned().filter_map(|x| x.try_int()).collect::<Vec<i64>>()),
            Num(n) => if n.is_infinite() {Val::Left} else {list::iota_scalar(n.re as i64)},
            Int(n) => list::iota_scalar(n),
            _ => Val::Av(AvT::Const, None, NAN.rc()),
        }
        Val::Pair => Val::lis(vec![a, ba!()]),
        Val::Enlist => Val::lis(vec![a]),
        Val::Ravel => {
            let mut list = Vec::new(); list::ravel(a, &mut list); Val::lis(list)
        },
        Val::Concat => list::concat(a, ba!()),
        Val::Reverse => list::reverse(a),
        Val::GetFill => match a {
            Lis {fill, ..} => (*fill).clone(),
            _ => NAN
        },
        Val::SetFill => match a {
            Lis {l, ..} => Lis {l, fill: ba!().rc()},
            a => a,
        },
        Val::Replist => if !a.is_infinite() {
            (0..or_nan!(ba!().try_int())).flat_map(|_| a.iterf().cloned()).collect()
        } else {a},
        Val::Cycle => match a {
            Lis{l, ..} => Val::DCycle(l),
            Num(_) | Int(_) => Val::DCycle(Rc::new(vec![a])),
            _ => a
        },
        Val::DCycle(l) => a.try_int().map_or(NAN, |a| l[(a as usize) % l.len()].clone()),
        Val::Match => Val::bool(a == ba!()),
        Val::Shape => Val::lis_fill(
            list::shape(&a).iter().map(|x| Int(*x as i64)).collect(),
            Int(1),
        ),

        Val::Deal => a.try_int().zip(ba!().try_int()).map_or(NAN, |(a, b)| {
            use rand::distributions::{Distribution, Uniform, Standard};
            if a <= 0 { 
                Standard.sample_iter(rand::thread_rng()).take(b as _).map(Val::flt).collect()
            } else {
                Uniform::from(0..a as _).sample_iter(rand::thread_rng())
                    .take(b as usize)
                    .map(|x| Int(i64::from(x))).collect::<Val>()
            }
        }),
        Val::Sample => a.try_int().zip(ba!().try_int()).map_or(NAN, |(a, b)|
            rand::seq::index::sample(&mut rand::thread_rng(), a as _, usize::min(a as _, b as _))
                .iter()
                .map(|x| Int(x as i64))
                .collect::<Val>(),
        ),
        Val::Replicate => {
            let fill = a.fill();
            Val::lis_fill(list::replicate(env, a, ba!()), fill)
        }

        Val::GradeUp   => list::grade_up(a),
        Val::GradeDown => list::grade_down(a),
        Val::SortUp    => list::sort_up(a),
        Val::SortDown  => list::sort_down(a),
        Val::BinsUp    => list::bins_up(&a, &ba!()),
        Val::BinsDown  => list::bins_down(&a, &ba!()),
        Val::Group     => list::group(env, a, ba!()),
        Val::Encode    => super::val::encode(a, ba!()),
        Val::FromCp => {
            if a.is_nan() { return NAN; }
            a.try_int()
                .and_then(|x| u8::try_from(x).ok())
                .map_or(NAN, |x| Int(
                    if x == b'\n' {'\n'} else {crate::codepage::tochar(x)}
                as i64))
        }
        Val::ToCp => {
            if a.is_nan() { return NAN; }
            a.try_int()
                .and_then(|x| u32::try_from(x).ok())
                .and_then(char::from_u32)
                .and_then(crate::codepage::tobyte)
                .map_or(NAN, |x| Int(i64::from(x)))
        }
        Val::Call => {
            let b = ba!();
            if b.is_scalar() {
                a.monad(env, b)
            } else {
                let aa = b.index(env, 0);
                let bb = (b.len() > 1).then(|| b.index(env, 1));
                a.call(env, aa, bb)
            }
        }
    }
}


pub fn rc(self) -> Rc<Self> { Rc::new(self) }


}

