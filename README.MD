# ez-bencoding

该项目是受到 libtorrent 第三版的 bdecoding 启发，使用 token 对 bencoded 字符串进行解析，由于避免了频繁的内存 alloc 和 dealloc 操作，所以速度快，内存占用小。
另外，本库还支持在多线程环境下使用，对 BdecodeNode 对象进行 clone() 的操作成本是很低的。

样例：

```rust
use ez_bencoding::BdecodeNode;

fn main() {
    // {"k1": "v1", "k2": [1, 2], "k03": 3, "k4": {"k5": 5, "k6": 6}}
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
```

显示结果：

```console
{ "k1": "v1", "k2": [1, 2], "k03": 3, "k4": { "k5": 5, "k6": 6 } }
[1, 2]
item_0 = 1
item_1 = 2
```