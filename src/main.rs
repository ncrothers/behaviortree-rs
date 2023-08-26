use bt_cpp_rust::TryToVec;

fn main() {
    println!("Hello, world!");
    // let result: Vec<f32> = "1;2;3".try_to_vec().unwrap();
    let result: bool = "TRUE".parse().unwrap();
    println!("{:?}", result);
}
