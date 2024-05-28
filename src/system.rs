use eyre::Result;
use node_network::Node;

pub struct System {
    pub node: Node,
}
impl System {
    pub fn new() -> Result<Self> {
        Ok(Self {
            node: Node::new_from_new_cert("0.0.0.0:0".parse()?)?,
        })
    }
}
impl Drop for System {
    fn drop(&mut self) {
        self.node.close(0, "节点实例被释放".as_bytes());
    }
}
