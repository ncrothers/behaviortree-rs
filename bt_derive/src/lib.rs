use proc_macro::TokenStream;
use syn::{DeriveInput, PathArguments, GenericArgument, Type, TypeParamBound};

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

    let attributes = input.attrs;

    let fields = if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(syn::FieldsNamed {ref named, ..}),
        ..
    }) = input.data {
        named
    } else {
        panic!("You can derive only on struct!")
    };

    let expanded = quote! {
        impl TreeNodeDefaults for #ident {
            fn status(&self) -> NodeStatus {
                self.status.clone()
            }

            fn reset_status(&mut self) {
                self.status = NodeStatus::Idle
            }

            fn config(&mut self) -> &mut NodeConfig {
                &mut self.config
            }
        }

        impl TreeNodeBase for #ident {}
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(ActionNode)]
/// Test docstring
pub fn derive_action_node(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let ident = input.ident;

    let expanded = quote! {
        impl ActionNode for #ident {}

        impl GetNodeType for #ident {
            fn node_type(&self) -> NodeType {
                NodeType::Action
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(ControlNode)]
pub fn derive_control_node(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let ident = input.ident;

    let attributes = input.attrs;

    let fields = if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(syn::FieldsNamed {ref named, ..}),
        ..
    }) = input.data {
        named
    } else {
        panic!("You can derive only on struct!")
    };

    let expanded = quote! {
        impl ControlNode for #ident {
            fn add_child(&mut self, child: TreeNodePtr) {
                self.children.push(child);
            }

            fn children(&self) -> &Vec<TreeNodePtr> {
                &self.children
            }

            fn halt_child(&mut self, index: usize) -> Result<(), NodeError> {
                match self.children.get_mut(index) {
                    Some(child) => Ok(child.borrow_mut().halt()),
                    None => Err(NodeError::IndexError),
                }
            }

            fn halt_children(&mut self, start: usize) -> Result<(), NodeError> {
                if start >= self.children.len() {
                    return Err(NodeError::IndexError);
                }
        
                self.children[start..].iter_mut().for_each(|child| child.borrow_mut().halt());
                Ok(())
            }

            fn reset_children(&mut self) {
                self
                    .children
                    .iter_mut()
                    .for_each(|child| child.borrow_mut().reset_status());
            }
        }

        impl GetNodeType for #ident {
            fn node_type(&self) -> NodeType {
                NodeType::Control
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