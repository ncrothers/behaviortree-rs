use proc_macro::TokenStream;
use syn::DeriveInput;

#[macro_use]
extern crate quote;
#[macro_use]
extern crate syn;

extern crate proc_macro;

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

            fn config(&mut self) -> &mut ::bt_cpp_rust::nodes::NodeConfig {
                &mut self.config
            }

            fn into_boxed(self) -> Box<dyn ::bt_cpp_rust::nodes::TreeNodeBase> {
                Box::new(self)
            }

            fn into_tree_node_ptr(&self) -> ::bt_cpp_rust::nodes::TreeNodePtr {
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
        }

        impl ::bt_cpp_rust::nodes::ActionNodeBase for #ident {}

        impl ::bt_cpp_rust::nodes::GetNodeType for #ident {
            fn node_type(&self) -> ::bt_cpp_rust::basic_types::NodeType {
                ::bt_cpp_rust::basic_types::NodeType::Action
            }
        }

        impl ::bt_cpp_rust::nodes::NodeTick for #ident {
            fn execute_tick(&mut self) -> NodeStatus {
                self.tick()
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
            fn execute_tick(&mut self) -> NodeStatus {
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
