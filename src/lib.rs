#[macro_use]
extern crate nom;

use std::str;
use std::str::FromStr;
use std::fmt;
use std::fs::File;
use std::io::prelude::*;

// like Into trait but works from a ref avoiding consumption or expensive clone
pub trait IntoSexp {
    fn into_sexp(&self) -> Sexp;
}

#[derive(Debug, Clone)]
pub struct Sexp {
    pub element:Element,
    pub meta:Meta,
}

pub struct Compact<'a> {
    what:&'a Sexp,
}

pub fn compact<'a>(e:&'a Sexp) -> Compact<'a> {
    Compact { what:e }
}

#[derive(Debug, Clone)]
pub enum Element {
    String(String),
    List(Vec<Sexp>),
    Empty,
}

#[derive(Debug, Clone)]
pub struct Meta {
    indent:String,
    nl:usize,
    after_list:String,
}

pub type ERes<T> = Result<T, String>;

impl Sexp {

    pub fn new_empty() -> Sexp {
        Sexp { element:Element::Empty, meta:Meta { indent:String::from(""), nl:0, after_list:String::from(""), } }
    }

    pub fn new(element:Element, indent:String, nl:usize, after_list:String) -> Sexp {
        Sexp { element:element, meta:Meta { indent:indent, nl:nl, after_list:after_list, }, }
    }

    pub fn from<T:IntoSexp>(t:&T) -> Sexp {
        t.into_sexp()
    }
    
    pub fn list(&self) -> ERes<&Vec<Sexp> > {
        match self.element {
            Element::List(ref v) => Ok(v),
            _ => Err(format!("not a list: {}", self))
        }
    }
    
    pub fn string(&self) -> ERes<&String> {
        match self.element {
            Element::String(ref s) => Ok(s),
            _ => Err(format!("not a string: {}", self))
        }
    }

    pub fn f(&self) -> ERes<f64> {
        let s = try!(self.string());
        match f64::from_str(&s) {
            Ok(f) => Ok(f),
            _ => Err(format!("Error parsing float"))
        }
    }

    pub fn i(&self) -> ERes<i64> {
        let s = try!(self.string());
        match i64::from_str(&s) {
            Ok(f) => Ok(f),
            _ => Err(format!("Error parsing int"))
        }
    }
    
    pub fn list_name(&self) -> ERes<&String> {
        let l = try!(self.list());
        let l = &l[..];
        let a = try!(l[0].string());
        Ok(a)
    }

    pub fn slice_atom(&self, s:&str) -> ERes<&[Sexp]> {
        let v = try!(self.list());
        let v2 =&v[..];
        let st = try!(v2[0].string());
        if st != s {
            return Err(format!("list doesn't start with {}, but with {}", s, st))
        };
        Ok(&v[1..])
    }
    pub fn slice_atom_num(&self, s:&str, num:usize) -> ERes<&[Sexp]> {
        let v = try!(self.list());
        let v2 =&v[..];
        let st = try!(v2[0].string());
        if st != s {
            return Err(format!("list doesn't start with {}, but with {}", s, st))
        };
        let x = &v[1..];
        if x.len() != num {
            return Err(format!("list ({}) doesn't have {} elements", s, num))
        }
        Ok(x)      
    }
}

impl fmt::Display for Sexp {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        try!(write!(f, "{}", self.meta.indent));
        try!(match self.element {
            Element::String(ref s) => {
                if s.contains("(") || s.contains(" ") {
                    write!(f,"\"{}\"", s)
                } else {
                    write!(f,"{}", s)
                }
            },
            Element::List(ref v) => {
                try!(write!(f, "("));
                let mut prev_nl = 0;
                for (i, x) in v.iter().enumerate() {
                    let s = if i == 0 || x.meta.indent.len() > 0 || prev_nl > 0 { "" } else { " " };
                    try!(write!(f, "{}{}", s, x));
                    prev_nl = x.meta.nl;
                }
                write!(f, "{})", self.meta.after_list)
            },
            Element::Empty => Ok(())
        });
        for _ in 0..self.meta.nl {
            try!(writeln!(f,""));
        }
        Ok(())
    }
}

impl<'a> fmt::Display for Compact<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self.what.element {
            Element::String(ref s) => {
                if s.contains("(") || s.contains(" ") {
                    write!(f,"\"{}\"", s)
                } else {
                    write!(f,"{}", s)
                }
            },
            Element::List(ref v) => {
                try!(write!(f, "("));
                for (i, x) in v.iter().enumerate() {
                    let s = if i == 0 { "" } else { " " };
                    try!(write!(f, "{}{}", s, compact(x)));
                }
                write!(f, ")")
            },
            Element::Empty => Ok(())
        }
    }
}

pub fn display_string(s:&String) -> String {
    if s.contains("(") || s.contains(" ") || s.len() == 0 {
        format!("\"{}\"", s)
    } else {
        s.clone()
    }
}

pub fn parse_str(sexp: &str) -> Result<Sexp, String> {
    if sexp.len() == 0 {
        return Ok(Sexp::new_empty())
    }
    match parse_sexp(&sexp.as_bytes()[..]) {
        nom::IResult::Done(_, c) => Ok(c),
        nom::IResult::Error(err) => {
            match err {
                nom::Err::Position(kind,p) => 
                    Err(format!("parse error: {:?} |{}|", kind, str::from_utf8(p).unwrap())),
                _ => Err(format!("parse error"))
            }
        },
        nom::IResult::Incomplete(x) => Err(format!("incomplete: {:?}", x)),
    }
}

fn read_file(name: &str) -> Result<String, std::io::Error> {
    let mut f = try!(File::open(name));
    let mut s = String::new();
    try!(f.read_to_string(&mut s));
    Ok(s)
}

pub fn parse_file(name: &str) -> ERes<Sexp> {
    let s = try!(match read_file(name) {
        Ok(s) => Ok(s),
        Err(x) => Err(format!("{:?}", x))
    }); 
    parse_str(&s[..])
}

named!(parse_qstring<String>,
       map_res!(
           map_res!(
               delimited!(char!('\"'), is_not!("\""), char!('\"')),
               str::from_utf8),
           FromStr::from_str)
       );

named!(parse_bare_string<String>,
       map_res!(
           map_res!(
               is_not!(b")( \r\n"),
               str::from_utf8),
           FromStr::from_str)
       );

named!(parse_string<(Element,Option<&[u8]>) >,
       map!(alt!(parse_qstring | parse_bare_string), |x| (Element::String(x), None))
       );

named!(parse_list<(Element,Option<&[u8]>) >,
       chain!(
           char!('(') ~
               v: many0!(parse_sexp) ~
               after_list: opt!(nom::multispace) ~ // sometimes there is space after a closing bracket, this would not be caught by parse_sexp
               char!(')'),
           || (Element::List(v), after_list) )
       );

// TODO: consider lines with just spaces and a nl as also nl
named!(line_ending<usize>,
       chain!(
           opt!(nom::space) ~
               c: opt!(is_a!(b"\r\n"))
               , || match c { None => 0, Some(ref x) => x.len(), }
               )
       );

named!(parse_sexp<Sexp>,
           chain!(
               indent: opt!(nom::space) ~
                   s_after_list: alt!(parse_list | parse_string) ~
                   nl: line_ending
                   ,
               || {
                   let s = s_after_list.0.clone();
                   let after_list = s_after_list.1;
                   let indent = match indent {
                       None => String::new(),
                       Some(x) => String::from(str::from_utf8(x).unwrap()),
                   };
                   let after_list = match after_list {
                       None => String::new(),
                       Some(x) => String::from(str::from_utf8(x).unwrap()),
                   };
                   Sexp::new(s, indent, nl, after_list)
               })
       );


// internal tests
#[test]
fn test_qstring1() {
    let x = parse_string(&b"\"hello world\""[..]);
    match x {
        nom::IResult::Done(_,y) => {
            let (e, _) = y;
            match e {
                Element::String(f) => assert_eq!(String::from("hello world"), f),
                _ => panic!("not string"),
            }
        },
        _ => panic!("parser not done"),
    }
}

/*
#[test]
#[should_panic(expected="assertion failed: `(left == right)` (left: `Incomplete(Size(1))`, right: `Done([], \"hello\")`)")]
fn test_qstring2() {
    parse_string(&b"\"hello"[..]);
}
*/

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(dead_code)]
    fn check_parse_res(s: &str, o:&str) {
        let e = parse_str(s).unwrap();
        let t = format!("{}", e);
        assert_eq!(o, t)
    }
    #[allow(dead_code)]
    fn check_parse(s: &str) {
        let e = parse_str(s).unwrap();
        let t = format!("{}", e);
        assert_eq!(s, t)
    }

    #[allow(dead_code)]
    fn parse_fail(s: &str) {
        parse_str(s).unwrap();
    }
    #[allow(dead_code)]
    fn check_compact(s: &str, o:&str) {
        let e = parse_str(s).unwrap();
        let t = format!("{}", compact(&e));
        assert_eq!(o, t)
    }
    

    #[test]
    fn test_empty() { check_parse("") }
    
    #[test]
    fn test_empty_qstring() { check_parse("(hello \"\")") }

    #[test]
    fn test_minimal() { check_parse("()") }

    #[test]
    fn test_string() { check_parse("hello") }

    #[test]
    fn test_qstring_a() { check_parse_res("\"hello\"", "hello") }
    
    #[test]
    fn test_qstring_a2() { check_parse("\"hello world\"") }
    
    #[test]
    fn test_qstring_a3() { check_parse("\"hello(world)\"") }

    #[test]
    fn test_number() { check_parse("1.3") }
    
    #[test]
    fn test_float_vs_int() { check_parse("2.0") }

    #[test]
    fn test_double() { check_parse("(())") }

    #[test]
    fn test_br_string() { check_parse("(world)") }

    #[test]
    fn test_br_qstring() { check_parse_res("(\"world\")", "(world)") }

    #[test]
    fn test_br_int() { check_parse("(42)") }

    #[test]
    fn test_br_float() { check_parse("(12.7)") }
    
    #[test]
    fn test_br_qbrstring() { check_parse("(\"(()\")") }
    
    #[test]
    fn test_number_string() { check_parse("567A_WZ") }
    
    #[test]
    #[should_panic(expected="called `Result::unwrap()` on an `Err` value: \"incomplete: Size(2)\"")]
    fn test_invalid1() { parse_fail("(") }

    #[test]
    #[should_panic(expected="called `Result::unwrap()` on an `Err` value: \"parse error: Alt |)|\"")]
    fn test_invalid2() { parse_fail(")") }

    #[test]
    #[should_panic(expected="incomplete: Size")]
    fn test_invalid3() { parse_fail("\"hello") }

    #[test]
    fn test_complex() { check_parse("(module SWITCH_3W_SIDE_MMP221-R (layer F.Cu) (descr \"\") (pad 1 thru_hole rect (size 1.2 1.2) (at -2.5 -1.6 0) (layers *.Cu *.Mask) (drill 0.8)) (pad 2 thru_hole rect (size 1.2 1.2) (at 0.0 -1.6 0) (layers *.Cu *.Mask) (drill 0.8)) (pad 3 thru_hole rect (size 1.2 1.2) (at 2.5 -1.6 0) (layers *.Cu *.Mask) (drill 0.8)) (pad 5 thru_hole rect (size 1.2 1.2) (at 0.0 1.6 0) (layers *.Cu *.Mask) (drill 0.8)) (pad 6 thru_hole rect (size 1.2 1.2) (at -2.5 1.6 0) (layers *.Cu *.Mask) (drill 0.8)) (pad 4 thru_hole rect (size 1.2 1.2) (at 2.5 1.6 0) (layers *.Cu *.Mask) (drill 0.8)) (fp_line (start -4.5 -1.75) (end 4.5 -1.75) (layer F.SilkS) (width 0.127)) (fp_line (start 4.5 -1.75) (end 4.5 1.75) (layer F.SilkS) (width 0.127)) (fp_line (start 4.5 1.75) (end -4.5 1.75) (layer F.SilkS) (width 0.127)) (fp_line (start -4.5 1.75) (end -4.5 -1.75) (layer F.SilkS) (width 0.127)))") }

    #[test]
    fn test_multiline() {
        check_parse("\
(hello \"test it\"
    (foo bar)
    (mars venus)
)
")
    }

    #[test]
    fn test_multiline2() {
        check_parse("\
(hello world
  mad
    (world)
  not)")
    }

    #[test]
    fn test_multiline_two_empty() {
        check_parse("\
(hello

world)")
    }

    #[test]
    fn test_compact1() {
        check_compact("( hello
world \"foo
  bar\" 
     (baz)
)", "(hello world \"foo
  bar\" (baz))")
    }

    #[test]
    fn test_fail_pcb() {
        check_parse("\
(kicad_pcb (version 4) (host pcbnew \"(2015-05-31 BZR 5692)-product\")
  (general
  )
)")
    }
}


