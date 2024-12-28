use ez_bencoding::BdecodeNode;

fn main() {
    let buf = "d 2:k1 2:v1 2:k2 l i1e i2e e 3:k03 i3e 2:k4 d 2:k5 i5e 2:k6 i6e e e".replace(" ", "");

    let root_node = BdecodeNode::parse_buffer(buf.into()).unwrap();
    println!("{}", root_node.to_json());

    let k2_node = root_node.dict_find(b"k2").unwrap();
    println!("{}", k2_node.to_json());
   
    for i in 0..k2_node.len() {
        let val = k2_node.list_item_as_int(i).unwrap();
        println!("item_{} = {}", i, val)
    }
}