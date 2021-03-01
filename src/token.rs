use std::iter::Enumerate;
use std::ops::Fn;
use std::str::Chars;

#[derive(Debug)]
pub struct Token {
    pub kind: TokenKind,
    pos: usize,
    len: usize,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Literal {
    Str(String),
    Int(i64),
    Float(f64),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Comment, // # comment
    Newline,
    Sentence,
    Indent(usize),
    TabIndent(usize),
    Title(usize),
    Literal(Literal),
    AtString, // @sksat@mstdn.maud.io
    Func,     // @<hoge>(arg){block}
    FuncArgOpen,
    FuncArgClose,
    FuncArg,
    FuncBlock,
    Math,       // $y = f(x)$
    InlineCode, // `printf("hello");`
    Quote,      // > quote
    CodeBlock,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct Tokenizer<'a> {
    src: &'a str,
    pos: usize,
    last: TokenKind,
}

impl<'a> Iterator for Tokenizer<'a> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        //let mut token = None;
        let token = match self.last {
            TokenKind::Newline => self.get_top_token(),
            TokenKind::Func => self.get_func_arg_or_default(),
            TokenKind::FuncArgOpen | TokenKind::FuncArg => self.get_func_arg(),
            TokenKind::FuncArgClose | TokenKind::FuncBlock => self.get_func_block_or_default(),
            _ => self.get_token(),
        };
        //if self.state == TokenKind::Newline {
        //    token = self.get_top_token();
        //} else {
        //    token = self.get_token();
        //}

        if token.is_none() {
            return None;
        }
        let t = token.unwrap();

        assert!(self.pos <= t.pos);
        assert!(t.len != 0);
        self.pos = t.pos + t.len;

        self.last = t.kind.clone();

        // skip
        match t.kind {
            TokenKind::FuncArgOpen | TokenKind::FuncArg => {
                let _ = self.skip_one(',');
                self.skip_whitespace();
            }
            _ => {}
        }

        Some(t)
    }
}

impl<'a> Tokenizer<'a> {
    pub fn new(src: &'a str) -> Self {
        Self {
            src,
            pos: 0,
            last: TokenKind::Newline,
        }
    }
    pub fn src(&self) -> &str {
        &self.src[self.pos..]
    }
    pub fn get_str(&self, token: &Token) -> &str {
        &self.src[token.pos..(token.pos + token.len)]
    }

    //fn set_state(&mut self, last: &TokenKind) {
    //    let l = last.clone();
    //    match l {
    //        TokenKind::Newline => self.state = l,
    //        _ => {}
    //    }
    //}

    pub fn skip_one(&mut self, c: char) -> Option<()> {
        let src = self.src();
        let s0 = src.chars().nth(0)?;

        if s0 == c {
            self.pos += 1;
            return Some(());
        }

        None
    }

    pub fn skip_char<F>(&mut self, f: F)
    where
        F: Fn(char) -> bool,
    {
        let src = self.src().chars();
        let mut n = 0;
        for c in src {
            if f(c) {
                n += 1;
            }
            {
                break;
            }
        }
        self.pos += n;
    }

    pub fn skip_whitespace(&mut self) {
        self.skip_char(|c| c.is_whitespace())
    }

    pub fn get_top_token(&mut self) -> Option<Token> {
        if self.src().len() == 0 {
            return None;
        }
        let c = &self.src().chars().nth(0).unwrap();

        match c {
            '=' => {
                let title = get_title(&self.src());
                if title.is_some() {
                    let (level, name) = title.unwrap();
                    let title = TokenKind::Title(level);
                    //println!("title({}): \"{}\"", level, name);

                    return Some(Token {
                        kind: title,
                        pos: self.pos + level + 1,
                        len: name.len(),
                    });
                }
            }
            '\n' => {
                return Some(Token {
                    kind: TokenKind::Newline,
                    pos: self.pos,
                    len: 1,
                });
            }
            '@' => {
                let at = get_at(&self.src());
                if let Some(at) = at {
                    let (kind, at) = at;
                    //println!("at: {}", at);

                    return Some(Token {
                        kind,
                        pos: self.pos + 1,
                        len: at.len(),
                    });
                }
            }
            _ => {
                let s = get_sentence(self.src());
                return Some(Token {
                    kind: TokenKind::Sentence,
                    pos: self.pos,
                    len: s.len(),
                });
            }
        }

        None
    }

    pub fn get_token(&self) -> Option<Token> {
        match &self.src().chars().nth(0).unwrap() {
            '\n' => {
                return Some(Token {
                    kind: TokenKind::Newline,
                    pos: self.pos,
                    len: 1,
                });
            }
            '@' => {
                let at = get_at(&self.src());
                if let Some(at) = at {
                    let (kind, at) = at;
                    //println!("at: {}", at);

                    return Some(Token {
                        kind,
                        pos: self.pos + 1,
                        len: at.len(),
                    });
                }
            }
            _ => {
                let s = get_sentence(self.src());
                return Some(Token {
                    kind: TokenKind::Sentence,
                    pos: self.pos,
                    len: s.len(),
                });
            }
        }
        None
    }

    pub fn get_func_arg_or_default(&self) -> Option<Token> {
        let src = &self.src();
        return match &src.chars().nth(0).unwrap() {
            '(' => Some(Token {
                kind: TokenKind::FuncArgOpen,
                pos: self.pos,
                len: 1,
            }),
            _ => self.get_token(),
        };
    }

    pub fn get_func_arg(&self) -> Option<Token> {
        let src = &self.src();
        if src.chars().nth(0).unwrap() == ')' {
            return Some(Token {
                kind: TokenKind::FuncArgClose,
                pos: self.pos,
                len: 1,
            });
        }

        let src = src.chars().enumerate();

        for c in src {
            let (i, c) = c;
            match c {
                ',' | ')' => {
                    println!("arg: {}", &self.src()[..i]);
                    return Some(Token {
                        kind: TokenKind::FuncArg,
                        pos: self.pos,
                        len: i,
                    });
                }
                _ => {}
            }
        }
        None
    }

    pub fn get_func_block_or_default(&self) -> Option<Token> {
        let src = &self.src();
        if src.chars().nth(0).unwrap() != '{' {
            return self.get_token();
        }

        let mut n = 0;
        for c in src.chars() {
            n += 1;
            if c == '}' {
                break;
            }
        }
        Some(Token {
            kind: TokenKind::FuncBlock,
            pos: self.pos,
            len: n,
        })
    }
}

fn get_sentence(s: &str) -> &str {
    let it = s.chars().enumerate();

    let mut last = 'A';
    let mut n = 0;
    for c in it {
        let (i, c) = c;

        match c {
            '\n' => break,
            '@' => {
                if last.is_whitespace() {
                    break;
                }
            }
            _ => {}
        }
        n = i;
        last = c;
    }
    println!("sentence: \"{}\"", &s[..n + 1]);
    &s[..n + 1]
}

fn get_title(mut s: &str) -> Option<(usize, &str)> {
    let mut level = 0;
    let mut it = s.chars().enumerate();

    // get level
    loop {
        let c = it.next();
        if c.is_none() {
            return None;
        }
        let (i, c) = c.unwrap();
        match c {
            ' ' => {
                s = &s[level + 1..];
                s = s.split('\n').next().unwrap();
                break;
            }
            '=' => level += 1,
            _ => return None,
        }
    }

    Some((level, s))
}

fn get_at(s: &str) -> Option<(TokenKind, &str)> {
    let mut it = s.chars();
    assert_eq!(it.next().unwrap(), '@');
    let first = it.next().unwrap();

    let it = it.enumerate();
    if first == '<' {
        // func
        for c in it {
            let (i, c) = c;
            //println!("{}, {}", i, c);
            if c == '>' {
                return Some((TokenKind::Func, &s[..i + 2]));
            }
        }
        return None;
    }

    let mut n = 1;

    for c in it {
        let (i, c) = c;
        match c {
            'a'..='z' | 'A'..='Z' => continue,
            '0'..='9' | '.' | '_' => continue,
            _ => {
                n += i;
                break;
            }
        }
    }

    return Some((TokenKind::AtString, &s[..n]));
}

#[cfg(test)]
mod tests {
    use crate::*;
    #[test]
    fn simple() {
        let s = r#"= title level 1
hoge fuga
aaa===beabnea

== title level 2

@sksat_tty
@<func>
@<func>(arg)
@<func>(arg1,arg2)
@<func>(arg1,arg2,arg3){block}
@<func>(a1, a2, a3){
    b0
    b1
    b3
}

function test: @<func>
SNS test: @sksat_tty @sksat@mstdn.maud.io
email test: sksat@sksat.net
"#;

        let tokenizer = token::Tokenizer::new(s);
        println!("string:\n{}", s);
        for t in tokenizer.clone() {
            //println!("token: {:?}", token.collect::<Vec<token::Token>>());
            println!("{:?}: \"{}\"", t, tokenizer.get_str(&t));
        }
    }

    #[test]
    fn title() {
        let (n, s) = token::get_title("== hoge").unwrap();
        assert_eq!(n, 2);
        assert_eq!(s, "hoge");
    }
}
