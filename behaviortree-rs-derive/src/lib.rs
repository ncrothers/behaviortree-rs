use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::ToTokens;
use syn::{parse::Parse, punctuated::Punctuated, token::Comma, DeriveInput};

#[macro_use]
extern crate quote;
#[macro_use]
extern crate syn;

extern crate proc_macro;

#[proc_macro_derive(FromString)]
pub fn derive_from_string(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let ident = input.ident;

    let expanded = quote! {
        impl ::behaviortree_rs::basic_types::FromString for #ident {
            type Err = <#ident as ::core::str::FromStr>::Err;

            fn from_string(value: impl AsRef<str>) -> Result<#ident, Self::Err> {
                value.as_ref().parse()
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(BTToString)]
pub fn derive_bt_to_string(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let ident = input.ident;

    let expanded = quote! {
        impl ::behaviortree_rs::basic_types::BTToString for #ident {
            fn bt_to_string(&self) -> String {
                ::std::string::ToString::to_string(self)
            }
        }
    };

    TokenStream::from(expanded)
}

struct NodeRegistration {
    factory: syn::Ident,
    name: proc_macro2::TokenStream,
    node_type: syn::Type,
    params: Punctuated<syn::Expr, Comma>,
}

impl Parse for NodeRegistration {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let factory = input.parse()?;
        input.parse::<Token![,]>()?;

        let node_name = input.parse::<syn::Expr>()?.to_token_stream();

        input.parse::<Token![,]>()?;
        let node_type = input.parse()?;
        // If there are extra parameters, try to parse a comma. Otherwise skip
        if !input.is_empty() {
            input.parse::<Token![,]>()?;
        }

        let params = input.parse_terminated(syn::Expr::parse, Token![,])?;

        Ok(Self {
            factory,
            name: node_name,
            node_type,
            params,
        })
    }
}

fn build_node(node: &NodeRegistration) -> proc_macro2::TokenStream {
    let NodeRegistration {
        factory: _,
        name,
        node_type,
        params,
    } = node;

    let cloned_names = (0..params.len()).fold(quote! {}, |acc, i| {
        let arg_name = Ident::new(&format!("arg{i}"), Span::call_site());
        quote! { #acc, #arg_name.clone() }
    });

    quote! {
        {
            let mut node = #node_type::create_node(#name, config #cloned_names);
            let manifest = ::behaviortree_rs::basic_types::TreeNodeManifest {
                node_type: node.node_category(),
                registration_id: #name.into(),
                ports: node.provided_ports(),
                description: ::std::string::String::new(),
            };
            node.config_mut().set_manifest(::std::sync::Arc::new(manifest));
            node
        }
    }
}

fn register_node(
    input: TokenStream,
    node_type_token: proc_macro2::TokenStream,
    node_type: NodeTypeInternal,
) -> TokenStream {
    let node_registration = parse_macro_input!(input as NodeRegistration);

    let factory = &node_registration.factory;
    let name = &node_registration.name;
    let params = &node_registration.params;

    // Create expression that clones all parameters
    let param_clone_expr = params.iter().enumerate().fold(quote! {}, |acc, (i, item)| {
        let arg_name = Ident::new(&format!("arg{i}"), Span::call_site());
        quote! {
            #acc
            let #arg_name = #item.clone();
        }
    });

    let node = build_node(&node_registration);

    let extra_steps = match node_type {
        NodeTypeInternal::Control => quote! {
            node.data.children = children;
        },
        NodeTypeInternal::Decorator => quote! {
            node.data.children = children;
        },
        _ => quote! {},
    };

    let expanded = quote! {
        {
            let blackboard = #factory.blackboard().clone();

            #param_clone_expr

            let node_fn = move |
                config: ::behaviortree_rs::nodes::NodeConfig,
                mut children: ::std::vec::Vec<::behaviortree_rs::nodes::TreeNode>
            | -> ::behaviortree_rs::nodes::TreeNode
            {
                let mut node = #node;

                #extra_steps

                node
            };

            #factory.register_node(#name, node_fn, #node_type_token);
        }
    };

    TokenStream::from(expanded)
}

enum NodeTypeInternal {
    Action,
    Control,
    Decorator,
}

/// Registers an Action type node with the factory.
///
/// **NOTE:** During tree creation, a new node is created using the parameters
/// given after the node type field. You specified these fields in your node struct
/// definition. Each time a node is created, the parameters are cloned using `Clone::clone`.
/// Thus, your parameters must implement `Clone`.
///
/// # Usage
///
/// ```ignore
/// let mut factory = Factory::new();
/// let arg1 = String::from("hello world");
/// let arg2 = 10u32;
///
/// register_action_node!(factory, "TestNode", TestNode, arg1, arg2);
/// ```
#[proc_macro]
pub fn register_action_node(input: TokenStream) -> TokenStream {
    register_node(
        input,
        quote! { ::behaviortree_rs::basic_types::NodeCategory::Action },
        NodeTypeInternal::Action,
    )
}

/// Registers an Control type node with the factory.
///
/// **NOTE:** During tree creation, a new node is created using the parameters
/// given after the node type field. You specified these fields in your node struct
/// definition. Each time a node is created, the parameters are cloned using `Clone::clone`.
/// Thus, your parameters must implement `Clone`.
///
/// # Usage
///
/// ```ignore
/// let mut factory = Factory::new();
/// let arg1 = String::from("hello world");
/// let arg2 = 10u32;
///
/// register_control_node!(factory, "TestNode", TestNode, arg1, arg2);
/// ```
#[proc_macro]
pub fn register_control_node(input: TokenStream) -> TokenStream {
    register_node(
        input,
        quote! { ::behaviortree_rs::basic_types::NodeCategory::Control },
        NodeTypeInternal::Control,
    )
}

/// Registers an Decorator type node with the factory.
///
/// **NOTE:** During tree creation, a new node is created using the parameters
/// given after the node type field. You specified these fields in your node struct
/// definition. Each time a node is created, the parameters are cloned using `Clone::clone`.
/// Thus, your parameters must implement `Clone`.
///
/// # Usage
///
/// ```ignore
/// let mut factory = Factory::new();
/// let arg1 = String::from("hello world");
/// let arg2 = 10u32;
///
/// register_decorator_node!(factory, "TestNode", TestNode, arg1, arg2);
/// ```
#[proc_macro]
pub fn register_decorator_node(input: TokenStream) -> TokenStream {
    register_node(
        input,
        quote! { ::behaviortree_rs::basic_types::NodeCategory::Decorator },
        NodeTypeInternal::Decorator,
    )
}
