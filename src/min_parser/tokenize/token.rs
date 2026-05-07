use core::iter::Enumerate;
use core::slice::Iter;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Position {
    pub offset: usize,
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Ident,
    IntDec,
    IntHex,
    IntBin,
    Float,
    Symbol,
    String,

    Space,
    Tab,
    Newline,

    LParen,
    RParen,
    LBrace,
    RBrace,
    LBrack,
    RBrack,
}

#[derive(Debug, Clone, Eq)]
pub struct Token<'a> {
    pub kind: TokenKind,
    pub text: &'a str,
    pub start: Position,
    pub end: Position,
}

impl PartialEq for Token<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind && self.text == other.text
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TokenSlice<'a>(pub &'a [Token<'a>]);

impl<'a> From<&'a [Token<'a>]> for TokenSlice<'a> {
    fn from(tokens: &'a [Token<'a>]) -> Self {
        TokenSlice(tokens)
    }
}

impl<'a> From<TokenSlice<'a>> for &'a [Token<'a>] {
    fn from(slice: TokenSlice<'a>) -> Self {
        slice.0
    }
}

impl<'a> nom::Input for TokenSlice<'a> {
    type Item = &'a Token<'a>;
    type Iter = Iter<'a, Token<'a>>;
    type IterIndices = Enumerate<Self::Iter>;

    fn input_len(&self) -> usize {
        self.0.len()
    }

    fn take(&self, index: usize) -> Self {
        TokenSlice(&self.0[..index])
    }

    fn take_from(&self, index: usize) -> Self {
        TokenSlice(&self.0[index..])
    }

    fn take_split(&self, index: usize) -> (Self, Self) {
        let (prefix, suffix) = self.0.split_at(index);
        (TokenSlice(suffix), TokenSlice(prefix))
    }

    fn position<P>(&self, predicate: P) -> Option<usize>
    where
        P: Fn(Self::Item) -> bool,
    {
        self.0.iter().position(predicate)
    }

    fn iter_elements(&self) -> Self::Iter {
        self.0.iter()
    }

    fn iter_indices(&self) -> Self::IterIndices {
        self.0.iter().enumerate()
    }

    fn slice_index(&self, count: usize) -> Result<usize, nom::Needed> {
        if self.0.len() >= count {
            Ok(count)
        } else {
            Err(nom::Needed::new(count - self.0.len()))
        }
    }
}

#[cfg(test)]
impl<'a> Token<'a> {
    pub fn new_fortest(kind: TokenKind, text: &'a str) -> Self {
        let invalid_position = Position {
            offset: usize::MAX,
            line: u32::MAX,
            column: u32::MAX,
        };

        Self {
            kind,
            text,
            start: invalid_position,
            end: invalid_position,
        }
    }
}