use alloc::borrow::ToOwned;
use alloc::collections::btree_map::BTreeMap;
use alloc::string::{String, ToString};
use alloc::{format, rc::Rc};

use crate::common::Name;
use crate::min_parser::tokenize::{
    token::TokenKind,
    token_tree::{GroupKind, TokenTree},
};

pub struct ParserEnv<R> {
    pub categories: BTreeMap<String, Category<R>>,
}

pub struct Category<R> {
    pub leading: BTreeMap<String, Rc<ParserFn<R>>>,
    pub trailing: BTreeMap<String, (u32, Assoc, Rc<TrailingParserFn<R>>)>,
}

impl<R> Default for Category<R> {
    fn default() -> Self {
        Self {
            leading: BTreeMap::default(),
            trailing: BTreeMap::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Assoc {
    Left,
    Right,
}

pub type ParserResult<'a, R> = Result<(R, ParserContext<'a>), String>;
pub type ParserFn<R> = dyn for<'a> Fn(ParserContext<'a>, &ParserEnv<R>) -> ParserResult<'a, R>;
pub type TrailingParserFn<R> =
    dyn for<'a> Fn(ParserContext<'a>, &ParserEnv<R>, R, u32) -> ParserResult<'a, R>;

#[derive(Debug, Clone, Copy)]
pub struct ParserContext<'a>(pub &'a [TokenTree<'a>]);

impl<'a> ParserContext<'a> {
    #[must_use]
    pub const fn peek(self) -> Option<&'a TokenTree<'a>> {
        self.0.first()
    }

    pub fn next_token(self) -> ParserResult<'a, &'a TokenTree<'a>> {
        match self.0.split_first() {
            Some((first, last)) => Ok((first, ParserContext(last))),
            None => Err("Unexpected end of input".to_owned()),
        }
    }

    pub fn parse<R>(
        mut self,
        env: &ParserEnv<R>,
        category_name: &str,
        min_bp: u32,
    ) -> ParserResult<'a, R> {
        let category = env
            .categories
            .get(category_name)
            .ok_or_else(|| format!("Category '{category_name}' not found"))?;

        let token = self.peek().ok_or("Unexpected end of input")?;
        let key = get_lookup_key(env, token);

        let leading_parser = category
            .leading
            .get(key)
            .ok_or_else(|| format!("Unexpected token for leading parser: {key:?}",))?;

        let (mut left, new_ctx) = leading_parser(self, env)?;
        self = new_ctx;

        while let Some(next_token) = self.peek() {
            let next_key = get_lookup_key(env, next_token);

            if let Some((bp, assoc, trailing_parser)) = category.trailing.get(next_key) {
                if *bp < min_bp {
                    break;
                }

                let next_bp = match assoc {
                    Assoc::Left => bp + 1,
                    Assoc::Right => *bp,
                };

                let (new_left, next_ctx) = trailing_parser(self, env, left, next_bp)?;
                left = new_left;
                self = next_ctx;
            } else {
                break;
            }
        }

        Ok((left, self))
    }

    pub fn expect(self, expected_text: &str) -> ParserResult<'a, ()> {
        let token = self.peek().ok_or("Unexpected end of input")?;
        let key = match token {
            TokenTree::Token(t) => t.text,
            TokenTree::Group { .. } => "",
        };

        if key == expected_text {
            Ok(((), self.next_token()?.1))
        } else {
            Err(format!("Expected '{expected_text}', found '{key}'"))
        }
    }

    pub fn expect_ident(self) -> ParserResult<'a, Name> {
        let (token, ctx) = self.next_token()?;
        match token {
            TokenTree::Token(t)
                if t.kind == TokenKind::Ident
                    && !["def", "axiom", "notation"].contains(&t.text) =>
            {
                Ok((t.text.to_string(), ctx))
            }
            _ => Err(format!("Expected Identifier, found {token:?}")),
        }
    }

    pub fn expect_number(self) -> ParserResult<'a, u64> {
        let (token, ctx) = self.next_token()?;
        match token {
            TokenTree::Token(t) if t.kind == TokenKind::IntDec => {
                // Remove underscores and parse as u64
                let num_str = t.text.replace('_', "");
                match num_str.parse::<u64>() {
                    Ok(value) => Ok((value, ctx)),
                    Err(e) => Err(format!("Invalid integer '{}': {}", t.text, e)),
                }
            }
            _ => Err(format!("Expected number, found {token:?}")),
        }
    }
    pub fn expect_symbol(self) -> ParserResult<'a, Name> {
        let (token, ctx) = self.next_token()?;
        match token {
            TokenTree::Token(t) if t.kind == TokenKind::Symbol => Ok((t.text.to_string(), ctx)),
            _ => Err(format!("Expected Symbol, found {token:?}")),
        }
    }
}

#[must_use]
pub fn get_lookup_key<'a, R>(env: &ParserEnv<R>, tree: &TokenTree<'a>) -> &'a str {
    match tree {
        TokenTree::Token(token) => match token.kind {
            TokenKind::Ident => {
                // Check if token text is a reserved keyword (has leading or trailing parser in any category)
                for category in env.categories.values() {
                    if category.leading.contains_key(token.text)
                        || category.trailing.contains_key(token.text)
                    {
                        return token.text;
                    }
                }
                // Check if token text is a category name
                if env.categories.contains_key(token.text) {
                    token.text
                } else {
                    "@ident"
                }
            }

            TokenKind::IntDec | TokenKind::IntHex | TokenKind::IntBin => "@int",
            TokenKind::Float => "@float",
            TokenKind::String => "@str",

            _ => token.text,
        },
        TokenTree::Group { kind, .. } => match kind {
            GroupKind::Paren => "@paren",
            GroupKind::Brace => "@brace",
            GroupKind::Brack => "@brack",
        },
    }
}

pub fn parse_inside_group<'a, T>(
    group_token: &'a TokenTree<'a>,
    expected_kind: GroupKind,
    parse_fn: impl Fn(ParserContext<'a>) -> ParserResult<'a, T>,
) -> Result<T, String> {
    if let TokenTree::Group { kind, body, .. } = group_token {
        if *kind != expected_kind {
            return Err(format!("Expected {expected_kind:?}, found {kind:?}"));
        }
        let ctx = ParserContext(body);
        let (result, ctx) = parse_fn(ctx)?;

        if ctx.peek().is_some() {
            return Err("Unexpected trailing tokens inside group".into());
        }
        Ok(result)
    } else {
        Err("Expected a group token".into())
    }
}
