use proc_macro::TokenStream;
use proc_macro2::Ident;
use syn::{DeriveInput, Item, token::{Struct, Comma}, parse::Parser, Attribute, ItemStruct, punctuated::Punctuated, AttrStyle};

#[macro_use]
extern crate quote;
#[macro_use]
extern crate syn;

extern crate proc_macro;

trait ToMap<T, K, V> {
    fn to_map(&self) -> syn::Result<std::collections::HashMap<K, V>>;
}

impl ToMap<Punctuated<syn::Meta, Comma>, syn::Ident, Option<proc_macro2::TokenStream>> for Punctuated<syn::Meta, Comma> {
    /// Convert a list of attribute arguments to a HashMap
    fn to_map(&self) -> syn::Result<std::collections::HashMap<syn::Ident, Option<proc_macro2::TokenStream>>> {
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
                                else {
                                    return Err(syn::Error::new_spanned(&arg.value, "argument value should be a:  variable, literal, function call"))
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
    fn concat(&self, value: proc_macro2::TokenStream) -> proc_macro2::TokenStream;
}

impl ConcatTokenStream for proc_macro2::TokenStream {
    fn concat(&self, value: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        if self.is_empty() {
            if value.is_empty() {
                // Both are empty
                proc_macro2::TokenStream::new()
            }
            else {
                // self empty, value not empty
                quote! {
                    #value
                }
            }
        } 
        else if value.is_empty() {
            // self not empty, value empty
            quote! {
                #self
            }
        }
        else {
            // Both have value
            quote! {
                #self,
                #value
            }
        }
    }
}

fn create_bt_node(args: TokenStream, mut item: ItemStruct) -> syn::Result<proc_macro2::TokenStream> {
    let args_parsed = syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated
        .parse(args)?;

    let mut derives = vec![quote! { Clone, ::std::fmt::Debug, ::bt_cpp_rust::derive::TreeNodeDefaults }];

    for arg in args_parsed.iter() {
        // Require parameter to be ident, no prefix path
        arg.require_ident()?;
        
        let ident = arg.get_ident().unwrap().to_string();

        // Match all possible node types
        match ident.as_str() {
            "SyncActionNode" => derives.push(quote! { ::bt_cpp_rust::derive::ActionNode, ::bt_cpp_rust::derive::SyncActionNode }),
            "StatefulActionNode" => derives.push(quote! { ::bt_cpp_rust::derive::ActionNode, ::bt_cpp_rust::derive::StatefulActionNode }),
            "ControlNode" => derives.push(quote! { ::bt_cpp_rust::derive::ControlNode }),
            "DecoratorNode" => derives.push(quote! { ::bt_cpp_rust::derive::DecoratorNode }),
            _ => return Err(syn::Error::new_spanned(arg, "unsupported node type"))
        }
    }

    let mut default_fields = proc_macro2::TokenStream::new();
    let mut manual_fields = proc_macro2::TokenStream::new();
    let mut manual_fields_with_types = proc_macro2::TokenStream::new();

    match &mut item.fields {
        syn::Fields::Named(fields) => {
            for f in fields.named.iter_mut() {
                let name = f.ident.as_ref().unwrap();
                let ty = &f.ty;

                let mut used_default = false;
                for a in f.attrs.iter() {
                    if a.path().is_ident("bt") {

                        let args: Punctuated<syn::Meta, Comma> = a.parse_args_with(Punctuated::parse_terminated)?;
                        let args_map = args.to_map()?;

                        // If the default argument was included
                        if let Some(value) = args_map.get(&syn::parse_str("default")?) {
                            used_default = true;
                            // Use the provided default, if provided by user
                            let default_value = if let Some(default_value) = value {
                                quote!{ #default_value }
                            }
                            // Otherwise, use Default
                            else {
                                quote! { #ty::default() }
                            };

                            default_fields = default_fields.concat(quote! { #name: #default_value });
                        }
                    }
                }

                // Mark field as manually specified if 
                if !used_default {
                    manual_fields = manual_fields.concat(quote! { #name });
                    manual_fields_with_types = manual_fields_with_types.concat(quote! { #name: #ty });
                }
    
                // Remove the bt attribute, keep all others
                f.attrs = f.attrs.clone().into_iter().filter(|a| !a.path().is_ident("bt")).collect();
            }

            // user_fields = fields.named.clone();
            // user_field_names = user_fields.iter().map(|f| f.ident.as_ref().unwrap()).collect();

            fields.named.push(
                syn::Field::parse_named.parse2(quote! { pub config: ::bt_cpp_rust::nodes::NodeConfig }).unwrap()
            );
            fields.named.push(
                syn::Field::parse_named.parse2(quote! { pub status: ::bt_cpp_rust::basic_types::NodeStatus }).unwrap()
            );
        }
        _ => return Err(syn::Error::new_spanned(item, "expected a struct with named fields"))
    };

    let mut user_attrs = Vec::new();

    for attr in item.attrs.iter() {
        if attr.path().is_ident("derive") {
            derives.push(attr.parse_args()?);
        }
        else if let AttrStyle::Outer = attr.style {
            user_attrs.push(attr);
        }
    }

    let user_attrs = user_attrs.into_iter().fold(proc_macro2::TokenStream::new(), |acc, a| {
        // Only want to transfer outer attributes
        if let AttrStyle::Outer = a.style {
            if acc.is_empty() {
                quote! {
                    #a
                }
            }
            else {
                quote! {
                    #acc
                    #a
                }
            }
        }
        else {
            acc
        }
    });

    // Convert Vec of derive Paths into one TokenStream
    let derives = derives.into_iter().fold(proc_macro2::TokenStream::new(), |acc, d| {
        if acc.is_empty() {
            quote! {
                #d
            }
        }
        else {
            quote! {
                #acc, #d
            }
        }
    });

    let ident = &item.ident;
    let vis = &item.vis;
    let struct_fields = &item.fields;

    let extra_fields = proc_macro2::TokenStream::new().concat(default_fields).concat(manual_fields);

    let output = quote! {
        #user_attrs
        #[derive(#derives)]
        #vis struct #ident #struct_fields

        impl #ident {
            fn new(config: ::bt_cpp_rust::nodes::NodeConfig, #manual_fields_with_types) -> #ident {
                Self {
                    config,
                    status: ::bt_cpp_rust::basic_types::NodeStatus::Idle,
                    #extra_fields
                }
            }
        }
    };

    Ok(output)
}

#[proc_macro_attribute]
pub fn bt_node(args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as ItemStruct);

    create_bt_node(args, item).unwrap_or_else(syn::Error::into_compile_error).into()
}

#[proc_macro_derive(TreeNodeDefaults)]
/// Test docstring
pub fn derive_tree_node(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let ident = input.ident;

    let expanded = quote! {
        impl ::bt_cpp_rust::nodes::TreeNodeDefaults for #ident {
            fn status(&self) -> ::bt_cpp_rust::basic_types::NodeStatus {
                self.status.clone()
            }

            fn reset_status(&mut self) {
                self.status = ::bt_cpp_rust::basic_types::NodeStatus::Idle
            }

            fn set_status(&mut self, status: ::bt_cpp_rust::basic_types::NodeStatus) {
                self.status = status;
            }

            fn config(&mut self) -> &mut ::bt_cpp_rust::nodes::NodeConfig {
                &mut self.config
            }

            fn into_boxed(self) -> Box<dyn ::bt_cpp_rust::nodes::TreeNodeBase> {
                Box::new(self)
            }

            fn to_tree_node_ptr(&self) -> ::bt_cpp_rust::nodes::TreeNodePtr {
                std::rc::Rc::new(std::cell::RefCell::new(self.clone()))
            }

            fn clone_node_boxed(&self) -> Box<dyn ::bt_cpp_rust::nodes::TreeNodeBase> {
                Box::new(self.clone())
            }
        }

        impl ::bt_cpp_rust::nodes::TreeNodeBase for #ident {}
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(ActionNode)]
/// Test docstring
pub fn derive_action_node(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let ident = input.ident;

    let expanded = quote! {
        impl ::bt_cpp_rust::nodes::ActionNode for #ident {
            fn clone_boxed(&self) -> Box<dyn ::bt_cpp_rust::nodes::ActionNodeBase> {
                Box::new(self.clone())
            }

            fn execute_action_tick(&mut self) -> Result<::bt_cpp_rust::basic_types::NodeStatus, ::bt_cpp_rust::nodes::NodeError> {
                match self.tick()? {
                    ::bt_cpp_rust::basic_types::NodeStatus::Idle => Err(::bt_cpp_rust::nodes::NodeError::StatusError(self.config.path.clone(), "Idle".to_string())),
                    status => Ok(status)
                }
            }
        }

        impl ::bt_cpp_rust::nodes::ActionNodeBase for #ident {}

        impl ::bt_cpp_rust::nodes::GetNodeType for #ident {
            fn node_type(&self) -> ::bt_cpp_rust::basic_types::NodeType {
                ::bt_cpp_rust::basic_types::NodeType::Action
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
        impl ::bt_cpp_rust::nodes::ControlNode for #ident {
            fn add_child(&mut self, child: ::bt_cpp_rust::nodes::TreeNodePtr) {
                self.children.push(child);
            }

            fn children(&self) -> &Vec<TreeNodePtr> {
                &self.children
            }

            fn halt_control(&mut self) {
                self.reset_children();
            }

            fn halt_child(&self, index: usize) -> Result<(), ::bt_cpp_rust::nodes::NodeError> {
                match self.children.get(index) {
                    Some(child) => {
                        if child.borrow().status() == NodeStatus::Running {
                            child.borrow_mut().halt();
                        }
                        Ok(child.borrow_mut().reset_status())
                    }
                    None => Err(::bt_cpp_rust::nodes::NodeError::IndexError),
                }
            }

            fn halt_children(&self, start: usize) -> Result<(), ::bt_cpp_rust::nodes::NodeError> {
                if start >= self.children.len() {
                    return Err(::bt_cpp_rust::nodes::NodeError::IndexError);
                }

                let end = self.children.len();

                for i in start..end {
                    self.halt_child(i)?;
                }

                Ok(())
            }

            fn reset_children(&self) {
                self.halt_children(0).unwrap();
            }

            fn clone_boxed(&self) -> Box<dyn ::bt_cpp_rust::nodes::ControlNodeBase> {
                Box::new(self.clone())
            }
        }

        impl ::bt_cpp_rust::nodes::NodeTick for #ident {
            fn execute_tick(&mut self) -> Result<::bt_cpp_rust::basic_types::NodeStatus, ::bt_cpp_rust::nodes::NodeError> {
                self.tick()
            }
        }

        impl ::bt_cpp_rust::nodes::ControlNodeBase for #ident {}

        impl ::bt_cpp_rust::nodes::GetNodeType for #ident {
            fn node_type(&self) -> ::bt_cpp_rust::basic_types::NodeType {
                ::bt_cpp_rust::basic_types::NodeType::Control
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
        impl ::bt_cpp_rust::nodes::DecoratorNode for #ident {
            fn set_child(&mut self, child: ::bt_cpp_rust::nodes::TreeNodePtr) {
                self.child = Some(child);
            }

            fn child(&self) -> Result<&::bt_cpp_rust::nodes::TreeNodePtr, ::bt_cpp_rust::nodes::NodeError> {
                match &self.child {
                    Some(child) => Ok(child),
                    None => Err(::bt_cpp_rust::nodes::NodeError::ChildMissing)
                }
            }

            fn halt_decorator(&mut self) {
                self.reset_child();
            }

            fn halt_child(&self) {
                self.reset_child();
            }

            fn reset_child(&self) {
                if let Some(child) = self.child.as_ref() {
                    if matches!(child.borrow().status(), ::bt_cpp_rust::basic_types::NodeStatus::Running) {
                        child.borrow_mut().halt();
                    }
    
                    child.borrow_mut().reset_status();
                }
            }

            fn clone_boxed(&self) -> Box<dyn ::bt_cpp_rust::nodes::DecoratorNodeBase> {
                Box::new(self.clone())
            }
        }

        impl ::bt_cpp_rust::nodes::NodeTick for #ident {
            fn execute_tick(&mut self) -> Result<::bt_cpp_rust::basic_types::NodeStatus, ::bt_cpp_rust::nodes::NodeError> {
                if self.child.is_none() {
                    return Err(::bt_cpp_rust::nodes::NodeError::ChildMissing);
                }
                
                self.tick()
            }
        }

        impl ::bt_cpp_rust::nodes::DecoratorNodeBase for #ident {}

        impl ::bt_cpp_rust::nodes::GetNodeType for #ident {
            fn node_type(&self) -> ::bt_cpp_rust::basic_types::NodeType {
                ::bt_cpp_rust::basic_types::NodeType::Decorator
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
        impl ::bt_cpp_rust::nodes::NodeTick for #ident {
            fn execute_tick(&mut self) -> Result<::bt_cpp_rust::basic_types::NodeStatus, ::bt_cpp_rust::nodes::NodeError> {
                match <Self as ::bt_cpp_rust::nodes::ActionNode>::execute_action_tick(self)? {
                    ::bt_cpp_rust::basic_types::NodeStatus::Running => Err(::bt_cpp_rust::nodes::NodeError::StatusError(self.config.path.clone(), "Running".to_string())),
                    status => Ok(status)
                }
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
        impl ::bt_cpp_rust::nodes::NodeTick for #ident where #ident: ::bt_cpp_rust::nodes::StatefulActionNode {
            fn execute_tick(&mut self) -> Result<::bt_cpp_rust::basic_types::NodeStatus, ::bt_cpp_rust::nodes::NodeError> {
                let prev_status = <Self as ::bt_cpp_rust::nodes::TreeNodeDefaults>::status(self);

                let new_status = match prev_status {
                    ::bt_cpp_rust::basic_types::NodeStatus::Idle => {
                        let new_status = self.on_start()?;
                        if matches!(new_status, ::bt_cpp_rust::basic_types::NodeStatus::Idle) {
                            return Err(NodeError::StatusError(format!("{}::on_start()", self.config.path), "Idle".to_string()))
                        }
                        new_status
                    }
                    ::bt_cpp_rust::basic_types::NodeStatus::Running => {
                        let new_status = self.on_running()?;
                        if matches!(new_status, ::bt_cpp_rust::basic_types::NodeStatus::Idle) {
                            return Err(NodeError::StatusError(format!("{}::on_running()", self.config.path), "Idle".to_string()))
                        }
                        new_status
                    }
                    prev_status => prev_status
                };

                <Self as ::bt_cpp_rust::nodes::TreeNodeDefaults>::set_status(self, new_status.clone());

                Ok(new_status)
            }
        }

        impl ::bt_cpp_rust::nodes::NodeHalt for #ident {
            fn halt(&mut self) {
                *self.halt_requested.borrow_mut() = true;

                if matches!(<Self as ::bt_cpp_rust::nodes::TreeNodeDefaults>::status(self), ::bt_cpp_rust::basic_types::NodeStatus::Running) {
                    self.on_halted();
                }
            }
        }
    };

    TokenStream::from(expanded)
}

// Matching types

// for f in fields.iter() {
//     match &f.ty {
//         syn::Type::Path(p) => {
//             for seg in p.path.segments.iter() {
//                 match seg.ident.to_string().as_str() {
//                     // Check for children Vec
//                     "Vec" => {
//                         let args = match &seg.arguments {
//                             PathArguments::AngleBracketed(args) => args,
//                             _ => continue
//                         };
//                         let args: Vec<GenericArgument> = args.args.clone().into_iter().collect();
//                         let arg = &args[0];
//                         // Check for Box type
//                         let arg_ident = match arg {
//                             GenericArgument::Type(t) => {
//                                 match t {
//                                     Type::Path(p) => {
//                                         p.path.
//                                     }
//                                     _ => continue
//                                 }
//                             }
//                             _ => continue
//                         };
//                         match arg {
//                             GenericArgument::Type(t) => {
//                                 if let Type::TraitObject(to) = t {
//                                     for b in to.bounds.iter() {
//                                         if let TypeParamBound::Trait(tr) = b {
//                                             for t in tr.path.segments.iter() {
//                                                 if t.ident == "TreeNode" {
//                                                     let _ = children_ident.insert(t.ident.clone());
//                                                 }
//                                             }
//                                         }
//                                     }
//                                     // panic!("{}, {:?}", seg.ident.to_string(), to.bounds);
//                                 }
//                             }
//                             _ => {}
//                         };
//                     }
//                     _ => {}
//                 }
//                 if seg.ident == "NodeConfig" {
//                     let _ = config_name.insert(f.ident.as_ref().unwrap().clone());
//                 }
//             }
//         }
//         syn::Type::Group(g) => {
//             panic!("Group: {:?}", g.group_token.span);
//         }
//         syn::Type::Paren(t) => {
//             panic!("{:?}", t.paren_token.span);
//         }
//         _ => {}
//     }
// }
