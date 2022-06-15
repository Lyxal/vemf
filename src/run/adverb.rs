use super::{Val, Env, NAN};
use std::{rc::Rc, ops::Range};

#[derive(Copy, Clone, Debug)]
pub enum AvT {
    Swap, Const, Monadic,
    Each, EachLeft, Conform, Extend,
    Scan, ScanPairs, Reduce, Stencil, Valences,
    Overleft, Overright, Over,
    Until, UntilScan, Power, PowerScan,
}

impl AvT {
pub fn call(&self, env: &mut Env, a: Val, b: Option<Val>, f: Option<&Rc<Val>>, g: &Rc<Val>) -> Val {
    //let ba = b.unwrap_or(a);
    let fg = f.unwrap_or(g);
    match self {
        Self::Swap =>      g.dyad_r(env, b.unwrap_or_else(|| a.clone()), a),
        Self::Const =>     (**g).clone(),
        Self::Monadic =>   g.monad_r(env, a),
        Self::Each =>      each(env, a, b, g),
        Self::EachLeft =>  each_left(env, a, b, g),
        Self::Conform =>   conform(env, a, b, g),
        Self::Extend =>    extend(env, a, b, g),
        Self::Scan =>      scan(env, a, b, g),
        Self::ScanPairs => scan_pairs(env, a, b, g),
        Self::Reduce =>    reduce(env, a, b, g),
        Self::Stencil =>   stencil(env, a, b, fg, g),
        Self::Valences =>  (if b.is_none() {fg} else {g}).call_r(env, a, b),
        Self::Over =>      {
            let l = fg.monad_r(env, a.clone());
            let r = fg.monad_r(env, b.unwrap_or(a));
            g.dyad_r(env, l, r) },
        Self::Overleft =>  {
            let x = fg.monad_r(env, a.clone());
            g.dyad_r(env, x, b.unwrap_or(a)) },
        Self::Overright => {
            let x = fg.monad_r(env, b.unwrap_or_else(|| a.clone()));
            g.dyad_r(env, a, x) },
        Self::Until =>     until(env, a, b, fg, g),
        Self::UntilScan => until_scan(env, a, b, fg, g),
        Self::Power =>     power(env, a, b, fg, g),
        Self::PowerScan => power_scan(env, a, b, fg, g),
    }
}
}

pub fn each(env: &mut Env, a: Val, b: Option<Val>, g: &Rc<Val>) -> Val {
    if a.is_scalar() && b.iter().all(|x| x.is_scalar()) {
        g.call_r(env, a, b)
    } else if let Some(b) = b {
        let mut collect_range = |x: Range<usize>| x.map(|n| {
            let l = a.index(env, n); let r = b.index(env, n); g.dyad_r(env, l, r)
        }).collect();
        if !a.is_finite() && !b.is_finite() { 
            Val::Fork { a: a.clone().rc(), 
                        f: Val::Av(AvT::Conform, None, Rc::clone(g)).rc(), 
                        b: b.clone().rc() }
        } else if a.is_scalar() { Val::lis_fill(collect_range(0..b.len()), b.fill())
        } else if b.is_scalar() { Val::lis_fill(collect_range(0..a.len()), a.fill())
        } else { Val::lis_fill(collect_range(0..usize::min(a.len(), b.len())), a.fill())
        }
    } else { each_left(env, a, None, g) }
}

pub fn each_left(env: &mut Env, a: Val, b: Option<Val>, g: &Rc<Val>) -> Val {
    if a.is_scalar() {
        g.call_r(env, a, b)
    } else if !a.is_finite() { if let Some(b) = b {
        Val::Fork { a: a.rc(), f: Rc::clone(g), b: b.rc() }
    } else {
        Val::Trn2 { a: a.rc(), f: Rc::clone(g) }
    }} else { Val::lis_fill(
        (0..a.len()).map(|n| { let l = a.index(env, n); g.call_r(env, l, b.clone()) }).collect(),
        a.fill(),
    )}
}


pub fn conform(env: &mut Env, a: Val, b: Option<Val>, g: &Rc<Val>) -> Val {
    if a.is_scalar() && b.iter().all(|x| x.is_scalar()) {
        g.call_r(env, a, b)
    } else if let Some(b) = b {
        let mut collect_range = |x: Range<usize>| x.map(|n| {
            let l = a.index(env, n); let r = b.index(env, n); conform(env, l, Some(r), g)
        }).collect();
        if !a.is_finite() && !b.is_finite() { 
            Val::Fork { a: a.clone().rc(), 
                        f: Val::Av(AvT::Conform, None, Rc::clone(g)).rc(), 
                        b: b.clone().rc() }
        } else if a.is_scalar() { Val::lis_fill(collect_range(0..b.len()), b.fill())
        } else if b.is_scalar() { Val::lis_fill(collect_range(0..a.len()), a.fill())
        } else { Val::lis_fill(collect_range(0..usize::min(a.len(), b.len())), a.fill())
        }
    } else { extend(env, a, None, g) }
}

pub fn extend(env: &mut Env, a: Val, b: Option<Val>, g: &Rc<Val>) -> Val {
    if a.is_scalar() {
        g.call_r(env, a, b)
    } else if !a.is_finite() { if let Some(b) = b {
        Val::Fork { a: a.rc(), f: Val::Av(AvT::Conform, None, Rc::clone(g)).rc(), b: b.rc() }
    } else {
        Val::Trn2 { a: a.rc(), f: Val::Av(AvT::Conform, None, Rc::clone(g)).rc() }
    }} else { Val::lis_fill(
        (0..a.len()).map(|n| { let l = a.index(env, n); extend(env, l, b.clone(), g) }).collect(),
        a.fill(),
    )}
}


pub fn scan(env: &mut Env, a: Val, b: Option<Val>, g: &Rc<Val>) -> Val {
    if !a.is_finite() { return NAN; }
    let mut values = Vec::with_capacity(a.len());
    let mut iter = a.into_iterf();
    let Some(start) = iter.next() else { return b.unwrap_or(NAN) };
    let mut val = match b {
        Some(b) => g.dyad_r(env, b, start),
        None => start,
    };
    values.push(val.clone());
    for i in iter {
        val = g.dyad_r(env, val, i);
        values.push(val.clone());
    }
    Val::lis(values)
}

pub fn reduce(env: &mut Env, a: Val, b: Option<Val>, g: &Rc<Val>) -> Val {
    if !a.is_finite() { return NAN; }
    let mut iter = a.into_iterf();
    let Some(start) = iter.next() else { return b.unwrap_or(NAN) };
    let mut val = match b {
        Some(b) => g.dyad_r(env, b, start),
        None => start,
    };
    for i in iter {
        val = g.dyad_r(env, val, i);
    }
    val
}

pub fn until_scan(env: &mut Env, a: Val, b: Option<Val>, f: &Rc<Val>, g: &Rc<Val>) -> Val {
    let mut values = vec![a.clone()];
    let mut val = a;
    loop {
        let tried = g.call_r(env, val.clone(), b.clone());
        if f.dyad_r(env, tried.clone(), val).as_bool() { break }
        values.push(tried.clone());
        val = tried;
    }
    Val::lis(values)
}


pub fn until(env: &mut Env, a: Val, b: Option<Val>, f: &Rc<Val>, g: &Rc<Val>) -> Val {
    let mut val = a;
    loop {
        let tried = g.call_r(env, val.clone(), b.clone());
        if f.dyad_r(env, tried.clone(), val.clone()).as_bool() { break }
        val = tried;
    }
    val
}

pub fn power_scan(env: &mut Env, a: Val, b: Option<Val>, f: &Rc<Val>, g: &Rc<Val>) -> Val {
    let num = f.call_r(env, a.clone(), b.clone()).try_int().map_or(0, |x| x.try_into().unwrap_or(0));
    let mut values = Vec::with_capacity(num);
    let mut val = a;
    for _ in 0..num {
        val = g.call_r(env, val, b.clone());
        values.push(val.clone());
    }
    Val::lis(values)
}


pub fn power(env: &mut Env, a: Val, b: Option<Val>, f: &Rc<Val>, g: &Rc<Val>) -> Val {
    let num = f.call_r(env, a.clone(), b.clone()).try_int().map_or(0, |x| x.try_into().unwrap_or(0));
    let mut val = a;
    for _ in 0..num {
        val = g.call_r(env, val, b.clone());
    }
    val
}

#[allow(clippy::needless_borrow)]
pub fn scan_pairs(env: &mut Env, a: Val, b: Option<Val>, g: &Rc<Val>) -> Val {
    if a.len() == 0 { return Val::lis_fill(Vec::new(), a.fill()); }
    let elems = a.iterf().collect::<Vec<_>>();
    let mut list = Vec::with_capacity(elems.len());
    let first = if let Some(b) = b { g.dyad_r(env, b, elems[0].clone()) } else { elems[0].clone() };
    list.push(first);
    for i in 1..elems.len() {
        list.push(g.dyad_r(env, elems[i-1].clone(), elems[i].clone()));
    }
    Val::lis(list)
}

pub fn stencil(env: &mut Env, a: Val, b: Option<Val>, f: &Rc<Val>, g: &Rc<Val>) -> Val {
    let Some(size) = f.call_r(env, a.clone(), b.clone()).try_int().map(|x| x as usize) else {
        // we could do something smart here like reshaping the output or using
        // multiple dimensions but uh
        return Val::lis(Vec::new());
    };
    if !a.is_finite() { return Val::lis(Vec::new()); }
    // 1234567 3╫◄ = (123)(234)(345)(456)(567) l-n+1
    (0..(a.len() + 1).saturating_sub(size)).map(|n| {
        g.call_r(env, a.iterf().skip(n).take(size).cloned().collect(), b.clone())
    }).collect()

}