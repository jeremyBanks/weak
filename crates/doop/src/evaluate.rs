use {
    crate::{parse::DoopBlock, tokens::Tokens, *},
    indexmap::{IndexMap, IndexSet},
    itertools::Itertools,
    proc_macro2::{Group, Ident, TokenStream, TokenTree},
    quote::ToTokens,
    std::{
        hash::{Hash, Hasher},
        iter::empty,
        ops::Deref,
        rc::Rc,
    },
};

pub struct DoopItem {
    pub for_bindings: Vec<ForBinding>,
    pub body: TokenStream,
}

pub struct ForBinding {
    pub target: Option<syn::Ident>,
    pub entries: Vec<TokenStream>,
}

pub fn evaluate(input: DoopBlock) -> Result<TokenStream, syn::Error> {
    let mut output = TokenStream::new();

    // Mappings from identifiers to ordered sets of tokens representing possible
    // replacements. These are the bindings created by use of the top-level
    // `let` statement.
    let mut let_bindings = IndexMap::<syn::Ident, IndexSet<Tokens>>::new();

    // Mappings from identifiers to tokens representing a single replacement.
    // These are the bindings created for each iteration of a `for` loop body.
    let mut for_bindings = IndexMap::<syn::Ident, Tokens>::new();

    for item in input.items {
        use parse::DoopBlockItem::*;
        match item {
            Static(item) => {
                output.extend(item.body);
            }
            Let(item) => {
                let token_lists = evaluate_binding_terms(
                    &item.first_term,
                    &item.rest_terms,
                    &let_bindings,
                    None,
                )?;
                let_bindings.insert(item.ident, token_lists);
            }
            For(item) => {
                let input_body = Tokens::from_iter(item.body);

                let mut all_binding_combinations: Vec<IndexMap<Ident, Tokens>> =
                    vec![IndexMap::new()];

                for binding in item.bindings {
                    let mut new_binding_combinations = vec![];

                    for binding_combination in &mut all_binding_combinations {
                        let token_lists = evaluate_binding_terms(
                            &binding.first_term,
                            &binding.rest_terms,
                            &let_bindings,
                            Some(binding_combination),
                        )?;

                        for token_list in token_lists {
                            let token_list = token_list.replace(&*binding_combination);
                            let mut new_binding_combination = binding_combination.clone();
                            match &binding.target {
                                parse::ForBindingTarget::Ident(ident) => {
                                    if let Some(ident) = ident.ident() {
                                        new_binding_combination.insert(ident.clone(), token_list);
                                    }
                                }
                                parse::ForBindingTarget::Tuple(idents) => {
                                    let tuple_value = token_list.into_inner();
                                    assert!(tuple_value.len() == 1);
                                    let tuple_group = match &tuple_value[0] {
                                        TokenTree::Group(target) => target,
                                        _ => unreachable!(),
                                    };
                                    assert!(
                                        tuple_group.delimiter()
                                            == proc_macro2::Delimiter::Parenthesis
                                    );
                                    let tuple_body = Tokens::from_iter(tuple_group.stream());

                                    let tuple_tokens: Vec<Tokens> = tuple_body
                                        .into_inner()
                                        .split(|token| {
                                            if let TokenTree::Punct(punct) = token {
                                                punct.as_char() == ','
                                            } else {
                                                false
                                            }
                                        })
                                        .map(|slice| slice.iter().cloned().collect())
                                        .collect();
                                    assert!(tuple_tokens.len() == idents.items.len());

                                    for (target, binding) in idents.items.iter().zip(tuple_tokens) {
                                        if let Some(ident) = target.ident() {
                                            new_binding_combination.insert(ident.clone(), binding);
                                        }
                                    }
                                }
                            }
                            new_binding_combinations.push(new_binding_combination);
                        }
                    }

                    all_binding_combinations = new_binding_combinations;
                }

                for bindings in all_binding_combinations.iter() {
                    output.extend(input_body.replace(bindings));
                }
            }
        }
    }

    /// evaluates a binding term, which may be an identifier (from a previous)
    /// `let` statement, or a braced or bracketed, comma-delimited, list of
    /// replacements. returns a token stream.
    fn evaluate_binding_term(
        term: &parse::BindingTerm,
        let_bindings: &IndexMap<syn::Ident, IndexSet<Tokens>>,
        for_bindings: Option<&IndexMap<syn::Ident, Tokens>>,
    ) -> Result<IndexSet<Tokens>, syn::Error> {
        Ok(match term {
            parse::BindingTerm::Ident(ident) => match let_bindings.get(ident) {
                Some(bindings) => bindings.clone(),
                None =>
                    return Err(syn::Error::new(
                        ident.span(),
                        format!("undefined doop variable {ident:?}"),
                    )),
            },

            parse::BindingTerm::BraceList(term) =>
                IndexSet::from_iter(term.entries.iter().map(|term| {
                    let mut tokens = Tokens::from_iter(term.clone());
                    if let Some(for_bindings) = for_bindings {
                        tokens = tokens.replace(for_bindings);
                    }
                    tokens
                })),
            parse::BindingTerm::BracketList(term) =>
                IndexSet::from_iter(term.entries.iter().map(|term| {
                    let mut tokens = Tokens::from_iter(term.clone());
                    if let Some(for_bindings) = for_bindings {
                        tokens = tokens.replace(for_bindings);
                    }
                    tokens
                })),
        })
    }

    fn evaluate_binding_terms(
        first: &parse::BindingTerm,
        rest: &[parse::RestTerm],
        let_bindings: &IndexMap<syn::Ident, IndexSet<Tokens>>,
        for_bindings: Option<&IndexMap<syn::Ident, Tokens>>,
    ) -> Result<IndexSet<Tokens>, syn::Error> {
        let mut token_lists = evaluate_binding_term(first, let_bindings, for_bindings)?;

        for term in rest {
            let term_token_lists = evaluate_binding_term(&term.term, let_bindings, for_bindings)?;
            match term.operation {
                parse::AddOrSub::Add(_) => token_lists.extend(term_token_lists),
                parse::AddOrSub::Sub(_) =>
                    token_lists = token_lists.difference(&term_token_lists).cloned().collect(),
            }
        }

        Ok(token_lists)
    }

    Ok(output)
}

impl Tokens {
    pub fn replace(&self, replacements: &IndexMap<Ident, Tokens>) -> Tokens {
        let mut output = Vec::new();
        for token in self.clone() {
            match token {
                TokenTree::Ident(ref candidate) =>
                    if let Some(replacement) = replacements.get(candidate) {
                        output.extend(replacement.clone().into_token_stream());
                    } else {
                        output.push(token);
                    },

                TokenTree::Group(group) => output.extend([TokenTree::Group(Group::new(
                    group.delimiter(),
                    Tokens::from_iter(group.stream()).replace(replacements).into_token_stream(),
                ))]),

                _ => output.push(token),
            }
        }
        Tokens::from_iter(output)
    }
}