use eyre::Result;

use crate::node::Node;

pub struct System {
    pub node: Node,
}
impl System {
    pub fn new() -> Result<Self> {
        Ok(Self {
            node: Node::new("测试节点".to_string())?,
        })
    }
}
