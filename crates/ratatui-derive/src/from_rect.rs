use darling::util::{Flag, Ignored};
use darling::{FromDeriveInput, ast};
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::punctuated::Punctuated;
use syn::{Attribute, DeriveInput, Error, Expr, Field, Generics, Ident, Path, Result, Token};

pub(crate) fn expand(input: &DeriveInput) -> TokenStream2 {
    let input = match FromRectInput::from_derive_input(input) {
        Ok(input) => input,
        Err(error) => return error.write_errors(),
    };

    match expand_validated_from_rect(&input) {
        Ok(tokens) => tokens,
        Err(error) => error.to_compile_error(),
    }
}

fn expand_validated_from_rect(input: &FromRectInput) -> Result<TokenStream2> {
    let direction = input.direction()?;
    let layout_path = input.layout_path();
    let direction_method = direction.method_ident();
    let fields = input.fields()?;
    let flex = input.flex_variant()?;

    let field_idents = fields
        .iter()
        .map(|field| field.ident.clone())
        .collect::<Vec<_>>();
    let constraints = fields
        .iter()
        .map(|field| field.constraint.to_tokens(&layout_path))
        .collect::<Vec<_>>();
    let layout_options = input.layout_option_tokens(&layout_path, flex.as_ref());
    let ident = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics ::core::convert::From<#layout_path::Rect> for #ident #ty_generics
        #where_clause
        {
            fn from(area: #layout_path::Rect) -> Self {
                let constraints = [
                    #(#constraints),*
                ];
                let [#(#field_idents),*] =
                    #layout_path::Layout::#direction_method(constraints)
                        #(#layout_options)*
                        .areas(area);
                Self {
                    #(#field_idents),*
                }
            }
        }
    })
}

#[derive(FromDeriveInput)]
#[darling(attributes(layout), supports(struct_named))]
struct FromRectInput {
    ident: Ident,
    generics: Generics,
    data: ast::Data<Ignored, Field>,
    #[darling(default)]
    horizontal: Flag,
    #[darling(default)]
    vertical: Flag,
    spacing: Option<Expr>,
    margin: Option<Expr>,
    horizontal_margin: Option<Expr>,
    vertical_margin: Option<Expr>,
    flex: Option<Path>,
    #[darling(default, rename = "crate")]
    crate_path: Option<Path>,
}

impl FromRectInput {
    fn direction(&self) -> Result<Direction> {
        match (self.horizontal.is_present(), self.vertical.is_present()) {
            (true, false) => Ok(Direction::Horizontal),
            (false, true) => Ok(Direction::Vertical),
            (false, false) => Err(Error::new_spanned(
                &self.ident,
                "missing layout direction: add #[layout(horizontal)] or #[layout(vertical)]",
            )),
            (true, true) => Err(Error::new(
                self.vertical.span(),
                "layout direction can only be specified once",
            )),
        }
    }

    fn fields(&self) -> Result<Vec<LayoutField>> {
        let ast::Data::Struct(fields) = &self.data else {
            return Err(Error::new_spanned(
                &self.ident,
                "FromRect can only be derived for structs with named fields",
            ));
        };

        if fields.is_empty() {
            return Err(Error::new_spanned(
                &self.ident,
                "FromRect requires at least one named field",
            ));
        }

        let mut parsed_fields = Vec::with_capacity(fields.len());
        let mut errors = None;

        for field in &fields.fields {
            match LayoutField::from_field(field) {
                Ok(field) => parsed_fields.push(field),
                Err(error) => push_error(&mut errors, error),
            }
        }

        if let Some(error) = errors {
            return Err(error);
        }

        Ok(parsed_fields)
    }

    fn layout_path(&self) -> TokenStream2 {
        if let Some(crate_path) = &self.crate_path {
            quote!(#crate_path::layout)
        } else {
            quote!(::ratatui::layout)
        }
    }

    fn flex_variant(&self) -> Result<Option<Ident>> {
        let Some(flex) = &self.flex else {
            return Ok(None);
        };

        parse_flex_variant(flex).map(Some)
    }

    fn layout_option_tokens(
        &self,
        layout_path: &TokenStream2,
        flex: Option<&Ident>,
    ) -> Vec<TokenStream2> {
        let mut options = Vec::new();

        if let Some(margin) = &self.margin {
            options.push(quote!(.margin(#margin)));
        }
        if let Some(horizontal_margin) = &self.horizontal_margin {
            options.push(quote!(.horizontal_margin(#horizontal_margin)));
        }
        if let Some(vertical_margin) = &self.vertical_margin {
            options.push(quote!(.vertical_margin(#vertical_margin)));
        }
        if let Some(flex) = flex {
            options.push(quote!(.flex(#layout_path::Flex::#flex)));
        }
        if let Some(spacing) = &self.spacing {
            options.push(quote!(.spacing(#spacing)));
        }

        options
    }
}

fn parse_flex_variant(path: &Path) -> Result<Ident> {
    let Some(ident) = path.get_ident().cloned() else {
        return Err(Error::new_spanned(
            path,
            "expected a Flex variant: Start, End, Center, SpaceBetween, SpaceAround, \
             SpaceEvenly, or Legacy",
        ));
    };

    match ident.to_string().as_str() {
        "Start" | "End" | "Center" | "SpaceBetween" | "SpaceAround" | "SpaceEvenly" | "Legacy" => {
            Ok(ident)
        }
        _ => Err(Error::new_spanned(
            ident,
            "expected a Flex variant: Start, End, Center, SpaceBetween, SpaceAround, \
             SpaceEvenly, or Legacy",
        )),
    }
}

struct LayoutField {
    ident: Ident,
    constraint: Constraint,
}

impl LayoutField {
    fn from_field(field: &Field) -> Result<Self> {
        let Some(ident) = field.ident.clone() else {
            return Err(Error::new_spanned(
                field,
                "FromRect only supports named fields",
            ));
        };
        let constraint = parse_constraint(field)?;

        Ok(Self { ident, constraint })
    }
}

enum Direction {
    Horizontal,
    Vertical,
}

impl Direction {
    fn method_ident(&self) -> Ident {
        match self {
            Self::Horizontal => format_ident!("horizontal"),
            Self::Vertical => format_ident!("vertical"),
        }
    }
}

enum Constraint {
    Length(Expr),
    Min(Expr),
    Max(Expr),
    Percentage(Expr),
    Ratio { numerator: Expr, denominator: Expr },
    Fill(Expr),
}

impl Constraint {
    fn to_tokens(&self, layout_path: &TokenStream2) -> TokenStream2 {
        match self {
            Self::Length(expr) => quote!(#layout_path::Constraint::Length(#expr)),
            Self::Min(expr) => quote!(#layout_path::Constraint::Min(#expr)),
            Self::Max(expr) => quote!(#layout_path::Constraint::Max(#expr)),
            Self::Percentage(expr) => quote!(#layout_path::Constraint::Percentage(#expr)),
            Self::Ratio {
                numerator,
                denominator,
            } => {
                quote!(#layout_path::Constraint::Ratio(#numerator, #denominator))
            }
            Self::Fill(expr) => quote!(#layout_path::Constraint::Fill(#expr)),
        }
    }
}

fn parse_constraint(field: &Field) -> Result<Constraint> {
    let mut constraint = None;
    let mut saw_constraint_attr = false;
    let mut errors = None;

    for attr in &field.attrs {
        let Some(kind) = ConstraintKind::from_attr(attr) else {
            continue;
        };
        saw_constraint_attr = true;

        if constraint.is_some() {
            push_error(
                &mut errors,
                Error::new_spanned(attr, "field can only have one layout constraint attribute"),
            );
            continue;
        }

        match kind.parse(attr) {
            Ok(parsed_constraint) => constraint = Some(parsed_constraint),
            Err(error) => push_error(&mut errors, error),
        }
    }

    if !saw_constraint_attr {
        push_error(
            &mut errors,
            Error::new_spanned(
                field,
                "missing layout constraint attribute: add one of #[length(expr)], #[min(expr)], \
                 #[max(expr)], #[percentage(expr)], #[ratio(numerator, denominator)], or \
                 #[fill(expr)]",
            ),
        );
    }

    if let Some(error) = errors {
        return Err(error);
    }

    constraint.ok_or_else(|| Error::new_spanned(field, "missing layout constraint attribute"))
}

enum ConstraintKind {
    Length,
    Min,
    Max,
    Percentage,
    Ratio,
    Fill,
}

impl ConstraintKind {
    fn from_attr(attr: &Attribute) -> Option<Self> {
        let ident = attr.path().get_ident()?;

        match ident.to_string().as_str() {
            "length" => Some(Self::Length),
            "min" => Some(Self::Min),
            "max" => Some(Self::Max),
            "percentage" => Some(Self::Percentage),
            "ratio" => Some(Self::Ratio),
            "fill" => Some(Self::Fill),
            _ => None,
        }
    }

    fn parse(&self, attr: &Attribute) -> Result<Constraint> {
        match self {
            Self::Length => single_expr_constraint(attr, "length", Constraint::Length),
            Self::Min => single_expr_constraint(attr, "min", Constraint::Min),
            Self::Max => single_expr_constraint(attr, "max", Constraint::Max),
            Self::Percentage => single_expr_constraint(attr, "percentage", Constraint::Percentage),
            Self::Fill => single_expr_constraint(attr, "fill", Constraint::Fill),
            Self::Ratio => ratio_constraint(attr),
        }
    }
}

fn single_expr_constraint(
    attr: &Attribute,
    attr_name: &'static str,
    build: impl FnOnce(Expr) -> Constraint,
) -> Result<Constraint> {
    let expr = attr.parse_args::<Expr>().map_err(|error| {
        let mut new_error = Error::new_spanned(attr, format!("expected #[{attr_name}(expr)]"));
        new_error.combine(error);
        new_error
    })?;

    Ok(build(expr))
}

fn ratio_constraint(attr: &Attribute) -> Result<Constraint> {
    let args = attr.parse_args_with(Punctuated::<Expr, Token![,]>::parse_terminated)?;
    if args.len() != 2 {
        return Err(Error::new_spanned(
            attr,
            "expected #[ratio(numerator, denominator)]",
        ));
    }

    let mut args = args.into_iter();
    let Some(numerator) = args.next() else {
        return Err(Error::new_spanned(
            attr,
            "expected #[ratio(numerator, denominator)]",
        ));
    };
    let Some(denominator) = args.next() else {
        return Err(Error::new_spanned(
            attr,
            "expected #[ratio(numerator, denominator)]",
        ));
    };

    Ok(Constraint::Ratio {
        numerator,
        denominator,
    })
}

fn push_error(errors: &mut Option<Error>, error: Error) {
    if let Some(errors) = errors {
        errors.combine(error);
    } else {
        *errors = Some(error);
    }
}
