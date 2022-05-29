use super::{Val::{self, Num, Lis}, Env, NAN};
use std::rc::Rc;

pub fn each(env: &mut Env, a: &Val, b: Option<&Val>, g: &Rc<Val>) -> Val {
    if let Some(b) = b {
        let len = match (a.is_finite(), b.is_finite()) {
            (true, true) => usize::max(a.len(), b.len()),
            (false, true) => b.len(),
            (true, false) => a.len(),
            (false, false) => return Val::Trn3 {
                a: Rc::new(a.clone()), f: Rc::clone(g), b: Rc::new(b.clone())
            }
        };
        let mut values = Vec::with_capacity(len);
        for n in 0..len {
            let l = a.index(env, n); let r = b.index(env, n);
            values.push(g.dyad(env, &l, &r));
        }
        let fill = match (a, b) {
            (Lis { fill: fa, .. }, Lis { fill: fb, .. }) => g.dyad(env, fa, fb),
            _ => NAN,
        };
        Lis{l: Rc::new(values), fill: Rc::new(fill)}
    } else {
        if !a.is_finite() { return Val::Trn2 {
            a: Rc::new(a.clone()), f: Rc::clone(g)
        }}
        let mut values = Vec::with_capacity(a.len());
        for n in 0..a.len() {
            let x = a.index(env, n);
            values.push(g.monad(env, &x));
        }
        let fill = match a { Lis { fill: f, .. } => f.clone(), _ => Rc::new(NAN) };
        Lis{l: Rc::new(values), fill}
    }
}

pub fn scal(env: &mut Env, a: &Val, b: Option<&Val>, g: &Rc<Val>) -> Val {
    if let Some(b) = b {
        match (a, b) {
            (Num(_), Num(_)) => g.dyad(env, a, b), // 1+2
            (Num(_)|Lis{..}, Num(_)|Lis{..}) => Lis {  // 123+4, 123+456
                l: Rc::new((0..usize::max(a.len(), b.len())).map(|n| {
                    let l = a.index(env, n); let r = b.index(env, n); scal(env, &l, Some(&r), g)
                }).collect()),
                fill: Rc::new(scal(env, &a.fill(), Some(&b.fill()), g))
            },
            (Lis{..}, _) => Lis {  // 123+Σ
                l: Rc::new((0..a.len()).map(|n| {
                    let l = a.index(env, n); let r = b.index(env, n); scal(env, &l, Some(&r), g)
                }).collect()),
                fill: Rc::new(a.fill()),
            },
            (_, Lis{..}) => Lis {  // Σ+123
                l: Rc::new((0..b.len()).map(|n| {
                    let l = a.index(env, n); let r = b.index(env, n); scal(env, &l, Some(&r), g)
                }).collect()),
                fill: Rc::new(b.fill()),
            },
            (_, _) => Val::Fork {  // 1+Σ | Σ+1 | Σ+Ω
                a: Rc::new(a.clone()), f: Rc::new(Val::DScalar(Rc::clone(g))), b: Rc::new(b.clone())
            },
        }
    } else {
        match a {
            Num(_) => g.monad(env, a),
            Lis{..} => Lis {
                l: Rc::new((0..a.len()).map(|n| {
                    let l = a.index(env, n); scal(env, &l, None, g)
                }).collect()),
                fill: Rc::new(a.fill()),
            },
            _ => Val::Trn2 {
                a: Rc::new(a.clone()), f: Rc::new(Val::DScalar(Rc::clone(g)))
            }
        }
    }
}


pub fn scan(env: &mut Env, a: &Val, b: Option<&Val>, g: &Rc<Val>) -> Val {
    if a.len() == 0 { return b.cloned().unwrap_or(NAN) }
    let Some(mut iter) = a.iterf() else {return NAN};
    let mut values = Vec::with_capacity(a.len());
    let start = iter.next().unwrap();
    let mut val = match b {
        Some(b) => g.dyad(env, b, start),
        None => start.clone(),
    };
    values.push(val.clone());
    for i in iter {
        val = g.dyad(env, &val, i);
        values.push(val.clone());
    }
    values.into_iter().collect()
}

pub fn reduce(env: &mut Env, a: &Val, b: Option<&Val>, g: &Rc<Val>) -> Val {
    if a.len() == 0 { return b.cloned().unwrap_or(NAN) }
    let Some(mut iter) = a.iterf() else {return NAN};
    let start = iter.next().unwrap();
    let mut val = match b {
        Some(b) => g.dyad(env, b, start),
        None => start.clone(),
    };
    for i in iter {
        val = g.dyad(env, &val, i);
    }
    val
}

pub fn until_scan(env: &mut Env, a: &Val, b: Option<&Val>, f: &Rc<Val>, g: &Rc<Val>) -> Val {
    let mut values = vec![a.clone()];
    let mut val = a.clone();
    loop {
        let tried = g.call(env, &val, b);
        if matches!( f.dyad(env, &tried, &val), Num(n) if n != 0. || n.is_nan()) { break }
        values.push(tried.clone());
        val = tried;
    }
    values.into_iter().collect()
}


pub fn until(env: &mut Env, a: &Val, b: Option<&Val>, f: &Rc<Val>, g: &Rc<Val>) -> Val {
    let mut val = a.clone();
    loop {
        let tried = g.call(env, &val, b);
        if matches!( f.dyad(env, &tried, &val), Num(n) if n != 0. || n.is_nan()) { break }
        val = tried;
    }
    val
}

pub fn power_scan(env: &mut Env, a: &Val, b: Option<&Val>, f: &Rc<Val>, g: &Rc<Val>) -> Val {
    let num = match f.call(env, a, b) {
        Num(n) if n > 0. && !n.is_nan() => n as usize,
        _ => 0,
    };
    let mut values = Vec::with_capacity(num);
    let mut val = a.clone();
    for _ in 0..num {
        val = g.call(env, &val, b);
        values.push(val.clone());
    }
    values.into_iter().collect()
}


pub fn power(env: &mut Env, a: &Val, b: Option<&Val>, f: &Rc<Val>, g: &Rc<Val>) -> Val {
    let num = match f.call(env, a, b) {
        Num(n) if n > 0. && !n.is_nan() => n as usize,
        _ => 0,
    };
    let mut val = a.clone();
    for _ in 0..num {
        val = g.call(env, &val, b);
    }
    val
}