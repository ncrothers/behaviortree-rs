use bt_cpp_rust::{basic_types::{BTToString, NodeStatus, StringInto, PortDirection, PortInfo}, blackboard::Blackboard, tree::{DummyNode, Node, Factory}, register_node, nodes::DummyLeafNode};
use quick_xml::{Reader, events::Event};

struct Test;

fn blackboard_test() {
    let status = NodeStatus::Success;
    println!("{}", status.into_string_color());
    let status = NodeStatus::Failure;
    println!("{}", status.into_string_color());
    let status = NodeStatus::Idle;
    println!("{}", status.into_string_color());
    
    // let dir = PortDirection::Input;
    // let port_info = PortInfo::new(PortDirection::Input, "hello");
    
    let mut blackboard = Blackboard::new();
    
    blackboard.write("test", "foo");
    blackboard.write("test int", 500u64);
    blackboard.write("test custom", status);
    blackboard.write("test string_into", "1;2;3;4");
    
    let val = blackboard.read::<String>("test");
    println!("{:?}", val.unwrap());
    let val = blackboard.read::<u64>("test int");
    println!("{:?}", val.unwrap());
    let val = blackboard.read::<NodeStatus>("test custom");
    println!("{:?}", val.unwrap());
    let val = blackboard.read::<Vec<String>>("test string_into");
    println!("{:?}", val.unwrap());
}

// fn xml_test() {
//     let text = std::fs::read_to_string("./test.xml").unwrap();

//     println!("{}", &text);

//     let value: bt_cpp_rust::tree::SequenceNode = quick_xml::de::from_str(text.as_str()).unwrap();
//     println!("{value:?}");

//     let node = bt_cpp_rust::tree::SequenceNode {
//         children: vec![
//             Node::DummyNode(DummyNode {
//                 value: String::from("hello")
//             })
//         ]
//     };

//     let value = quick_xml::se::to_string(&node).unwrap();
//     println!("{value}");
// }



fn main() {
    let text = std::fs::read_to_string("./test.xml").unwrap();
    let mut factory = Factory::new();

    register_node!(factory, "DummyNode", DummyLeafNode);
    register_node!(factory, "CustomNode", DummyLeafNode);

    let mut tree = match factory.parse_xml(text) {
        Ok(tree) => tree,
        Err(e) => {
            println!("Error: {e}");
            panic!("");
        }
    };
    println!("{tree:?}");
    
    let status = tree.tick_while_running();
    println!("{status:?}");

}
