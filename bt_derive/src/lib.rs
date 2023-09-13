use proc_macro::TokenStream;
use syn::{DeriveInput, Item, token::Struct, parse::Parser, Attribute, ItemStruct};

#[macro_use]
extern crate quote;
#[macro_use]
extern crate syn;

extern crate proc_macro;

fn create_bt_node(args: TokenStream, input: TokenStream, mut item: ItemStruct) -> syn::Result<proc_macro2::TokenStream> {
    let args_parsed = syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated
        .parse(args)?;


    for arg in args_parsed.iter() {
        // match arg.is_ident("SyncActionNode") {
        //     true => compile_error!("I am a SyncActionNode!"),
        //     false => {}
        // }

        arg.require_ident()?;
    }

    match &mut item.fields {
        syn::Fields::Named(fields) => {
            fields.named.push(
                syn::Field::parse_named.parse2(quote! { pub config: ::bt_cpp_rust::nodes::NodeConfig }).unwrap()
            );
            fields.named.push(
                syn::Field::parse_named.parse2(quote! { pub status: ::bt_cpp_rust::basic_types::NodeStatus }).unwrap()
            );
        }
        _ => return Err(syn::Error::new_spanned(item, "expected a struct with named fields"))
    };

    let ident = item.ident.clone();

    let output = quote! {
        // #[derive(::bt_cpp_rust::derive::ActionNode, ::bt_cpp_rust::derive::SyncActionNode, ::std::fmt::Debug, Clone)]
        pub struct #ident {
            // pub config: ::bt_cpp_rust::nodes::NodeConfig,
            // pub status: ::bt_cpp_rust::basic_types::NodeStatus,
        }

        impl #ident {
            pub fn new() -> #ident {
                Self {

                }
            }
        }

        // impl ::bt_cpp_rust::nodes::TreeNodeDefaults for #ident {
        //     fn status(&self) -> ::bt_cpp_rust::basic_types::NodeStatus {
        //         self.status.clone()
        //     }

        //     fn reset_status(&mut self) {
        //         self.status = ::bt_cpp_rust::basic_types::NodeStatus::Idle
        //     }

        //     fn set_status(&mut self, status: ::bt_cpp_rust::basic_types::NodeStatus) {
        //         self.status = status;
        //     }

        //     fn config(&mut self) -> &mut ::bt_cpp_rust::nodes::NodeConfig {
        //         &mut self.config
        //     }

        //     fn into_boxed(self) -> Box<dyn ::bt_cpp_rust::nodes::TreeNodeBase> {
        //         Box::new(self)
        //     }

        //     fn to_tree_node_ptr(&self) -> ::bt_cpp_rust::nodes::TreeNodePtr {
        //         std::rc::Rc::new(std::cell::RefCell::new(self.clone()))
        //     }

        //     fn clone_node_boxed(&self) -> Box<dyn ::bt_cpp_rust::nodes::TreeNodeBase> {
        //         Box::new(self.clone())
        //     }
        // }

        // impl ::bt_cpp_rust::nodes::TreeNodeBase for #ident {}
    };


    Ok(output)
}

#[proc_macro_attribute]
pub fn bt_node(args: TokenStream, input: TokenStream) -> TokenStream {
    let input_cloned = input.clone();
    let mut item = parse_macro_input!(input_cloned as ItemStruct);



    // parse_args

    create_bt_node(args, input, item).unwrap_or_else(syn::Error::into_compile_error).into()
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
