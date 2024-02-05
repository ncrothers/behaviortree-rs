use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::ToTokens;
use syn::{
    parse::{Parse, Parser}, punctuated::Punctuated, token::Comma, AttrStyle, DeriveInput, ItemStruct, LitStr
};

#[macro_use]
extern crate quote;
#[macro_use]
extern crate syn;

extern crate proc_macro;

trait ToMap<T, K, V> {
    fn to_map(&self) -> syn::Result<std::collections::HashMap<K, V>>;
}

impl ToMap<Punctuated<syn::Meta, Comma>, syn::Ident, Option<proc_macro2::TokenStream>>
    for Punctuated<syn::Meta, Comma>
{
    /// Convert a list of attribute arguments to a HashMap
    fn to_map(
        &self,
    ) -> syn::Result<std::collections::HashMap<syn::Ident, Option<proc_macro2::TokenStream>>> {
        self.iter()
            .map(|m| {
                match m {
                    syn::Meta::NameValue(arg) => {
                        // Convert Expr to one of the valid types:
                        // Ident (variable name etc)
                        // ExprCall (function call etc)
                        // Lit (literal, for integer types etc)
                        if let syn::Expr::Lit(lit) = &arg.value {
                            if let syn::Lit::Str(arg_str) = &lit.lit {
                                let value = if let Ok(call) = arg_str.parse::<syn::ExprCall>() {
                                    quote! { #call }
                                }
                                else if let Ok(ident) = arg_str.parse::<syn::Ident>() {
                                    quote! { #ident }
                                }
                                else if let Ok(lit) = arg_str.parse::<syn::Lit>() {
                                    quote! { #lit }
                                }
                                else if let Ok(path) = arg_str.parse::<syn::Path>() {
                                    quote! { #path }
                                }
                                else {
                                    return Err(syn::Error::new_spanned(&arg.value, "argument value should be a:  variable, literal, path, function call"))
                                };

                                Ok((arg.path.get_ident().unwrap().clone(), Some(value)))
                            }
                            else {
                                Err(syn::Error::new_spanned(&arg.value, "argument value should be a string literal"))
                            }
                        }
                        else {
                            Err(syn::Error::new_spanned(&arg.value, "argument value should be a string literal"))
                        }
                    }
                    syn::Meta::Path(arg) => {
                        Ok((arg.get_ident().unwrap().clone(), None))
                    }
                    _ => Err(syn::Error::new_spanned(m, "argument type should be Path or NameValue: `#[bt(default)]`, or `#[bt(default = \"String::new()\")]`"))
                }
            })
            .collect()
    }
}

trait ConcatTokenStream {
    fn concat_list(&self, value: proc_macro2::TokenStream) -> proc_macro2::TokenStream;
    fn concat_blocks(&self, value: proc_macro2::TokenStream) -> proc_macro2::TokenStream;
}

impl ConcatTokenStream for proc_macro2::TokenStream {
    fn concat_list(&self, value: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        if self.is_empty() {
            if value.is_empty() {
                // Both are empty
                proc_macro2::TokenStream::new()
            } else {
                // self empty, value not empty
                quote! {
                    #value
                }
            }
        } else if value.is_empty() {
            // self not empty, value empty
            quote! {
                #self
            }
        } else {
            // Both have value
            quote! {
                #self,
                #value
            }
        }
    }

    fn concat_blocks(&self, value: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        if self.is_empty() {
            if value.is_empty() {
                // Both are empty
                proc_macro2::TokenStream::new()
            } else {
                // self empty, value not empty
                quote! {
                    #value
                }
            }
        } else if value.is_empty() {
            // self not empty, value empty
            quote! {
                #self
            }
        } else {
            // Both have value
            quote! {
                #self
                #value
            }
        }
    }
}

fn create_bt_node(
    args: TokenStream,
    mut item: ItemStruct,
) -> syn::Result<proc_macro2::TokenStream> {
    let args_parsed =
        syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated.parse(args)?;

    if args_parsed.empty_or_trailing() {
        return Err(syn::Error::new_spanned(
            args_parsed,
            "you must specify at least one argument: node type",
        ));
    }

    let mut args_parsed_iter = args_parsed.iter();

    let mut derives =
        vec![quote! { ::std::fmt::Debug, ::behaviortree_rs::derive::TreeNodeDefaults }];

    let arg = args_parsed_iter.next().unwrap();

    // Require parameter to be ident, no prefix path
    arg.require_ident()?;

    let type_ident = arg.get_ident().unwrap().to_string();
    // return Err(syn::Error::new_spanned(arg, format!("{type_ident}")));

    let runtime_str = if let Some(runtime) = args_parsed_iter.next() {
        runtime.require_ident()?;

        let ident = runtime.get_ident().unwrap().to_string();

        match ident.as_str() {
            "Async" | "Sync" => {}
            _ => {
                return Err(syn::Error::new_spanned(
                    runtime,
                    format!("unsupported runtime: must be either Async or Sync: {ident}"),
                ))
            }
        }

        ident
    } else {
        String::from("Async")
    };

    let item_ident = &item.ident;

    let mut default_fields = proc_macro2::TokenStream::new();
    let mut manual_fields = proc_macro2::TokenStream::new();
    let mut manual_fields_with_types = proc_macro2::TokenStream::new();
    let mut extra_impls = proc_macro2::TokenStream::new();

    match &mut item.fields {
        syn::Fields::Named(fields) => {
            for f in fields.named.iter_mut() {
                let name = f.ident.as_ref().unwrap();
                let ty = &f.ty;

                let mut used_default = false;
                for a in f.attrs.iter() {
                    if a.path().is_ident("bt") {
                        let args: Punctuated<syn::Meta, Comma> =
                            a.parse_args_with(Punctuated::parse_terminated)?;
                        let args_map = args.to_map()?;

                        // If the default argument was included
                        if let Some(value) = args_map.get(&syn::parse_str("default")?) {
                            used_default = true;
                            // Use the provided default, if provided by user
                            let default_value = if let Some(default_value) = value {
                                quote! { #default_value }
                            }
                            // Otherwise, use Default
                            else {
                                quote! { <#ty>::default() }
                            };

                            default_fields =
                                default_fields.concat_list(quote! { #name: #default_value });
                        }
                    }
                }

                // Mark field as manually specified if
                if !used_default {
                    manual_fields = manual_fields.concat_list(quote! { #name });
                    manual_fields_with_types =
                        manual_fields_with_types.concat_list(quote! { #name: #ty });
                }

                // Remove the bt attribute, keep all others
                f.attrs = f
                    .attrs
                    .clone()
                    .into_iter()
                    .filter(|a| !a.path().is_ident("bt"))
                    .collect();
            }

            fields.named.push(
                syn::Field::parse_named
                    .parse2(quote! { pub name: String })
                    .unwrap(),
            );
            fields.named.push(
                syn::Field::parse_named
                    .parse2(quote! { pub config: ::behaviortree_rs::nodes::NodeConfig })
                    .unwrap(),
            );
            fields.named.push(
                syn::Field::parse_named
                    .parse2(quote! { pub status: ::behaviortree_rs::basic_types::NodeStatus })
                    .unwrap(),
            );

            // Match all possible node types
            match type_ident.as_str() {
                "SyncActionNode" => {
                    // Add proper derive macros
                    derives.push(quote! { ::behaviortree_rs::derive::ActionNode, ::behaviortree_rs::derive::SyncActionNode });
                }
                "StatefulActionNode" => {
                    // Add StatefulActionNode-specific fields
                    fields.named.push(
                        syn::Field::parse_named
                            .parse2(quote! { pub halt_requested: bool })
                            .unwrap(),
                    );
                    default_fields = default_fields.concat_list(quote! { halt_requested: false });
                    // Add proper derive macros
                    derives.push(quote! { ::behaviortree_rs::derive::ActionNode, ::behaviortree_rs::derive::StatefulActionNode });

                    // impl empty tick function
                    extra_impls = extra_impls.concat_blocks(quote! {
                        impl ::behaviortree_rs::nodes::AsyncTick for #item_ident {
                            fn tick(&mut self) -> ::behaviortree_rs::sync::BoxFuture<::behaviortree_rs::NodeResult> {
                                ::std::boxed::Box::pin(async move {
                                    Ok(::behaviortree_rs::basic_types::NodeStatus::Idle)
                                })
                            }
                        }
                    });

                    match runtime_str.as_str() {
                        "Async" => {
                            extra_impls = extra_impls.concat_blocks(quote! {
                                impl ::behaviortree_rs::nodes::action::SyncStatefulActionNode for #item_ident {
                                    fn on_start(&mut self) -> ::behaviortree_rs::NodeResult {
                                        ::behaviortree_rs::sync::block_on(::behaviortree_rs::nodes::action::AsyncStatefulActionNode::on_start(self))
                                    }

                                    fn on_running(&mut self) -> ::behaviortree_rs::NodeResult {
                                        ::behaviortree_rs::sync::block_on(::behaviortree_rs::nodes::action::AsyncStatefulActionNode::on_running(self))
                                    }

                                    fn on_halted(&mut self) {
                                        ::behaviortree_rs::sync::block_on(::behaviortree_rs::nodes::action::AsyncStatefulActionNode::on_halted(self))
                                    }
                                }
                            });
                        }
                        "Sync" => {
                            extra_impls = extra_impls.concat_blocks(quote! {
                                impl ::behaviortree_rs::nodes::action::AsyncStatefulActionNode for #item_ident {
                                    fn on_start(&mut self) -> ::behaviortree_rs::sync::BoxFuture<::behaviortree_rs::NodeResult> {
                                        ::std::boxed::Box::pin(async move {
                                            ::behaviortree_rs::sync::spawn_blocking(move || ::behaviortree_rs::nodes::action::SyncStatefulActionNode::on_start(self)).await
                                        })
                                    }

                                    fn on_running(&mut self) -> ::behaviortree_rs::sync::BoxFuture<::behaviortree_rs::NodeResult> {
                                        ::std::boxed::Box::pin(async move {
                                            ::behaviortree_rs::sync::spawn_blocking(move || ::behaviortree_rs::nodes::action::SyncStatefulActionNode::on_running(self)).await
                                        })
                                    }

                                    fn on_halted(&mut self) -> ::behaviortree_rs::sync::BoxFuture<()>{
                                        ::std::boxed::Box::pin(async move {
                                            ::behaviortree_rs::sync::spawn_blocking(move || ::behaviortree_rs::nodes::action::SyncStatefulActionNode::on_halted(self)).await
                                        })
                                    }
                                }
                            });
                        }
                        _ => unreachable!(),
                    };
                }
                "ControlNode" => {
                    // Add ControlNode-specific fields
                    fields.named.push(
                        syn::Field::parse_named
                            .parse2(quote! { pub children: Vec<::behaviortree_rs::nodes::TreeNodePtr> })
                            .unwrap(),
                    );
                    default_fields = default_fields.concat_list(quote! { children: Vec::new() });
                    // Add proper derive macros
                    derives.push(quote! { ::behaviortree_rs::derive::ControlNode });
                }
                "DecoratorNode" => {
                    // Add DecoratorNode-specific fields
                    fields.named.push(
                        syn::Field::parse_named
                            .parse2(quote! { pub child: Option<::behaviortree_rs::nodes::TreeNodePtr> })
                            .unwrap(),
                    );
                    default_fields = default_fields.concat_list(quote! { child: None });
                    // Add proper derive macros
                    derives.push(quote! { ::behaviortree_rs::derive::DecoratorNode });
                }
                _ => return Err(syn::Error::new_spanned(arg, "unsupported node type")),
            }
        }
        _ => {
            return Err(syn::Error::new_spanned(
                item,
                "expected a struct with named fields",
            ))
        }
    };

    let vis = &item.vis;
    let struct_fields = &item.fields;

    let mut user_attrs = Vec::new();

    for attr in item.attrs.iter() {
        if attr.path().is_ident("derive") {
            derives.push(attr.parse_args()?);
        } else if let AttrStyle::Outer = attr.style {
            user_attrs.push(attr);
        }
    }

    let user_attrs = user_attrs
        .into_iter()
        .fold(proc_macro2::TokenStream::new(), |acc, a| {
            // Only want to transfer outer attributes
            if let AttrStyle::Outer = a.style {
                if acc.is_empty() {
                    quote! {
                        #a
                    }
                } else {
                    quote! {
                        #acc
                        #a
                    }
                }
            } else {
                acc
            }
        });

    // Convert Vec of derive Paths into one TokenStream
    let derives = derives
        .into_iter()
        .fold(proc_macro2::TokenStream::new(), |acc, d| {
            if acc.is_empty() {
                quote! {
                    #d
                }
            } else {
                quote! {
                    #acc, #d
                }
            }
        });

    match runtime_str.as_str() {
        "Async" => {
            extra_impls = extra_impls.concat_blocks(quote! {
                impl ::behaviortree_rs::nodes::SyncTick for #item_ident {
                    fn tick(&mut self) -> ::behaviortree_rs::NodeResult {
                        Err(::behaviortree_rs::nodes::NodeError::UnreachableTick)
                    }
                }

                impl ::behaviortree_rs::nodes::SyncHalt for #item_ident {}
            });
        }
        "Sync" => {
            extra_impls = extra_impls.concat_blocks(quote! {
                impl ::behaviortree_rs::nodes::AsyncTick for #item_ident {
                    fn tick(&mut self) -> ::behaviortree_rs::sync::BoxFuture<::behaviortree_rs::NodeResult> {
                        ::std::boxed::Box::pin(async move {
                            ::behaviortree_rs::sync::spawn_blocking(|| <#item_ident as ::behaviortree_rs::nodes::SyncTick>::tick(self)).await
                        })
                    }
                }

                impl ::behaviortree_rs::nodes::AsyncHalt for #item_ident {
                    fn halt(&mut self) -> ::behaviortree_rs::sync::BoxFuture<()> {
                        ::std::boxed::Box::pin(async move {
                            ::behaviortree_rs::sync::spawn_blocking(|| <#item_ident as ::behaviortree_rs::nodes::SyncHalt>::halt(self)).await
                        })
                    }
                }
            });
        }
        _ => unreachable!(),
    }

    let extra_fields = proc_macro2::TokenStream::new()
        .concat_list(default_fields)
        .concat_list(manual_fields);

    let output = quote! {
        #user_attrs
        #[derive(#derives)]
        #vis struct #item_ident #struct_fields

        impl #item_ident {
            pub fn new(name: impl AsRef<str>, config: ::behaviortree_rs::nodes::NodeConfig, #manual_fields_with_types) -> #item_ident {
                Self {
                    name: name.as_ref().to_string(),
                    config,
                    status: ::behaviortree_rs::basic_types::NodeStatus::Idle,
                    #extra_fields
                }
            }
        }

        #extra_impls
    };

    Ok(output)
}

/// Macro used to automatically generate the default boilerplate needed for all `TreeNode`s.
///
/// # Basic Usage
///
/// To use the macro, you need to add `#[bt_node(...)]` above your struct. As an argument
/// to the attribute, specify the NodeType that you would like to implement.
///
/// Supported options:
/// - `SyncActionNode`
/// - `StatefulActionNode`
/// - `ControlNode`
/// - `DecoratorNode`
///
/// By default, the tick method implementation is `async`. To specify this explicitly (or
/// make it synchronous), add `Async` or `Sync` after the node type.
///
/// ===
///
/// ```rust
/// use behaviortree_rs::{bt_node, basic_types::NodeStatus, nodes::{AsyncTick, NodeResult, AsyncHalt, NodePorts}, sync::BoxFuture};
///
/// // Here we are specifying a `SyncActionNode` as the node type.
/// #[bt_node(SyncActionNode)]
/// // Defaults to #[bt_node(SyncActionNode, Async)]
/// struct MyActionNode {} // No additional fields
///
/// // Now I need to `impl TreeNode`
/// impl AsyncTick for MyActionNode {
///     fn tick(&mut self) -> BoxFuture<NodeResult> {
///         Box::pin(async move {
///             // Do something here
///             // ...
///
///             Ok(NodeStatus::Success)
///         })
///     }
/// }
///
/// impl NodePorts for MyActionNode {}
///
/// // Also need to `impl NodeHalt`
/// // However, we'll just use the default implementation
/// impl AsyncHalt for MyActionNode {}
/// ```
///
/// ===
///
/// The above code will add fields to `MyActionNode` and create a `new()` associated method:
///
/// ```ignore
/// impl DummyActionNode {
///     pub fn new(name: impl AsRef<str>, config: NodeConfig) -> DummyActionNode {
///         Self {
///             name: name.as_ref().to_string(),
///             config,
///             status: NodeStatus::Idle
///         }
///     }
/// }
/// ```
///
/// # Adding Fields
///
/// When you add your own fields into the struct, be default they will be added
/// to the `new()` definition as arguments. To specify default values, use
/// the `#[bt(default)]` attribute above the fields.
///
/// `#[bt(default)]` will use the type's implementation of the `Default` trait. If
/// the trait isn't implemented on the type, or if you want to manually specify
/// a value, use `#[bt(default = "...")]`, where `...` is the value.
///
/// Valid argument types within the `"..."` are:
///
/// ```ignore
/// // Function calls
/// #[bt(default = "String::from(10)")]
///
/// // Variables
/// #[bt(default = "foo")]
///
/// // Paths (like enums)
/// #[bt(default = "NodeStatus::Idle")]
///
/// // Literals
/// #[bt(default = "10")]
/// ```
///
/// ## Example
///
/// ```rust
/// use behaviortree_rs::{bt_node, basic_types::NodeStatus, nodes::{AsyncTick, NodePorts, NodeResult, AsyncHalt}, sync::BoxFuture};
///
/// #[bt_node(SyncActionNode)]
/// struct MyActionNode {
///     #[bt(default = "NodeStatus::Success")]
///     foo: NodeStatus,
///     #[bt(default)] // defaults to empty String
///     bar: String
/// }
///
/// // Now I need to `impl TreeNode`
/// impl AsyncTick for MyActionNode {
///     fn tick(&mut self) -> BoxFuture<NodeResult> {
///         Box::pin(async move {
///             Ok(NodeStatus::Success)
///         })
///     }
/// }
///
/// impl NodePorts for MyActionNode {}
///
/// impl AsyncHalt for MyActionNode {}
/// ```
#[proc_macro_attribute]
pub fn bt_node(args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as ItemStruct);

    create_bt_node(args, item)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_derive(TreeNodeDefaults)]
/// Test docstring
pub fn derive_tree_node(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let ident = input.ident;

    let expanded = quote! {
        impl ::behaviortree_rs::nodes::TreeNodeDefaults for #ident {
            fn name(&self) -> &String {
                &self.name
            }

            fn path(&self) -> &String {
                &self.config.path
            }

            fn status(&self) -> ::behaviortree_rs::basic_types::NodeStatus {
                self.status.clone()
            }

            fn reset_status(&mut self) {
                self.status = ::behaviortree_rs::basic_types::NodeStatus::Idle
            }

            fn set_status(&mut self, status: ::behaviortree_rs::basic_types::NodeStatus) {
                self.status = status;
            }

            fn config(&self) -> &::behaviortree_rs::nodes::NodeConfig {
                &self.config
            }

            fn config_mut(&mut self) -> &mut ::behaviortree_rs::nodes::NodeConfig {
                &mut self.config
            }

            fn into_boxed(self) -> Box<dyn ::behaviortree_rs::nodes::TreeNodeBase> {
                Box::new(self)
            }
        }

        impl ::behaviortree_rs::nodes::TreeNodeBase for #ident {}
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(ActionNode)]
/// Test docstring
pub fn derive_action_node(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let ident = input.ident;

    let expanded = quote! {
        impl ::behaviortree_rs::nodes::ActionNode for #ident {
            fn execute_action_tick(&mut self) -> ::behaviortree_rs::sync::BoxFuture<::behaviortree_rs::NodeResult> {
                ::std::boxed::Box::pin(async move {
                    match self.tick().await? {
                        ::behaviortree_rs::basic_types::NodeStatus::Idle => Err(::behaviortree_rs::nodes::NodeError::StatusError(self.config.path.clone(), "Idle".to_string())),
                        status => Ok(status)
                    }
                })
            }
        }

        impl ::behaviortree_rs::nodes::ActionNodeBase for #ident {}

        impl ::behaviortree_rs::nodes::GetNodeType for #ident {
            fn node_type(&self) -> ::behaviortree_rs::basic_types::NodeType {
                ::behaviortree_rs::basic_types::NodeType::Action
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(ControlNode)]
pub fn derive_control_node(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let ident = input.ident;

    let expanded = quote! {
        impl ::behaviortree_rs::nodes::ControlNode for #ident {
            fn add_child(&mut self, child: ::behaviortree_rs::nodes::TreeNodePtr) {
                self.children.push(child);
            }

            fn children(&self) -> &Vec<::behaviortree_rs::nodes::TreeNodePtr> {
                &self.children
            }

            fn halt_child(&mut self, index: usize) -> ::behaviortree_rs::sync::BoxFuture<Result<(), ::behaviortree_rs::nodes::NodeError>> {
                ::std::boxed::Box::pin(async move {
                    match self.children.get_mut(index) {
                        Some(child) => {
                            if child.status() == ::behaviortree_rs::nodes::NodeStatus::Running {
                                let child_ptr: *mut _ = &mut **child;
                                unsafe {
                                    ::behaviortree_rs::nodes::AsyncHalt::halt(&mut *child_ptr).await;
                                }
                            }
                            Ok(child.reset_status())
                        }
                        None => Err(::behaviortree_rs::nodes::NodeError::IndexError),
                    }
                })
            }

            fn halt_children(&mut self, start: usize) -> ::behaviortree_rs::sync::BoxFuture<Result<(), ::behaviortree_rs::nodes::NodeError>> {
                ::std::boxed::Box::pin(async move {

                    if start >= self.children.len() {
                        return Err(::behaviortree_rs::nodes::NodeError::IndexError);
                    }

                    let end = self.children.len();

                    for i in start..end {
                        self.halt_child(i).await?;
                    }

                    Ok(())
                })
            }

            fn reset_children(&mut self) -> ::behaviortree_rs::sync::BoxFuture<()> {
                ::std::boxed::Box::pin(async move {
                    self.halt_children(0).await.unwrap();
                })
            }
        }

        impl ::behaviortree_rs::nodes::ExecuteTick for #ident {
            fn execute_tick(&mut self) -> ::behaviortree_rs::sync::BoxFuture<::behaviortree_rs::NodeResult> {
                ::std::boxed::Box::pin(async move {
                    ::log::debug!("[behaviortree_rs]: {}::tick()", <Self as ::behaviortree_rs::nodes::TreeNodeDefaults>::path(self));
                    <Self as ::behaviortree_rs::nodes::AsyncTick>::tick(self).await
                })
            }
        }

        impl ::behaviortree_rs::nodes::ControlNodeBase for #ident {}

        impl ::behaviortree_rs::nodes::GetNodeType for #ident {
            fn node_type(&self) -> ::behaviortree_rs::basic_types::NodeType {
                ::behaviortree_rs::basic_types::NodeType::Control
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(DecoratorNode)]
pub fn derive_decorator_node(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let ident = input.ident;

    let expanded = quote! {
        impl ::behaviortree_rs::nodes::DecoratorNode for #ident {
            fn set_child(&mut self, child: ::behaviortree_rs::nodes::TreeNodePtr) {
                self.child = Some(child);
            }

            fn child(&self) -> Result<&::behaviortree_rs::nodes::TreeNodePtr, ::behaviortree_rs::nodes::NodeError> {
                match &self.child {
                    Some(child) => Ok(child),
                    None => Err(::behaviortree_rs::nodes::NodeError::ChildMissing)
                }
            }

            fn halt_child(&mut self) -> BoxFuture<()> {
                ::std::boxed::Box::pin(async move {
                    self.reset_child().await;
                })
            }

            fn reset_child(&mut self) -> BoxFuture<()> {
                ::std::boxed::Box::pin(async move {
                    if let Some(child) = self.child.as_mut() {
                        let mut child = child;
                        if matches!(child.status(), ::behaviortree_rs::basic_types::NodeStatus::Running) {
                            let child_ptr: *mut _ = &mut **child;
                            unsafe {
                                ::behaviortree_rs::nodes::AsyncHalt::halt(&mut *child_ptr).await;
                            }
                        }

                        child.reset_status();
                    }
                })
            }
        }

        impl ::behaviortree_rs::nodes::ExecuteTick for #ident {
            fn execute_tick(&mut self) -> ::behaviortree_rs::sync::BoxFuture<::behaviortree_rs::NodeResult> {
                ::std::boxed::Box::pin(async move {
                    if self.child.is_none() {
                        return Err(::behaviortree_rs::nodes::NodeError::ChildMissing);
                    }

                    ::log::debug!("[behaviortree_rs]: {}::tick()", <Self as ::behaviortree_rs::nodes::TreeNodeDefaults>::name(self));
                    self.tick().await
                })
            }
        }

        impl ::behaviortree_rs::nodes::DecoratorNodeBase for #ident {}

        impl ::behaviortree_rs::nodes::GetNodeType for #ident {
            fn node_type(&self) -> ::behaviortree_rs::basic_types::NodeType {
                ::behaviortree_rs::basic_types::NodeType::Decorator
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(SyncActionNode)]
/// Test docstring
pub fn derive_sync_action_node(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let ident = input.ident;

    let expanded = quote! {
        impl ::behaviortree_rs::nodes::ExecuteTick for #ident {
            fn execute_tick(&mut self) -> ::behaviortree_rs::sync::BoxFuture<::behaviortree_rs::NodeResult> {
                ::std::boxed::Box::pin(async move {
                    ::log::debug!("[behaviortree_rs]: {}::tick()", <Self as ::behaviortree_rs::nodes::TreeNodeDefaults>::name(self));
                    match <Self as ::behaviortree_rs::nodes::ActionNode>::execute_action_tick(self).await? {
                        ::behaviortree_rs::basic_types::NodeStatus::Running => Err(::behaviortree_rs::nodes::NodeError::StatusError(self.config.path.clone(), "Running".to_string())),
                        status => Ok(status)
                    }
                })
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(StatefulActionNode)]
/// Test docstring
pub fn derive_stateful_action_node(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let ident = input.ident;

    let expanded = quote! {
        impl ::behaviortree_rs::nodes::ExecuteTick for #ident where #ident: ::behaviortree_rs::nodes::AsyncStatefulActionNode {
            fn execute_tick(&mut self) -> ::behaviortree_rs::sync::BoxFuture<::behaviortree_rs::NodeResult> {
                ::std::boxed::Box::pin(async move {
                    let prev_status = <Self as ::behaviortree_rs::nodes::TreeNodeDefaults>::status(self);

                    let new_status = match prev_status {
                        ::behaviortree_rs::basic_types::NodeStatus::Idle => {
                            ::log::debug!("[behaviortree_rs]: {}::on_start()", <Self as ::behaviortree_rs::nodes::TreeNodeDefaults>::path(self));
                            let new_status = ::behaviortree_rs::nodes::action::AsyncStatefulActionNode::on_start(self).await?;
                            if matches!(new_status, ::behaviortree_rs::basic_types::NodeStatus::Idle) {
                                return Err(::behaviortree_rs::nodes::NodeError::StatusError(format!("{}::on_start()", self.config.path), "Idle".to_string()))
                            }
                            new_status
                        }
                        ::behaviortree_rs::basic_types::NodeStatus::Running => {
                            ::log::debug!("[behaviortree_rs]: {}::on_running()", <Self as ::behaviortree_rs::nodes::TreeNodeDefaults>::path(self));
                            let new_status = ::behaviortree_rs::nodes::action::AsyncStatefulActionNode::on_running(self).await?;
                            if matches!(new_status, ::behaviortree_rs::basic_types::NodeStatus::Idle) {
                                return Err(::behaviortree_rs::nodes::NodeError::StatusError(format!("{}::on_running()", self.config.path), "Idle".to_string()))
                            }
                            new_status
                        }
                        prev_status => prev_status
                    };

                    <Self as ::behaviortree_rs::nodes::TreeNodeDefaults>::set_status(self, new_status.clone());

                    Ok(new_status)
                })
            }
        }

        impl ::behaviortree_rs::nodes::AsyncHalt for #ident {
            fn halt(&mut self) -> ::behaviortree_rs::sync::BoxFuture<()> {
                ::std::boxed::Box::pin(async move {
                    self.halt_requested = true;

                    if matches!(<Self as ::behaviortree_rs::nodes::TreeNodeDefaults>::status(self), ::behaviortree_rs::basic_types::NodeStatus::Running) {
                        self.on_halted().await;
                    }

                    <Self as ::behaviortree_rs::nodes::TreeNodeDefaults>::reset_status(self);
                })
            }
        }
    };

    TokenStream::from(expanded)
}

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
        // let node_name = input
        //     .peek(syn::LitStr)
        //     .then_some(input.parse::<LitStr>()?.to_token_stream())
        //     .or_else(|| input.peek(syn::Ident).then_some(input.parse::<syn::Ident>().unwrap().to_token_stream()))
        //     .or_else(|| input.peek(syn::Path).then_some(input.parse::<syn::Ident>().unwrap().to_token_stream()));
        let node_name = input.parse::<syn::Expr>()?.to_token_stream();
        // if node_name.is_none() {
        //     return Err(input.error("Node name must either be a string literal or an ident"));
        // }
        // let node_name = node_name.unwrap();
        
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
        params
    } = node;

    let cloned_names = (0..params.len())
        .fold(quote!{}, |acc, i| {
            let arg_name = Ident::new(&format!("arg{i}"), Span::call_site());
            quote!{ #acc, #arg_name.clone() }
        });

    quote! {
        {
            let mut node = #node_type::new(#name, config #cloned_names);
            let manifest = ::behaviortree_rs::basic_types::TreeNodeManifest {
                node_type: <#node_type as ::behaviortree_rs::nodes::GetNodeType>::node_type(&node),
                registration_id: #name.into(),
                ports: <#node_type as ::behaviortree_rs::nodes::NodePorts>::provided_ports(&node),
                description: ::std::string::String::new(),
            };
            <#node_type as ::behaviortree_rs::nodes::TreeNodeDefaults>::config_mut(&mut node).set_manifest(::std::sync::Arc::new(manifest));
            node
        }
    }
}

fn register_node(input: TokenStream, node_type_token: proc_macro2::TokenStream, node_type: NodeTypeInternal) -> TokenStream {
    let node_registration = parse_macro_input!(input as NodeRegistration);

    let factory = &node_registration.factory;
    let name = &node_registration.name;
    let params = &node_registration.params;

    // Create expression that clones all parameters
    let param_clone_expr = params
        .iter()
        .enumerate()
        .fold(quote!{}, |acc, (i, item)| {
            let arg_name = Ident::new(&format!("arg{i}"), Span::call_site());
            quote! {
                #acc
                let #arg_name = #item.clone();
            }
        });

    let node = build_node(&node_registration);

    let extra_steps = match node_type {
        NodeTypeInternal::Control => quote! { 
            for child in children {
                node.children.push(child);
            }
        },
        NodeTypeInternal::Decorator => quote! { node.child = Some(children.remove(0)); },
        _ => quote!{ }
    };

    let expanded = quote! {
        {
            let blackboard = #factory.blackboard().clone();

            #param_clone_expr

            let node_fn = move |
                config: ::behaviortree_rs::nodes::NodeConfig,
                mut children: ::std::vec::Vec<::std::boxed::Box<dyn ::behaviortree_rs::nodes::TreeNodeBase + Send + Sync>>
            | -> ::std::boxed::Box<dyn ::behaviortree_rs::nodes::TreeNodeBase + Send + Sync>
            {
                let mut node = #node;
                
                #extra_steps

                ::std::boxed::Box::new(node)
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
    register_node(input, quote! { ::behaviortree_rs::basic_types::NodeType::Action }, NodeTypeInternal::Action)
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
    register_node(input, quote! { ::behaviortree_rs::basic_types::NodeType::Control }, NodeTypeInternal::Control)
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
    register_node(input, quote! { ::behaviortree_rs::basic_types::NodeType::Decorator }, NodeTypeInternal::Decorator)
}
