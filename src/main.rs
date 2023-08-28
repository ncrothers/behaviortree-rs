use bt_cpp_rust::{basic_types::{BTToString, NodeStatus, StringInto, PortDirection, PortInfo}, blackboard::Blackboard};

struct Test;

fn main() {
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
