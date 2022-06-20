#![feature(let_else)]
#![warn(clippy::cast_lossless)]
#![warn(clippy::map_unwrap_or)]
#![warn(clippy::semicolon_if_nothing_returned)]

#[macro_use] mod codepage;
mod token; mod parse; mod run; mod test;

pub type Bstr = smallvec::SmallVec<[u8; 16]>; // length will be the same as a Vec in 64bit archs

fn main() {
    //println!("sizeof(Val) = {}", std::mem::size_of::<run::Val>());
    if let Some(path) = std::env::args().nth(1) {
        let mut state = run::Env::new();
        state.include_stdlib();
        println!("{}",
            state.include_file(&mut std::fs::File::open(path).unwrap()).unwrap().display_string());
    } else {
        println!("welcome to vemf repl. enjoy your stay");
        let mut state = run::Env::new();
        state.include_stdlib();
        loop {
            print!("    ");
            let _ = std::io::Write::flush(&mut std::io::stdout());
            let mut code = String::new();
            std::io::stdin().read_line(&mut code).expect("error while reading from stdin");
            if code.trim_start().starts_with(')') {
                if let Some((l, r)) = code[1..].split_once(' ') {
                    let val = state.include_string(r);
                    state.locals.insert(Bstr::from(&b"__"[..]), val);
                    state.include_string(&format!("__ⁿ({})☻", l));
                } else {
                    state.include_string(&format!("__ⁿ({})☻", &code[1..]));
                }
                continue;
            }
            let val = state.include_string(&code);
            if !val.is_nan() { println!("{}", val); }
            state.locals.insert(Bstr::from(&b"__"[..]), val);
        }
    }
}
