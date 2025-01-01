use ez_bencoding::BdecodeNode;

fn main() {
        // {"\x04b": "v\x02", "k2": {"k3": "v3", "k4": 9}, "k5": [7, 8], "k6": "v6"}
        let buffer = b"d 2:\x04b 2:v\x02 2:k2 d 2:k3 2:v3 2:k4 i9e e 2:k5 l i7e i8e e 2:k6 2:v6 e"
            .into_iter()
            .filter(|v| {
                **v != b' '
            })
            .cloned()
            .collect::<Vec<_>>();

    let root_node = BdecodeNode::parse_buffer(buffer.into()).unwrap();
    println!("{}", &root_node.to_json_pretty());

    let k5_node = root_node.dict_find(b"k5").unwrap();
    println!("{}", &k5_node.to_json());
   
    for i in 0..k5_node.len() {
        let val = k5_node.list_item_as_int(i).unwrap();
        println!("item_{} = {}", i, val)
    }
}