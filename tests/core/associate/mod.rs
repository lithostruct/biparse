mod proptest;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum SExp {
    Symbol(String),
    List(Vec<Self>),
}
impl SExp {
    fn to_text(&self) -> String {
        match self {
            SExp::Symbol(s) => s.clone(),
            SExp::List(sexps) => {
                sexps.iter().fold(String::from("("), |mut s, child| {
                    if &s == "(" {
                        //  First element
                        s += &child.to_text();
                    } else {
                        s += " ";
                        s += &child.to_text();
                    }
                    s
                }) + ")"
            }
        }
    }
}
